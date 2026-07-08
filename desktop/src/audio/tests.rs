use super::*;

#[test]
fn doubler_disabled_preserves_dual_mono() {
    let ui = GreyboundUi::default();
    let controls = SharedRuntimeControls::new(&ui);
    let mut doubler = DoublerProcessor::new(48_000.0);

    for index in 0..256 {
        let sample = (index as f32 * 0.01).sin() * 0.2;
        let (left, right) = doubler.process(sample, &controls);
        assert!((left - sample).abs() < 1.0e-7);
        assert!((right - sample).abs() < 1.0e-7);
    }
}

#[test]
fn doubler_enabled_decorrelates_left_and_right() {
    let mut ui = GreyboundUi::default();
    ui.doubler.enabled = true;
    ui.doubler.delay_ms = 7.15;
    let controls = SharedRuntimeControls::new(&ui);
    let mut doubler = DoublerProcessor::new(48_000.0);

    let (left, right) = doubler.process(0.5, &controls);

    assert!(left > right, "left={left}, right={right}");
    assert!((left - right).abs() > 0.1, "left={left}, right={right}");
}

#[test]
fn graphic_eq_disabled_preserves_input() {
    let mut ui = GreyboundUi::default();
    ui.eq.enabled = false;
    ui.eq.bands[4] = 1.0;
    let controls = SharedRuntimeControls::new(&ui);
    let mut eq = GraphicEqProcessor::new(48_000.0);

    for index in 0..256 {
        let sample = (index as f32 * 0.07).sin() * 0.25;
        let output = eq.process(sample, &controls);
        assert!((output - sample).abs() < 1.0e-7);
    }
}

#[test]
fn graphic_eq_band_boost_changes_signal_energy() {
    let mut flat_ui = GreyboundUi::default();
    flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    let flat_controls = SharedRuntimeControls::new(&flat_ui);
    let mut flat_eq = GraphicEqProcessor::new(48_000.0);

    let mut boost_ui = GreyboundUi::default();
    boost_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    boost_ui.eq.bands[4] = 1.0;
    let boost_controls = SharedRuntimeControls::new(&boost_ui);
    let mut boost_eq = GraphicEqProcessor::new(48_000.0);

    let mut flat_energy = 0.0;
    let mut boost_energy = 0.0;
    for index in 0..1024 {
        let phase = std::f32::consts::TAU * 1_000.0 * index as f32 / 48_000.0;
        let sample = phase.sin() * 0.1;
        let flat = flat_eq.process(sample, &flat_controls);
        let boosted = boost_eq.process(sample, &boost_controls);
        flat_energy += flat * flat;
        boost_energy += boosted * boosted;
    }

    assert!(boost_energy.is_finite());
    assert!(
        boost_energy > flat_energy * 1.4,
        "flat={flat_energy}, boost={boost_energy}"
    );
}

#[test]
fn graphic_eq_hpf_reduces_low_frequency_energy() {
    let mut flat_ui = GreyboundUi::default();
    flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    let flat_controls = SharedRuntimeControls::new(&flat_ui);
    let mut flat_eq = GraphicEqProcessor::new(48_000.0);

    let mut filtered_ui = GreyboundUi::default();
    filtered_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    filtered_ui.eq.hpf = 1.0;
    let filtered_controls = SharedRuntimeControls::new(&filtered_ui);
    let mut filtered_eq = GraphicEqProcessor::new(48_000.0);

    let mut flat_energy = 0.0;
    let mut filtered_energy = 0.0;
    for index in 0..2_048 {
        let phase = std::f32::consts::TAU * 60.0 * index as f32 / 48_000.0;
        let sample = phase.sin() * 0.1;
        flat_energy += flat_eq.process(sample, &flat_controls).powi(2);
        filtered_energy += filtered_eq.process(sample, &filtered_controls).powi(2);
    }

    assert!(
        filtered_energy < flat_energy * 0.45,
        "flat={flat_energy}, filtered={filtered_energy}"
    );
}

#[test]
fn graphic_eq_lpf_reduces_high_frequency_energy() {
    let mut flat_ui = GreyboundUi::default();
    flat_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    let flat_controls = SharedRuntimeControls::new(&flat_ui);
    let mut flat_eq = GraphicEqProcessor::new(48_000.0);

    let mut filtered_ui = GreyboundUi::default();
    filtered_ui.eq.bands = [0.5; greybound_ui::EQ_BAND_COUNT];
    filtered_ui.eq.lpf = 1.0;
    let filtered_controls = SharedRuntimeControls::new(&filtered_ui);
    let mut filtered_eq = GraphicEqProcessor::new(48_000.0);

    let mut flat_energy = 0.0;
    let mut filtered_energy = 0.0;
    for index in 0..2_048 {
        let phase = std::f32::consts::TAU * 8_000.0 * index as f32 / 48_000.0;
        let sample = phase.sin() * 0.1;
        flat_energy += flat_eq.process(sample, &flat_controls).powi(2);
        filtered_energy += filtered_eq.process(sample, &filtered_controls).powi(2);
    }

    assert!(
        filtered_energy < flat_energy * 0.25,
        "flat={flat_energy}, filtered={filtered_energy}"
    );
}

#[test]
fn default_fx_loop_controls_target_springfield() {
    let ui = GreyboundUi::default();
    let controls = SharedRuntimeControls::new(&ui);
    let mut slots = Vec::new();

    controls.load_device_controls_into(&mut slots);

    assert_eq!(slots.len(), 2);
    match slots[1].controls {
        DeviceControls::Springfield(controls) => {
            assert!(slots[1].bypassed);
            assert!((controls.dwell - 0.48).abs() < 1.0e-6);
            assert!((controls.tone - 0.58).abs() < 1.0e-6);
            assert!((controls.mix - 0.26).abs() < 1.0e-6);
        }
        other => panic!("expected bypassed Springfield controls, got {other:?}"),
    }
}

#[test]
fn fully_bypassed_runtime_preserves_live_input() {
    let mut ui = GreyboundUi::default();
    ui.amp.bypassed = true;
    ui.cab.bypassed = true;
    ui.cab.master = 0.0;
    ui.eq.enabled = false;
    ui.input_gain = 0.5;
    ui.output_gain = 0.8;
    for device in &mut ui.devices {
        device.bypassed = true;
    }

    let controls = SharedRuntimeControls::new(&ui);
    let meters = MeterStats::default();
    let (mut producer, consumer) = RingBuffer::<f32>::new(1024);
    let mut runtime =
        AudioRuntime::new(48_000.0, consumer, ui.amp_model_id(), ui.app_profile).unwrap();

    for sample_idx in 0..512 {
        let input = (std::f32::consts::TAU * 880.0 * sample_idx as f32 / 48_000.0).sin() * 0.2;
        producer.push(input).unwrap();

        let (left, right) = runtime.process(&controls, &meters);

        assert!((left - input).abs() < 1.0e-6, "left={left}, input={input}");
        assert!(
            (right - input).abs() < 1.0e-6,
            "right={right}, input={input}"
        );
    }
}
