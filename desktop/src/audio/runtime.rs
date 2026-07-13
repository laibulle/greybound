use anyhow::Result;
use greybound::ir::SpeakerStage;
use greybound::{
    DeviceSlotConfig, DeviceSlotControls, SignalChain, SignalChainConfig, SignalChainControls,
};
use greybound_ui::{AppProfile, RuntimeDeviceSection};
use rtrb::Consumer;

use super::controls::SharedRuntimeControls;
use super::doubler::DoublerProcessor;
use super::eq::GraphicEqProcessor;
use super::meter::MeterStats;
use super::metronome::MetronomeGenerator;
use super::util::protect_dac;

pub(super) struct AudioRuntime {
    input: Consumer<f32>,
    chain: SignalChain,
    speaker: SpeakerStage,
    device_controls: Vec<DeviceSlotControls>,
    eq: GraphicEqProcessor,
    metronome: MetronomeGenerator,
    doubler: DoublerProcessor,
}

impl AudioRuntime {
    pub(super) fn new(
        sample_rate: f32,
        input: Consumer<f32>,
        amp_model: &str,
        app_profile: AppProfile,
    ) -> Result<Self> {
        let mut config = SignalChainConfig::amp_only(amp_model);
        for slot in app_profile.runtime_devices {
            let device = if slot.bypassed {
                DeviceSlotConfig::bypassed(slot.config)
            } else {
                DeviceSlotConfig::active(slot.config)
            };
            match slot.section {
                RuntimeDeviceSection::PreAmp => config.pre_amp.push(device),
                RuntimeDeviceSection::PostAmp => config.post_amp.push(device),
            }
        }

        Ok(Self {
            input,
            chain: SignalChain::new(sample_rate, config),
            speaker: reference_speaker_or_bypass(sample_rate as u32),
            device_controls: Vec::with_capacity(3),
            eq: GraphicEqProcessor::new(sample_rate),
            metronome: MetronomeGenerator::new(sample_rate),
            doubler: DoublerProcessor::new(sample_rate),
        })
    }

    pub(super) fn process(
        &mut self,
        controls: &SharedRuntimeControls,
        meters: &MeterStats,
    ) -> (f32, f32) {
        let guitar = self.process_guitar_mono(controls, meters);
        if controls.tuner_muted() {
            return (0.0, 0.0);
        }
        let guitar = self.doubler.process(guitar, controls);
        let metronome = self.metronome.process(controls);
        mix_final_output(guitar, metronome)
    }

    fn process_guitar_mono(
        &mut self,
        controls: &SharedRuntimeControls,
        meters: &MeterStats,
    ) -> f32 {
        let input = match self.input.pop() {
            Ok(sample) => sample,
            Err(_) => {
                meters.record_input_underrun();
                0.0
            }
        } * controls.input_gain();
        meters.record_input(input);
        controls.load_device_controls_into(&mut self.device_controls);
        let chain_output = self.chain.process_with_amp_enabled(
            input,
            SignalChainControls {
                amp: controls.load_amp_controls(),
                devices: &self.device_controls,
            },
            controls.amp_enabled(),
        );
        let cab_mix = controls.cab_mix();
        let wet = self.speaker.process(chain_output, cab_mix > 0.0);
        let cabbed = chain_output * (1.0 - cab_mix) + wet * cab_mix;
        self.eq.process(cabbed, controls) * controls.output_gain()
    }
}

fn reference_speaker_or_bypass(sample_rate: u32) -> SpeakerStage {
    SpeakerStage::from_embedded_ir(sample_rate).unwrap_or_else(|error| {
        #[cfg(debug_assertions)]
        eprintln!("Greybound speaker IR disabled: {error:#}");
        #[cfg(not(debug_assertions))]
        let _ = error;
        SpeakerStage::bypassed()
    })
}

fn mix_final_output(guitar: (f32, f32), metronome: (f32, f32)) -> (f32, f32) {
    (
        protect_dac(guitar.0 + metronome.0),
        protect_dac(guitar.1 + metronome.1),
    )
}

pub(super) fn pre_amp_device_summary(app_profile: AppProfile) -> String {
    runtime_device_summary(app_profile, RuntimeDeviceSection::PreAmp)
}

pub(super) fn post_amp_device_summary(app_profile: AppProfile) -> String {
    runtime_device_summary(app_profile, RuntimeDeviceSection::PostAmp)
}

fn runtime_device_summary(app_profile: AppProfile, section: RuntimeDeviceSection) -> String {
    let labels: Vec<String> = app_profile
        .runtime_devices
        .iter()
        .filter(|slot| slot.section == section)
        .map(|slot| {
            let label = slot.config.model_descriptor().label;
            if slot.bypassed {
                format!("{label} bypassed")
            } else {
                label.to_string()
            }
        })
        .collect();

    if labels.is_empty() {
        "none".to_string()
    } else {
        labels.join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::controls::SharedRuntimeControls;
    use crate::audio::meter::MeterStats;
    use greybound_ui::GreyboundUi;
    use rtrb::RingBuffer;

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
}
