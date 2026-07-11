use greybound::{
    AmpControls, AuralithControls, DeviceConfig, DeviceControls, DeviceSlotControls, LumenControls,
    MinotaurControls, SpringfieldControls, StudioDelayControls, StudioVerbAlgorithm,
    StudioVerbControls,
};
use greybound_ui::{GreyboundUi, RuntimeAudioSnapshot};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

use super::util::{atomic_f32, load_f32, store_f32};

#[derive(Clone)]
pub(super) struct SharedRuntimeControls {
    input_gain: Arc<AtomicU32>,
    output_gain: Arc<AtomicU32>,
    amp_volume: Arc<AtomicU32>,
    amp_bass: Arc<AtomicU32>,
    amp_treble: Arc<AtomicU32>,
    amp_cut: Arc<AtomicU32>,
    amp_output: Arc<AtomicU32>,
    amp_drive: Arc<AtomicU32>,
    amp_presence: Arc<AtomicU32>,
    amp_sag: Arc<AtomicU32>,
    amp_bypassed: Arc<AtomicBool>,
    lumen_bypassed: Arc<AtomicBool>,
    lumen_peak_reduction: Arc<AtomicU32>,
    lumen_gain: Arc<AtomicU32>,
    lumen_emphasis: Arc<AtomicU32>,
    lumen_mix: Arc<AtomicU32>,
    minotaur_bypassed: Arc<AtomicBool>,
    minotaur_gain: Arc<AtomicU32>,
    minotaur_treble: Arc<AtomicU32>,
    minotaur_output: Arc<AtomicU32>,
    studiodelay_bypassed: Arc<AtomicBool>,
    studiodelay_time_ms: Arc<AtomicU32>,
    studiodelay_feedback: Arc<AtomicU32>,
    studiodelay_tone: Arc<AtomicU32>,
    studiodelay_mod_depth: Arc<AtomicU32>,
    studiodelay_mix: Arc<AtomicU32>,
    springfield_bypassed: Arc<AtomicBool>,
    springfield_dwell: Arc<AtomicU32>,
    springfield_tone: Arc<AtomicU32>,
    springfield_mix: Arc<AtomicU32>,
    auralith_bypassed: Arc<AtomicBool>,
    auralith_decay: Arc<AtomicU32>,
    auralith_size: Arc<AtomicU32>,
    auralith_texture: Arc<AtomicU32>,
    auralith_tone: Arc<AtomicU32>,
    auralith_low_cut: Arc<AtomicU32>,
    auralith_mix: Arc<AtomicU32>,
    studioverb_bypassed: Arc<AtomicBool>,
    studioverb_decay: Arc<AtomicU32>,
    studioverb_size: Arc<AtomicU32>,
    studioverb_diffusion: Arc<AtomicU32>,
    studioverb_tone: Arc<AtomicU32>,
    studioverb_low_cut: Arc<AtomicU32>,
    studioverb_mix: Arc<AtomicU32>,
    runtime_devices: &'static [greybound_ui::RuntimeDeviceSlot],
    cab_enabled: Arc<AtomicBool>,
    cab_mix: Arc<AtomicU32>,
    metronome_enabled: Arc<AtomicBool>,
    metronome_bpm: Arc<AtomicU32>,
    metronome_volume: Arc<AtomicU32>,
    metronome_pan: Arc<AtomicU32>,
    metronome_beats_per_bar: Arc<AtomicU32>,
    metronome_rhythm_division: Arc<AtomicU32>,
    eq_enabled: Arc<AtomicBool>,
    eq_hpf_hz: Arc<AtomicU32>,
    eq_lpf_hz: Arc<AtomicU32>,
    eq_band_gains_db: [Arc<AtomicU32>; greybound_ui::EQ_BAND_COUNT],
    doubler_enabled: Arc<AtomicBool>,
    doubler_delay_ms: Arc<AtomicU32>,
    tuner_live: Arc<AtomicBool>,
    tuner_muted: Arc<AtomicBool>,
    tuner_reference_hz: Arc<AtomicU32>,
}

impl SharedRuntimeControls {
    pub(super) fn new(ui: &GreyboundUi) -> Self {
        let controls = Self {
            input_gain: atomic_f32(1.0),
            output_gain: atomic_f32(1.0),
            amp_volume: atomic_f32(0.0),
            amp_bass: atomic_f32(0.0),
            amp_treble: atomic_f32(0.0),
            amp_cut: atomic_f32(0.0),
            amp_output: atomic_f32(0.58),
            amp_drive: atomic_f32(0.0),
            amp_presence: atomic_f32(0.0),
            amp_sag: atomic_f32(0.0),
            amp_bypassed: Arc::new(AtomicBool::new(false)),
            lumen_bypassed: Arc::new(AtomicBool::new(true)),
            lumen_peak_reduction: atomic_f32(LumenControls::default().peak_reduction),
            lumen_gain: atomic_f32(LumenControls::default().gain),
            lumen_emphasis: atomic_f32(LumenControls::default().emphasis),
            lumen_mix: atomic_f32(LumenControls::default().mix),
            minotaur_bypassed: Arc::new(AtomicBool::new(false)),
            minotaur_gain: atomic_f32(0.0),
            minotaur_treble: atomic_f32(0.0),
            minotaur_output: atomic_f32(0.0),
            studiodelay_bypassed: Arc::new(AtomicBool::new(true)),
            studiodelay_time_ms: atomic_f32(360.0),
            studiodelay_feedback: atomic_f32(0.34),
            studiodelay_tone: atomic_f32(0.58),
            studiodelay_mod_depth: atomic_f32(0.08),
            studiodelay_mix: atomic_f32(0.18),
            springfield_bypassed: Arc::new(AtomicBool::new(true)),
            springfield_dwell: atomic_f32(0.48),
            springfield_tone: atomic_f32(0.58),
            springfield_mix: atomic_f32(0.26),
            auralith_bypassed: Arc::new(AtomicBool::new(false)),
            auralith_decay: atomic_f32(0.52),
            auralith_size: atomic_f32(0.55),
            auralith_texture: atomic_f32(0.68),
            auralith_tone: atomic_f32(0.55),
            auralith_low_cut: atomic_f32(0.32),
            auralith_mix: atomic_f32(0.24),
            studioverb_bypassed: Arc::new(AtomicBool::new(false)),
            studioverb_decay: atomic_f32(0.42),
            studioverb_size: atomic_f32(0.46),
            studioverb_diffusion: atomic_f32(0.64),
            studioverb_tone: atomic_f32(0.54),
            studioverb_low_cut: atomic_f32(0.36),
            studioverb_mix: atomic_f32(0.24),
            runtime_devices: ui.app_profile.runtime_devices,
            cab_enabled: Arc::new(AtomicBool::new(true)),
            cab_mix: atomic_f32(1.0),
            metronome_enabled: Arc::new(AtomicBool::new(false)),
            metronome_bpm: atomic_f32(120.0),
            metronome_volume: atomic_f32(0.70),
            metronome_pan: atomic_f32(0.50),
            metronome_beats_per_bar: Arc::new(AtomicU32::new(4)),
            metronome_rhythm_division: Arc::new(AtomicU32::new(1)),
            eq_enabled: Arc::new(AtomicBool::new(true)),
            eq_hpf_hz: atomic_f32(0.0),
            eq_lpf_hz: atomic_f32(0.0),
            eq_band_gains_db: std::array::from_fn(|_| atomic_f32(0.0)),
            doubler_enabled: Arc::new(AtomicBool::new(false)),
            doubler_delay_ms: atomic_f32(7.15),
            tuner_live: Arc::new(AtomicBool::new(false)),
            tuner_muted: Arc::new(AtomicBool::new(false)),
            tuner_reference_hz: atomic_f32(440.0),
        };
        controls.store_from_ui(ui);
        controls
    }

    pub(super) fn store_from_ui(&self, ui: &GreyboundUi) {
        self.store_snapshot(&ui.runtime_audio_snapshot());
    }

    fn store_snapshot(&self, snapshot: &RuntimeAudioSnapshot) {
        store_f32(&self.input_gain, snapshot.input_gain);
        store_f32(&self.output_gain, snapshot.output_gain);
        store_f32(&self.amp_volume, snapshot.amp.volume);
        store_f32(&self.amp_bass, snapshot.amp.bass);
        store_f32(&self.amp_treble, snapshot.amp.treble);
        store_f32(&self.amp_cut, snapshot.amp.cut);
        store_f32(&self.amp_output, snapshot.amp.output);
        store_f32(&self.amp_drive, snapshot.amp.drive);
        store_f32(&self.amp_presence, snapshot.amp.presence);
        store_f32(&self.amp_sag, snapshot.amp.sag);
        self.amp_bypassed
            .store(!snapshot.amp_enabled, Ordering::Relaxed);
        self.cab_enabled
            .store(snapshot.cab_mix > 0.0, Ordering::Relaxed);
        store_f32(&self.cab_mix, snapshot.cab_mix.clamp(0.0, 1.0));
        self.metronome_enabled
            .store(snapshot.metronome_enabled, Ordering::Relaxed);
        store_f32(&self.metronome_bpm, snapshot.metronome_bpm);
        store_f32(&self.metronome_volume, snapshot.metronome_volume);
        store_f32(&self.metronome_pan, snapshot.metronome_pan);
        self.metronome_beats_per_bar
            .store(snapshot.metronome_beats_per_bar, Ordering::Relaxed);
        self.metronome_rhythm_division
            .store(snapshot.metronome_rhythm_division, Ordering::Relaxed);
        self.eq_enabled
            .store(snapshot.eq_enabled, Ordering::Relaxed);
        store_f32(&self.eq_hpf_hz, snapshot.eq_hpf_hz.unwrap_or(0.0));
        store_f32(&self.eq_lpf_hz, snapshot.eq_lpf_hz.unwrap_or(0.0));
        for (target, value) in self
            .eq_band_gains_db
            .iter()
            .zip(snapshot.eq_band_gains_db.iter())
        {
            store_f32(target, *value);
        }
        self.doubler_enabled
            .store(snapshot.doubler_enabled, Ordering::Relaxed);
        store_f32(&self.doubler_delay_ms, snapshot.doubler_delay_ms);
        self.tuner_live
            .store(snapshot.tuner_live, Ordering::Relaxed);
        self.tuner_muted
            .store(snapshot.tuner_muted, Ordering::Relaxed);
        store_f32(&self.tuner_reference_hz, snapshot.tuner_reference_hz);

        for slot in &snapshot.devices {
            match slot.controls {
                DeviceControls::Lumen(controls) => {
                    self.lumen_bypassed.store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.lumen_peak_reduction, controls.peak_reduction);
                    store_f32(&self.lumen_gain, controls.gain);
                    store_f32(&self.lumen_emphasis, controls.emphasis);
                    store_f32(&self.lumen_mix, controls.mix);
                }
                DeviceControls::Minotaur(controls) => {
                    self.minotaur_bypassed
                        .store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.minotaur_gain, controls.gain);
                    store_f32(&self.minotaur_treble, controls.treble);
                    store_f32(&self.minotaur_output, controls.output);
                }
                DeviceControls::StudioDelay(controls) => {
                    self.studiodelay_bypassed
                        .store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.studiodelay_time_ms, controls.time_ms);
                    store_f32(&self.studiodelay_feedback, controls.feedback);
                    store_f32(&self.studiodelay_tone, controls.tone);
                    store_f32(&self.studiodelay_mod_depth, controls.mod_depth);
                    store_f32(&self.studiodelay_mix, controls.mix);
                }
                DeviceControls::Springfield(controls) => {
                    self.springfield_bypassed
                        .store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.springfield_dwell, controls.dwell);
                    store_f32(&self.springfield_tone, controls.tone);
                    store_f32(&self.springfield_mix, controls.mix);
                }
                DeviceControls::Auralith(controls) => {
                    self.auralith_bypassed
                        .store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.auralith_decay, controls.decay);
                    store_f32(&self.auralith_size, controls.size);
                    store_f32(&self.auralith_texture, controls.texture);
                    store_f32(&self.auralith_tone, controls.tone);
                    store_f32(&self.auralith_low_cut, controls.low_cut);
                    store_f32(&self.auralith_mix, controls.mix);
                }
                DeviceControls::StudioVerb(controls) => {
                    self.studioverb_bypassed
                        .store(slot.bypassed, Ordering::Relaxed);
                    store_f32(&self.studioverb_decay, controls.decay);
                    store_f32(&self.studioverb_size, controls.size);
                    store_f32(&self.studioverb_diffusion, controls.diffusion);
                    store_f32(&self.studioverb_tone, controls.tone);
                    store_f32(&self.studioverb_low_cut, controls.low_cut);
                    store_f32(&self.studioverb_mix, controls.mix);
                }
                _ => {}
            }
        }
    }

    pub(super) fn load_amp_controls(&self) -> AmpControls {
        AmpControls {
            volume: load_f32(&self.amp_volume),
            bass: load_f32(&self.amp_bass),
            treble: load_f32(&self.amp_treble),
            cut: load_f32(&self.amp_cut),
            output: load_f32(&self.amp_output),
            drive: load_f32(&self.amp_drive),
            presence: load_f32(&self.amp_presence),
            sag: load_f32(&self.amp_sag),
        }
    }

    pub(super) fn input_gain(&self) -> f32 {
        load_f32(&self.input_gain)
    }

    pub(super) fn output_gain(&self) -> f32 {
        load_f32(&self.output_gain)
    }

    pub(super) fn amp_enabled(&self) -> bool {
        !self.amp_bypassed.load(Ordering::Relaxed)
    }

    pub(super) fn cab_mix(&self) -> f32 {
        if self.cab_enabled.load(Ordering::Relaxed) {
            load_f32(&self.cab_mix).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    pub(super) fn load_device_controls_into(&self, target: &mut Vec<DeviceSlotControls>) {
        target.clear();
        for slot in self.runtime_devices {
            match slot.config {
                DeviceConfig::Lumen => target.push(DeviceSlotControls {
                    bypassed: self.lumen_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Lumen(LumenControls {
                        peak_reduction: load_f32(&self.lumen_peak_reduction),
                        gain: load_f32(&self.lumen_gain),
                        emphasis: load_f32(&self.lumen_emphasis),
                        mix: load_f32(&self.lumen_mix),
                    }),
                }),
                DeviceConfig::Minotaur => target.push(DeviceSlotControls {
                    bypassed: self.minotaur_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Minotaur(MinotaurControls {
                        gain: load_f32(&self.minotaur_gain),
                        treble: load_f32(&self.minotaur_treble),
                        output: load_f32(&self.minotaur_output),
                    }),
                }),
                DeviceConfig::StudioDelay => target.push(DeviceSlotControls {
                    bypassed: self.studiodelay_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::StudioDelay(StudioDelayControls {
                        time_ms: load_f32(&self.studiodelay_time_ms),
                        feedback: load_f32(&self.studiodelay_feedback),
                        tone: load_f32(&self.studiodelay_tone),
                        mod_depth: load_f32(&self.studiodelay_mod_depth),
                        mix: load_f32(&self.studiodelay_mix),
                    }),
                }),
                DeviceConfig::Springfield => target.push(DeviceSlotControls {
                    bypassed: self.springfield_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Springfield(SpringfieldControls {
                        dwell: load_f32(&self.springfield_dwell),
                        tone: load_f32(&self.springfield_tone),
                        mix: load_f32(&self.springfield_mix),
                    }),
                }),
                DeviceConfig::Auralith => target.push(DeviceSlotControls {
                    bypassed: self.auralith_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::Auralith(AuralithControls {
                        decay: load_f32(&self.auralith_decay),
                        size: load_f32(&self.auralith_size),
                        texture: load_f32(&self.auralith_texture),
                        tone: load_f32(&self.auralith_tone),
                        low_cut: load_f32(&self.auralith_low_cut),
                        mix: load_f32(&self.auralith_mix),
                    }),
                }),
                DeviceConfig::StudioVerb => target.push(DeviceSlotControls {
                    bypassed: self.studioverb_bypassed.load(Ordering::Relaxed),
                    controls: DeviceControls::StudioVerb(StudioVerbControls {
                        algorithm: StudioVerbAlgorithm::Room,
                        decay: load_f32(&self.studioverb_decay),
                        size: load_f32(&self.studioverb_size),
                        pre_delay_ms: 12.0,
                        diffusion: load_f32(&self.studioverb_diffusion),
                        tone: load_f32(&self.studioverb_tone),
                        low_cut: load_f32(&self.studioverb_low_cut),
                        mod_depth: 0.18,
                        mix: load_f32(&self.studioverb_mix),
                    }),
                }),
                _ => {}
            }
        }
    }

    pub(super) fn metronome_enabled(&self) -> bool {
        self.metronome_enabled.load(Ordering::Relaxed)
    }

    pub(super) fn metronome_bpm(&self) -> f32 {
        load_f32(&self.metronome_bpm).clamp(30.0, 260.0)
    }

    pub(super) fn metronome_volume(&self) -> f32 {
        load_f32(&self.metronome_volume).clamp(0.0, 1.0)
    }

    pub(super) fn metronome_pan(&self) -> f32 {
        load_f32(&self.metronome_pan).clamp(0.0, 1.0)
    }

    pub(super) fn metronome_beats_per_bar(&self) -> u32 {
        self.metronome_beats_per_bar
            .load(Ordering::Relaxed)
            .clamp(1, 16)
    }

    pub(super) fn metronome_rhythm_division(&self) -> u32 {
        self.metronome_rhythm_division
            .load(Ordering::Relaxed)
            .clamp(1, 16)
    }

    pub(super) fn eq_enabled(&self) -> bool {
        self.eq_enabled.load(Ordering::Relaxed)
    }

    pub(super) fn eq_band_gain_db(&self, index: usize) -> f32 {
        self.eq_band_gains_db
            .get(index)
            .map(|slot| load_f32(slot.as_ref()))
            .unwrap_or(0.0)
            .clamp(-greybound_ui::EQ_MAX_GAIN_DB, greybound_ui::EQ_MAX_GAIN_DB)
    }

    pub(super) fn eq_hpf_hz(&self) -> Option<f32> {
        let frequency = load_f32(&self.eq_hpf_hz);
        (frequency > 0.0).then_some(frequency)
    }

    pub(super) fn eq_lpf_hz(&self) -> Option<f32> {
        let frequency = load_f32(&self.eq_lpf_hz);
        (frequency > 0.0).then_some(frequency)
    }

    pub(super) fn doubler_enabled(&self) -> bool {
        self.doubler_enabled.load(Ordering::Relaxed)
    }

    pub(super) fn doubler_delay_ms(&self) -> f32 {
        load_f32(&self.doubler_delay_ms).clamp(0.0, 20.0)
    }

    pub(super) fn tuner_live(&self) -> bool {
        self.tuner_live.load(Ordering::Relaxed)
    }

    pub(super) fn tuner_muted(&self) -> bool {
        self.tuner_muted.load(Ordering::Relaxed)
    }

    pub(super) fn tuner_reference_hz(&self) -> f32 {
        load_f32(&self.tuner_reference_hz).clamp(415.0, 466.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_free_controls_keep_lumen_and_springfield_bypassed() {
        let ui = GreyboundUi::default();
        let controls = SharedRuntimeControls::new(&ui);
        let mut slots = Vec::new();

        controls.load_device_controls_into(&mut slots);

        assert_eq!(slots.len(), 4);
        match slots[0].controls {
            DeviceControls::Lumen(controls) => {
                assert!(slots[0].bypassed);
                assert!(
                    (controls.peak_reduction - LumenControls::default().peak_reduction).abs()
                        < 1.0e-6
                );
                assert!((controls.gain - LumenControls::default().gain).abs() < 1.0e-6);
                assert!((controls.emphasis - LumenControls::default().emphasis).abs() < 1.0e-6);
                assert!((controls.mix - LumenControls::default().mix).abs() < 1.0e-6);
            }
            other => panic!("expected bypassed Lumen controls, got {other:?}"),
        }
        match slots[1].controls {
            DeviceControls::Minotaur(_) => {
                assert!(!slots[1].bypassed);
            }
            other => panic!("expected active Minotaur controls, got {other:?}"),
        }
        match slots[2].controls {
            DeviceControls::Auralith(controls) => {
                assert!(!slots[2].bypassed);
                assert!((controls.decay - 0.52).abs() < 1.0e-6);
                assert!((controls.size - 0.55).abs() < 1.0e-6);
                assert!((controls.texture - 0.68).abs() < 1.0e-6);
                assert!((controls.tone - 0.55).abs() < 1.0e-6);
                assert!((controls.low_cut - 0.32).abs() < 1.0e-6);
                assert!((controls.mix - 0.24).abs() < 1.0e-6);
            }
            other => panic!("expected active Auralith controls, got {other:?}"),
        }
        match slots[3].controls {
            DeviceControls::Springfield(controls) => {
                assert!(slots[3].bypassed);
                assert!((controls.dwell - 0.48).abs() < 1.0e-6);
                assert!((controls.tone - 0.58).abs() < 1.0e-6);
                assert!((controls.mix - 0.26).abs() < 1.0e-6);
            }
            other => panic!("expected bypassed Springfield controls, got {other:?}"),
        }
    }
}
