use super::NamModel;
use anyhow::{Context, Result};
use serde_json::Value;

const NUM_LAYERS: usize = 23;
const HEAD_KERNEL_SIZE: usize = 16;
const LEAKY_SLOPE: f32 = 0.01;
const KERNEL_SIZES: [usize; NUM_LAYERS] = [
    6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 15, 15, 6, 6, 6, 6, 6, 6, 6,
];
const DILATIONS: [usize; NUM_LAYERS] = [
    1, 3, 7, 17, 41, 101, 239, 1, 3, 7, 17, 41, 101, 239, 1, 13, 1, 3, 7, 17, 41, 101, 239,
];

#[derive(Debug)]
pub struct NamA2Processor {
    channels: usize,
    rechannel_w: Vec<f32>,
    layers: Vec<A2Layer>,
    head_w: Vec<f32>,
    head_b: f32,
    head_scale: f32,
    layer_in: Vec<f32>,
    activation: Vec<f32>,
    head_sum: Vec<f32>,
    head_history: RingHistory,
    prewarm_samples: usize,
}

#[derive(Debug)]
struct A2Layer {
    kernel_size: usize,
    dilation: usize,
    conv_w: Vec<f32>,
    conv_b: Vec<f32>,
    mixin_w: Vec<f32>,
    layer_1x1_w: Vec<f32>,
    layer_1x1_b: Vec<f32>,
    history: RingHistory,
}

#[derive(Debug)]
struct RingHistory {
    channels: usize,
    columns: usize,
    write_pos: usize,
    data: Vec<f32>,
}

impl NamA2Processor {
    pub fn from_model(model: &NamModel) -> Result<Self> {
        let channels =
            supported_channels(&model.config).context("NAM model is not A2 standard/nano")?;
        let mut weights = WeightReader::new(&model.weights);
        let rechannel_w = weights.take_vec(channels)?;
        let mut layers = Vec::with_capacity(NUM_LAYERS);

        let mut prewarm_samples = HEAD_KERNEL_SIZE - 1;
        for layer_idx in 0..NUM_LAYERS {
            let kernel_size = KERNEL_SIZES[layer_idx];
            let dilation = DILATIONS[layer_idx];
            let mut conv_w = vec![0.0; kernel_size * channels * channels];

            for out_channel in 0..channels {
                for in_channel in 0..channels {
                    for tap in 0..kernel_size {
                        conv_w[tap * channels * channels + in_channel * channels + out_channel] =
                            weights.take()?;
                    }
                }
            }

            let conv_b = weights.take_vec(channels)?;
            let mixin_w = weights.take_vec(channels)?;
            let mut layer_1x1_w = vec![0.0; channels * channels];

            for out_channel in 0..channels {
                for in_channel in 0..channels {
                    layer_1x1_w[in_channel * channels + out_channel] = weights.take()?;
                }
            }

            let layer_1x1_b = weights.take_vec(channels)?;
            let max_lookback = (kernel_size - 1) * dilation;
            prewarm_samples += max_lookback;

            layers.push(A2Layer {
                kernel_size,
                dilation,
                conv_w,
                conv_b,
                mixin_w,
                layer_1x1_w,
                layer_1x1_b,
                history: RingHistory::new(channels, max_lookback + 1),
            });
        }

        let mut head_w = vec![0.0; HEAD_KERNEL_SIZE * channels];
        for in_channel in 0..channels {
            for tap in 0..HEAD_KERNEL_SIZE {
                head_w[tap * channels + in_channel] = weights.take()?;
            }
        }
        let head_b = weights.take()?;
        let head_scale = weights.take()?;
        weights.finish()?;

        let mut processor = Self {
            channels,
            rechannel_w,
            layers,
            head_w,
            head_b,
            head_scale,
            layer_in: vec![0.0; channels],
            activation: vec![0.0; channels],
            head_sum: vec![0.0; channels],
            head_history: RingHistory::new(channels, HEAD_KERNEL_SIZE),
            prewarm_samples,
        };
        processor.prewarm();
        Ok(processor)
    }

    pub fn is_supported(model: &NamModel) -> bool {
        model.architecture == "WaveNet" && supported_channels(&model.config).is_some()
    }

    pub fn reset(&mut self) {
        self.layer_in.fill(0.0);
        self.activation.fill(0.0);
        self.head_sum.fill(0.0);
        self.head_history.clear();
        for layer in &mut self.layers {
            layer.history.clear();
        }
        self.prewarm();
    }

    pub fn process(&mut self, input: f32) -> f32 {
        for channel in 0..self.channels {
            self.layer_in[channel] = self.rechannel_w[channel] * input;
            self.head_sum[channel] = 0.0;
        }

        let layer_in = &mut self.layer_in;
        let activation = &mut self.activation;
        let head_sum = &mut self.head_sum;
        for layer in &mut self.layers {
            layer.history.push(layer_in);

            activation.fill(0.0);
            for out_channel in 0..self.channels {
                let mut value = layer.conv_b[out_channel] + layer.mixin_w[out_channel] * input;

                for tap in 0..layer.kernel_size {
                    let taps_back = layer.kernel_size - 1 - tap;
                    let history_col = layer.history.get(taps_back * layer.dilation);
                    for in_channel in 0..self.channels {
                        let weight_index = tap * self.channels * self.channels
                            + in_channel * self.channels
                            + out_channel;
                        value += layer.conv_w[weight_index] * history_col[in_channel];
                    }
                }

                activation[out_channel] = leaky_relu(value);
            }

            for channel in 0..self.channels {
                head_sum[channel] += activation[channel];
            }

            for out_channel in 0..self.channels {
                let mut residual = layer.layer_1x1_b[out_channel];
                for in_channel in 0..self.channels {
                    residual += layer.layer_1x1_w[in_channel * self.channels + out_channel]
                        * activation[in_channel];
                }
                layer_in[out_channel] += residual;
            }
        }

        self.head_history.push(&self.head_sum);
        let mut output = self.head_b;
        for tap in 0..HEAD_KERNEL_SIZE {
            let taps_back = HEAD_KERNEL_SIZE - 1 - tap;
            let history_col = self.head_history.get(taps_back);
            for channel in 0..self.channels {
                output += self.head_w[tap * self.channels + channel] * history_col[channel];
            }
        }
        output * self.head_scale
    }

    fn prewarm(&mut self) {
        for _ in 0..self.prewarm_samples {
            self.process(0.0);
        }
    }
}

impl RingHistory {
    fn new(channels: usize, columns: usize) -> Self {
        Self {
            channels,
            columns,
            write_pos: 0,
            data: vec![0.0; channels * columns],
        }
    }

    fn clear(&mut self) {
        self.write_pos = 0;
        self.data.fill(0.0);
    }

    fn push(&mut self, frame: &[f32]) {
        debug_assert_eq!(frame.len(), self.channels);
        let offset = self.write_pos * self.channels;
        self.data[offset..offset + self.channels].copy_from_slice(frame);
        self.write_pos = (self.write_pos + 1) % self.columns;
    }

    fn get(&self, delay: usize) -> &[f32] {
        debug_assert!(delay < self.columns);
        let col = (self.write_pos + self.columns - 1 - delay) % self.columns;
        let offset = col * self.channels;
        &self.data[offset..offset + self.channels]
    }
}

struct WeightReader<'a> {
    weights: &'a [f32],
    cursor: usize,
}

impl<'a> WeightReader<'a> {
    fn new(weights: &'a [f32]) -> Self {
        Self { weights, cursor: 0 }
    }

    fn take(&mut self) -> Result<f32> {
        let value = self
            .weights
            .get(self.cursor)
            .copied()
            .context("NAM A2 weight stream exhausted")?;
        self.cursor += 1;
        Ok(value)
    }

    fn take_vec(&mut self, len: usize) -> Result<Vec<f32>> {
        let end = self.cursor + len;
        let values = self
            .weights
            .get(self.cursor..end)
            .context("NAM A2 weight stream exhausted")?
            .to_vec();
        self.cursor = end;
        Ok(values)
    }

    fn finish(self) -> Result<()> {
        if self.cursor == self.weights.len() {
            Ok(())
        } else {
            anyhow::bail!(
                "NAM A2 weight stream has {} trailing values",
                self.weights.len() - self.cursor
            )
        }
    }
}

fn supported_channels(config: &Value) -> Option<usize> {
    let layers = config.get("layers")?.as_array()?;
    if layers.len() != 1 {
        return None;
    }
    if !config.get("head").is_none_or(Value::is_null) {
        return None;
    }
    if !config.get("condition_dsp").is_none_or(Value::is_null) {
        return None;
    }
    if !config.get("head_scale")?.is_number() {
        return None;
    }
    if int_field(config, "in_channels").unwrap_or(1) != 1 {
        return None;
    }

    let layer = &layers[0];
    if int_field(layer, "input_size")? != 1 || int_field(layer, "condition_size")? != 1 {
        return None;
    }

    let channels = int_field(layer, "channels")?;
    if channels != int_field(layer, "bottleneck")? || (channels != 3 && channels != 8) {
        return None;
    }
    if !int_array_equals(layer.get("kernel_sizes")?, &KERNEL_SIZES) {
        return None;
    }
    if !int_array_equals(layer.get("dilations")?, &DILATIONS) {
        return None;
    }
    if !a2_activations(layer.get("activation")?) {
        return None;
    }
    if !none_strings_or_absent(layer.get("gating_mode"), NUM_LAYERS) {
        return None;
    }
    if bool_field(layer, "gated").unwrap_or(false) {
        return None;
    }
    if !null_array_or_absent(layer.get("secondary_activation"), NUM_LAYERS) {
        return None;
    }
    if object_bool(layer.get("head1x1"), "active").unwrap_or(false) {
        return None;
    }

    let layer_1x1 = layer.get("layer1x1")?;
    if !object_bool(Some(layer_1x1), "active").unwrap_or(false) {
        return None;
    }
    if object_int(Some(layer_1x1), "groups").unwrap_or(1) != 1 {
        return None;
    }

    let head = layer.get("head")?;
    if object_int(Some(head), "out_channels")? != 1
        || object_int(Some(head), "kernel_size")? != HEAD_KERNEL_SIZE
        || object_int(Some(head), "head_dilation").unwrap_or(1) != 1
        || !object_bool(Some(head), "bias").unwrap_or(false)
    {
        return None;
    }

    for key in [
        "conv_pre_film",
        "conv_post_film",
        "input_mixin_pre_film",
        "input_mixin_post_film",
        "activation_pre_film",
        "activation_post_film",
        "layer1x1_post_film",
        "head1x1_post_film",
    ] {
        if !film_inactive(layer.get(key)) {
            return None;
        }
    }
    if int_field(layer, "groups_input").unwrap_or(1) != 1
        || int_field(layer, "groups_input_mixin").unwrap_or(1) != 1
        || !layer.get("slimmable").is_none_or(Value::is_null)
    {
        return None;
    }

    Some(channels)
}

fn int_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn bool_field(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn object_int(value: Option<&Value>, key: &str) -> Option<usize> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn object_bool(value: Option<&Value>, key: &str) -> Option<bool> {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_bool)
}

fn int_array_equals(value: &Value, expected: &[usize]) -> bool {
    let Some(values) = value.as_array() else {
        return false;
    };
    values.len() == expected.len()
        && values.iter().zip(expected).all(|(value, expected)| {
            value
                .as_u64()
                .is_some_and(|value| usize::try_from(value).ok() == Some(*expected))
        })
}

fn a2_activations(value: &Value) -> bool {
    let Some(values) = value.as_array() else {
        return false;
    };
    values.len() == NUM_LAYERS
        && values.iter().all(|value| {
            value.get("type").and_then(Value::as_str) == Some("LeakyReLU")
                && value
                    .get("negative_slope")
                    .and_then(Value::as_f64)
                    .is_some_and(|slope| (slope as f32 - LEAKY_SLOPE).abs() <= 1e-7)
        })
}

fn none_strings_or_absent(value: Option<&Value>, expected_len: usize) -> bool {
    let Some(value) = value else {
        return true;
    };
    if value.is_null() {
        return true;
    }
    let Some(values) = value.as_array() else {
        return false;
    };
    values.len() == expected_len && values.iter().all(|value| value.as_str() == Some("none"))
}

fn null_array_or_absent(value: Option<&Value>, expected_len: usize) -> bool {
    let Some(value) = value else {
        return true;
    };
    if value.is_null() {
        return true;
    }
    let Some(values) = value.as_array() else {
        return false;
    };
    values.len() == expected_len && values.iter().all(Value::is_null)
}

fn film_inactive(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Bool(active)) => !active,
        Some(Value::Object(object)) => !object
            .get("active")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    }
}

fn leaky_relu(value: f32) -> f32 {
    if value >= 0.0 {
        value
    } else {
        value * LEAKY_SLOPE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_standard_a2_shape() {
        let model = test_model(8, vec![0.0; a2_weight_count(8)]);

        assert!(NamA2Processor::is_supported(&model));
    }

    #[test]
    fn rejects_non_a2_wavenet_shape() {
        let model = NamModel {
            version: "0.7.0".to_string(),
            architecture: "WaveNet".to_string(),
            config: json!({"layers": [], "head": null, "head_scale": 0.01}),
            weights: vec![0.0],
            sample_rate_hz: 48_000.0,
            metadata: Default::default(),
        };

        assert!(!NamA2Processor::is_supported(&model));
    }

    #[test]
    fn zero_weights_produce_silence() {
        let model = test_model(8, vec![0.0; a2_weight_count(8)]);
        let mut processor = NamA2Processor::from_model(&model).unwrap();

        for _ in 0..128 {
            assert_eq!(processor.process(0.5), 0.0);
        }
    }

    #[test]
    fn rejects_trailing_weights() {
        let model = test_model(8, vec![0.0; a2_weight_count(8) + 1]);
        let error = NamA2Processor::from_model(&model).unwrap_err();

        assert!(error.to_string().contains("trailing"));
    }

    #[test]
    fn official_a2_fixture_loads_when_available() {
        let path = std::path::Path::new("/tmp/NeuralAmpModelerCore/example_models/A2.nam");
        if !path.exists() {
            return;
        }

        let model = NamModel::from_path(path).unwrap();
        assert!(NamA2Processor::is_supported(&model));

        let mut processor = NamA2Processor::from_model(&model).unwrap();
        let mut outputs = Vec::with_capacity(2048);
        for frame in 0..2048 {
            let input = ((frame as f32) * 0.017).sin() * 0.1;
            let output = processor.process(input);
            assert!(output.is_finite());
            outputs.push(output);
        }

        let expected = [
            (0, 0.0002503977),
            (1, 0.0006381204),
            (2, -0.0009207843),
            (10, -0.0945109501),
            (100, -0.0324719846),
            (511, 0.0578287169),
            (1024, 0.1297123283),
            (2047, 0.2164351344),
        ];
        for (index, expected) in expected {
            assert!(
                (outputs[index] - expected).abs() < 1e-5,
                "sample {index}: got {}, expected {expected}",
                outputs[index]
            );
        }
    }

    fn test_model(channels: usize, weights: Vec<f32>) -> NamModel {
        NamModel {
            version: "0.7.0".to_string(),
            architecture: "WaveNet".to_string(),
            config: test_config(channels),
            weights,
            sample_rate_hz: 48_000.0,
            metadata: Default::default(),
        }
    }

    fn a2_weight_count(channels: usize) -> usize {
        channels
            + KERNEL_SIZES
                .iter()
                .map(|kernel_size| {
                    kernel_size * channels * channels
                        + channels
                        + channels
                        + channels * channels
                        + channels
                })
                .sum::<usize>()
            + HEAD_KERNEL_SIZE * channels
            + 1
            + 1
    }

    fn test_config(channels: usize) -> Value {
        json!({
            "in_channels": 1,
            "layers": [
                {
                    "input_size": 1,
                    "condition_size": 1,
                    "channels": channels,
                    "bottleneck": channels,
                    "kernel_sizes": KERNEL_SIZES,
                    "dilations": DILATIONS,
                    "activation": vec![json!({"type": "LeakyReLU", "negative_slope": 0.01}); NUM_LAYERS],
                    "gating_mode": vec!["none"; NUM_LAYERS],
                    "secondary_activation": vec![Value::Null; NUM_LAYERS],
                    "head": {"out_channels": 1, "kernel_size": HEAD_KERNEL_SIZE, "bias": true},
                    "layer1x1": {"active": true, "groups": 1},
                    "head1x1": {"active": false, "out_channels": 1, "groups": 1},
                    "groups_input": 1,
                    "groups_input_mixin": 1
                }
            ],
            "head": null,
            "head_scale": 0.01
        })
    }
}
