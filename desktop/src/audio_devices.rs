use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{
    BufferSize, Device, SampleFormat, SampleRate, StreamConfig, SupportedStreamConfigRange,
};
use greybound_ui::{GreyboundUi, Message};

pub(crate) fn select_config(
    configs: impl Iterator<Item = SupportedStreamConfigRange>,
    sample_rate: u32,
    period_size: u32,
    direction: &str,
) -> Result<SupportedStreamConfigRange> {
    let rate = SampleRate(sample_rate);
    configs
        .filter(|config| config.sample_format() == SampleFormat::F32)
        .find(|config| {
            (config.min_sample_rate()..=config.max_sample_rate()).contains(&rate)
                && match config.buffer_size() {
                    cpal::SupportedBufferSize::Range { min, max } => {
                        (*min..=*max).contains(&period_size)
                    }
                    cpal::SupportedBufferSize::Unknown => true,
                }
        })
        .with_context(|| {
            format!(
                "no f32 {direction} configuration supports {sample_rate} Hz / {period_size} samples"
            )
        })
}

pub(crate) fn stream_config(
    range: &SupportedStreamConfigRange,
    sample_rate: u32,
    period_size: u32,
) -> StreamConfig {
    StreamConfig {
        channels: range.channels(),
        sample_rate: SampleRate(sample_rate),
        buffer_size: BufferSize::Fixed(period_size),
    }
}

pub(crate) fn device_name(device: &Device) -> String {
    device
        .name()
        .unwrap_or_else(|_| "unknown device".to_string())
}

pub(crate) fn refresh_audio_devices(ui: &mut GreyboundUi) {
    let host = cpal::default_host();
    let inputs = device_names(host.input_devices());
    let outputs = device_names(host.output_devices());
    let sample_rate = ui.audio_settings.sample_rate;
    let period_size = ui.audio_settings.period_size;
    let selected_input = ui
        .audio_settings
        .selected_input
        .clone()
        .filter(|selected| inputs.contains(selected))
        .or_else(|| preferred_input_name(&host, sample_rate, period_size));
    let selected_output = ui
        .audio_settings
        .selected_output
        .clone()
        .filter(|selected| outputs.contains(selected))
        .or_else(|| preferred_output_name(&host, sample_rate, period_size));

    ui.update(Message::AudioDevicesChanged {
        inputs,
        outputs,
        selected_input,
        selected_output,
        status: "Audio devices loaded".to_string(),
    });
}

fn device_names<I>(devices: std::result::Result<I, cpal::DevicesError>) -> Vec<String>
where
    I: Iterator<Item = Device>,
{
    let mut names = devices
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_else(|_| Vec::new());
    names.sort();
    names.dedup();
    names
}

fn preferred_input_name(host: &cpal::Host, sample_rate: u32, period_size: u32) -> Option<String> {
    let default_name = host
        .default_input_device()
        .map(|device| device_name(&device));
    preferred_device_name(
        host.input_devices().ok()?,
        default_name.as_deref(),
        sample_rate,
        period_size,
        AudioDirection::Input,
    )
    .or(default_name)
}

fn preferred_output_name(host: &cpal::Host, sample_rate: u32, period_size: u32) -> Option<String> {
    let default_name = host
        .default_output_device()
        .map(|device| device_name(&device));
    preferred_device_name(
        host.output_devices().ok()?,
        default_name.as_deref(),
        sample_rate,
        period_size,
        AudioDirection::Output,
    )
    .or(default_name)
}

fn preferred_device_name<I>(
    devices: I,
    default_name: Option<&str>,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> Option<String>
where
    I: Iterator<Item = Device>,
{
    devices
        .filter_map(|device| {
            let name = device.name().ok()?;
            let score = low_latency_device_score(
                &device,
                &name,
                default_name,
                sample_rate,
                period_size,
                direction,
            );
            (score > 0).then_some((score, name))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, name)| name)
}

fn low_latency_device_score(
    device: &Device,
    name: &str,
    default_name: Option<&str>,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> i32 {
    let Some(support) = low_latency_support(device, sample_rate, period_size, direction) else {
        return 0;
    };

    let mut score = match support {
        LowLatencySupport::Explicit => 120,
        LowLatencySupport::Unknown => 70,
    };

    if is_pro_audio_device_name(name) {
        score += 90;
    }
    if is_integrated_audio_name(name) {
        score -= 80;
    }
    if default_name.is_some_and(|default| default == name) {
        score += 5;
    }

    score
}

#[derive(Clone, Copy)]
enum LowLatencySupport {
    Explicit,
    Unknown,
}

fn low_latency_support(
    device: &Device,
    sample_rate: u32,
    period_size: u32,
    direction: AudioDirection,
) -> Option<LowLatencySupport> {
    match direction {
        AudioDirection::Input => {
            configs_low_latency_support(device.supported_input_configs(), sample_rate, period_size)
        }
        AudioDirection::Output => {
            configs_low_latency_support(device.supported_output_configs(), sample_rate, period_size)
        }
    }
}

fn configs_low_latency_support<I>(
    configs: std::result::Result<I, cpal::SupportedStreamConfigsError>,
    sample_rate: u32,
    period_size: u32,
) -> Option<LowLatencySupport>
where
    I: Iterator<Item = SupportedStreamConfigRange>,
{
    let rate = SampleRate(sample_rate);
    let mut has_unknown = false;

    for config in configs.ok()? {
        if config.sample_format() != SampleFormat::F32
            || !(config.min_sample_rate()..=config.max_sample_rate()).contains(&rate)
        {
            continue;
        }

        match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => {
                if (*min..=*max).contains(&period_size) {
                    return Some(LowLatencySupport::Explicit);
                }
            }
            cpal::SupportedBufferSize::Unknown => {
                has_unknown = true;
            }
        }
    }

    has_unknown.then_some(LowLatencySupport::Unknown)
}

fn is_pro_audio_device_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "scarlett",
        "focusrite",
        "clarett",
        "apollo",
        "universal audio",
        "rme",
        "babyface",
        "fireface",
        "motu",
        "audient",
        "ssl",
        "antelope",
        "presonus",
        "quantum",
        "steinberg",
        "ur22",
        "ur44",
        "komplete",
        "axe-fx",
        "helix",
        "quad cortex",
        "volt",
        "behringer",
        "umc",
        "zoom",
        "tascam",
        "m-audio",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

fn is_integrated_audio_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "macbook",
        "built-in",
        "built in",
        "internal",
        "airpods",
        "display audio",
        "microphone",
        "speakers",
    ]
    .iter()
    .any(|keyword| name.contains(keyword))
}

#[derive(Clone, Copy)]
enum AudioDirection {
    Input,
    Output,
}

pub(crate) fn selected_or_default_input(
    host: &cpal::Host,
    selected: Option<&str>,
) -> Result<Device> {
    selected_device(host.input_devices()?, selected)
        .or_else(|| host.default_input_device())
        .context("missing default input device")
}

pub(crate) fn selected_or_default_output(
    host: &cpal::Host,
    selected: Option<&str>,
) -> Result<Device> {
    selected_device(host.output_devices()?, selected)
        .or_else(|| host.default_output_device())
        .context("missing default output device")
}

fn selected_device<I>(devices: I, selected: Option<&str>) -> Option<Device>
where
    I: Iterator<Item = Device>,
{
    let selected = selected?;
    devices
        .filter_map(|device| device.name().ok().map(|name| (device, name)))
        .find(|(_, name)| name == selected)
        .map(|(device, _)| device)
}
