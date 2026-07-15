use super::common::*;
use super::minotaur::{minotaur_boundaries, MinotaurCircuitParams};
use super::springfield::{
    springfield_boundaries, SpringfieldCircuitParams, SpringfieldStageVoltages,
};
use super::*;

#[test]
fn connection_state_applies_voltage_divider() {
    let mut connection = ConnectionState::new(48_000.0, 0.0);
    let output = connection.drive_load(ElectricalSignal::new(1.0, 100_000.0), Load::new(100_000.0));

    assert!((output - 0.5).abs() < 1e-6);
}

#[test]
fn muffin_exposes_low_output_impedance() {
    let mut pedal = Muffin::new(48_000.0);
    let output = pedal.process(
        ElectricalSignal::new(0.1, GUITAR_SOURCE_IMPEDANCE_OHMS),
        MuffinControls::default(),
    );

    assert_eq!(output.source_impedance_ohms, Muffin::OUTPUT_IMPEDANCE_OHMS);
    assert!(output.voltage.is_finite());
}

#[test]
fn muffin_exposes_finite_ac_boundary_voltages() {
    let mut pedal = Muffin::new(48_000.0);
    let controls = MuffinControls {
        sustain: 1.0,
        tone: 0.5,
        level: 1.0,
        wicker: 0.0,
        voicing: 0.0,
    };

    let mut last = MuffinNodeVoltages::default();
    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.04;
        let (_output, stages) =
            pedal.process_with_node_voltages(ElectricalSignal::new(input, 10_000.0), controls);
        last = stages;
    }

    for voltage in [
        last.loaded_input,
        last.q1_collector,
        last.sustain_wiper,
        last.q2_collector,
        last.q3_collector,
        last.tone_wiper,
        last.q4_collector,
        last.output,
    ] {
        assert!(voltage.is_finite());
    }
    assert!(last.q3_collector.abs() > 0.01);
    assert!(last.tone_wiper.abs() > 0.001);
}

#[test]
fn muffin_sustain_changes_transfer_curve() {
    let mut quiet = Muffin::new(48_000.0);
    let mut middle = Muffin::new(48_000.0);
    let mut driven = Muffin::new(48_000.0);
    let mut quiet_energy = 0.0;
    let mut middle_energy = 0.0;
    let mut driven_energy = 0.0;
    let mut quiet_peak = 0.0_f32;
    let mut samples = 0.0_f32;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 110.0 * sample_idx as f32 / 48_000.0).sin() * 0.02;
        let quiet_output = quiet.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 0.0,
                tone: 0.5,
                level: 0.5,
                wicker: 0.0,
                voicing: 0.0,
            },
        );
        let middle_output = middle.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 0.5,
                tone: 0.5,
                level: 0.5,
                wicker: 0.0,
                voicing: 0.0,
            },
        );
        let driven_output = driven.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 1.0,
                tone: 0.5,
                level: 0.5,
                wicker: 0.0,
                voicing: 0.0,
            },
        );
        if sample_idx >= 4_800 {
            quiet_energy += quiet_output.voltage.powi(2);
            middle_energy += middle_output.voltage.powi(2);
            driven_energy += driven_output.voltage.powi(2);
            quiet_peak = quiet_peak.max(quiet_output.voltage.abs());
            samples += 1.0;
        }
    }

    // Sustain must have useful travel below the clipping plateau.  The V3
    // circuit is a passive divider with a 1 kOhm minimum-stop resistor, not
    // an empirical gain floor: low, middle, and full must remain distinct.
    assert!(
        middle_energy > quiet_energy * 1.25,
        "quiet={quiet_energy}, middle={middle_energy}, driven={driven_energy}"
    );
    assert!(
        driven_energy > middle_energy * 1.02,
        "quiet={quiet_energy}, middle={middle_energy}, driven={driven_energy}"
    );
    let quiet_crest_factor = quiet_peak / (quiet_energy / samples).sqrt();
    assert!(
        quiet_crest_factor > 1.2,
        "low Sustain is unexpectedly hard-clipped: crest={quiet_crest_factor}, quiet={quiet_energy}"
    );
}

#[test]
fn muffin_component_circuit_stays_bounded_under_hot_drive() {
    let mut pedal = Muffin::new(48_000.0);

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 110.0 * sample_idx as f32 / 48_000.0).sin() * 0.8;
        let output = pedal.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 1.0,
                tone: 1.0,
                level: 1.0,
                wicker: 0.0,
                voicing: 0.0,
            },
        );
        assert!(output.voltage.is_finite());
        assert!(output.voltage.abs() <= 4.5);
    }
}

#[test]
fn muffin_wicker_and_voicing_controls_change_the_component_response() {
    let mut standard = Muffin::new(48_000.0);
    let mut wicker = Muffin::new(48_000.0);
    let mut rams_head = Muffin::new(48_000.0);
    let mut green_russian = Muffin::new(48_000.0);
    let mut triangle = Muffin::new(48_000.0);
    let base = MuffinControls {
        sustain: 0.78,
        tone: 0.50,
        level: 0.50,
        wicker: 0.0,
        voicing: 0.0,
    };
    let mut wicker_difference = 0.0;
    let mut rams_head_difference = 0.0;
    let mut green_russian_difference = 0.0;
    let mut triangle_difference = 0.0;

    for sample_idx in 0..12_000 {
        // High enough to exercise the lifted 470 pF filters, while remaining
        // in the intended guitar/pedal spectral range.
        let input = (std::f32::consts::TAU * 3_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.03;
        let standard_output = standard.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            base,
        );
        let wicker_output = wicker.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                wicker: 1.0,
                ..base
            },
        );
        let rams_head_output = rams_head.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                voicing: 1.0,
                ..base
            },
        );
        let green_russian_output = green_russian.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                voicing: 2.0,
                ..base
            },
        );
        let triangle_output = triangle.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                voicing: 3.0,
                ..base
            },
        );
        if sample_idx >= 6_000 {
            wicker_difference += (wicker_output.voltage - standard_output.voltage).abs();
            rams_head_difference += (rams_head_output.voltage - standard_output.voltage).abs();
            green_russian_difference +=
                (green_russian_output.voltage - standard_output.voltage).abs();
            triangle_difference += (triangle_output.voltage - standard_output.voltage).abs();
        }
    }

    assert!(
        wicker_difference > 1.0,
        "wicker difference={wicker_difference}"
    );
    assert!(
        rams_head_difference > 0.1,
        "Ram's Head difference={rams_head_difference}"
    );
    assert!(
        green_russian_difference > 0.1,
        "Green Russian difference={green_russian_difference}"
    );
    assert!(
        triangle_difference > 0.1,
        "Triangle difference={triangle_difference}"
    );
}

#[test]
fn muffin_wicker_switch_recovers_after_returning_to_standard_mode() {
    let mut pedal = Muffin::new(48_000.0);
    let base = MuffinControls {
        sustain: 1.0,
        tone: 0.50,
        level: 0.70,
        wicker: 0.0,
        voicing: 0.0,
    };
    let mut standard_energy_after_switch = 0.0;
    let mut standard_energy_before_switch = 0.0;
    let mut wicker_energy = 0.0;

    for sample_idx in 0..24_000 {
        let input = (std::f32::consts::TAU * 2_400.0 * sample_idx as f32 / 48_000.0).sin() * 0.12;
        let controls = if (8_000..16_000).contains(&sample_idx) {
            MuffinControls {
                wicker: 1.0,
                ..base
            }
        } else {
            base
        };
        let output = pedal.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            controls,
        );
        assert!(output.voltage.is_finite(), "sample={sample_idx}");
        assert!(output.voltage.abs() <= 4.5, "sample={sample_idx}");
        if (4_000..8_000).contains(&sample_idx) {
            standard_energy_before_switch += output.voltage.powi(2);
        }
        if (12_000..16_000).contains(&sample_idx) {
            wicker_energy += output.voltage.powi(2);
        }
        if sample_idx >= 20_000 {
            standard_energy_after_switch += output.voltage.powi(2);
        }
    }

    assert!(
        wicker_energy > standard_energy_before_switch * 0.50,
        "Wicker is nearly silent: wicker={wicker_energy}, standard={standard_energy_before_switch}"
    );
    assert!(
        standard_energy_after_switch > 0.01,
        "standard mode did not recover after Wicker: {standard_energy_after_switch}"
    );
}

#[test]
fn muffin_tone_wicker_bypasses_tone_control() {
    let mut dark = Muffin::new(48_000.0);
    let mut bright = Muffin::new(48_000.0);
    let mut difference = 0.0;

    for sample_idx in 0..12_000 {
        let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.04;
        let dark_output = dark.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 0.75,
                tone: 0.0,
                level: 0.50,
                wicker: 1.0,
                voicing: 0.0,
            },
        );
        let bright_output = bright.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuffinControls {
                sustain: 0.75,
                tone: 1.0,
                level: 0.50,
                wicker: 1.0,
                voicing: 0.0,
            },
        );
        if sample_idx >= 6_000 {
            difference += (dark_output.voltage - bright_output.voltage).abs();
        }
    }

    assert!(
        difference < 1.0e-5,
        "Tone affected Tone Wicker: {difference}"
    );
}

#[test]
fn muffin_wicker_stays_audible_under_hot_drive() {
    let mut pedal = Muffin::new(48_000.0);
    let controls = MuffinControls {
        sustain: 1.0,
        tone: 0.50,
        level: 1.0,
        wicker: 1.0,
        voicing: 0.0,
    };
    let mut sustained_energy = 0.0;

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.8;
        let output = pedal
            .process(
                ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
                controls,
            )
            .voltage;
        assert!(output.is_finite(), "sample={sample_idx}");
        assert!(output.abs() <= 4.5, "sample={sample_idx}");
        if sample_idx >= 24_000 {
            sustained_energy += output.powi(2);
        }
    }

    assert!(
        sustained_energy > 100.0,
        "Wicker latched into a near-silent DC state: energy={sustained_energy}"
    );
}

#[test]
fn muffin_tone_wicker_sustain_travel_never_drops_out() {
    for input_peak in [0.12, 0.30, 0.80] {
        for sustain in [0.50, 0.75, 1.0] {
            let mut pedal = Muffin::new(48_000.0);
            let controls = MuffinControls {
                sustain,
                tone: 0.50,
                level: 0.70,
                wicker: 1.0,
                voicing: 0.0,
            };
            let mut sustained_energy = 0.0;

            for sample_idx in 0..12_000 {
                let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin()
                    * input_peak;
                let output = pedal
                    .process(
                        ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
                        controls,
                    )
                    .voltage;
                assert!(output.is_finite(), "input={input_peak}, sustain={sustain}");
                if sample_idx >= 6_000 {
                    sustained_energy += output.powi(2);
                }
            }

            assert!(
                sustained_energy > 500.0,
                "Tone Wicker dropped out: input={input_peak}, sustain={sustain}, energy={sustained_energy}"
            );
        }
    }
}

#[test]
fn muffin_voice_switch_keeps_a_continuous_output_state() {
    let mut pedal = Muffin::new(48_000.0);
    let mut previous_output = 0.0;
    let mut switch_delta = 0.0;
    let mut normal_delta = 0.0_f32;

    for sample_idx in 0..18_000 {
        let input = (std::f32::consts::TAU * 330.0 * sample_idx as f32 / 48_000.0).sin() * 0.05;
        let controls = MuffinControls {
            sustain: 0.80,
            tone: 0.50,
            level: 0.50,
            wicker: 0.0,
            voicing: if sample_idx < 9_000 { 0.0 } else { 1.0 },
        };
        let output = pedal
            .process(
                ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
                controls,
            )
            .voltage;
        let delta = (output - previous_output).abs();
        if sample_idx == 9_000 {
            switch_delta = delta;
        } else if sample_idx > 1_000 && sample_idx != 9_001 {
            normal_delta = normal_delta.max(delta);
        }
        previous_output = output;
    }

    assert!(
        switch_delta <= normal_delta * 2.0,
        "Voice switch stepped too far: switch={switch_delta}, normal={normal_delta}"
    );
}

#[test]
fn minotaur_exposes_buffered_output_impedance() {
    let mut pedal = Minotaur::new(48_000.0);
    let output = pedal.process(
        ElectricalSignal::new(0.1, GUITAR_SOURCE_IMPEDANCE_OHMS),
        MinotaurControls::default(),
    );

    assert_eq!(
        output.source_impedance_ohms,
        Minotaur::OUTPUT_IMPEDANCE_OHMS
    );
    assert!(output.voltage.is_finite());
}

#[test]
fn minotaur_gain_changes_clean_drive_blend() {
    let mut low_gain = Minotaur::new(48_000.0);
    let mut high_gain = Minotaur::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.12;
        let low_output = low_gain.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MinotaurControls {
                gain: 0.05,
                treble: 0.5,
                output: 0.5,
            },
        );
        let high_output = high_gain.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MinotaurControls {
                gain: 0.9,
                treble: 0.5,
                output: 0.5,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (high_output.voltage - low_output.voltage).abs();
        }
    }

    assert!(difference_sum > 10.0);
}

#[test]
fn minotaur_treble_changes_presence_band() {
    let mut dark = Minotaur::new(48_000.0);
    let mut bright = Minotaur::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 2_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.04;
        let dark_output = dark.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MinotaurControls {
                gain: 0.35,
                treble: 0.05,
                output: 0.5,
            },
        );
        let bright_output = bright.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MinotaurControls {
                gain: 0.35,
                treble: 0.95,
                output: 0.5,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (bright_output.voltage - dark_output.voltage).abs();
        }
    }

    assert!(difference_sum > 2.0);
}

#[test]
fn minotaur_reference_setting_tracks_spice_output_driver_gain() {
    let mut pedal = Minotaur::new(48_000.0);
    let mut input_sum = 0.0;
    let mut output_sum = 0.0;
    let mut count = 0.0;

    for sample_idx in 0..12_000 {
        let input = (std::f32::consts::TAU * 1_000.0 * sample_idx as f32 / 48_000.0).sin() * 0.12;
        let output = pedal.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MinotaurControls {
                gain: 0.55,
                treble: 0.60,
                output: 0.70,
            },
        );
        if sample_idx >= 6_000 {
            input_sum += input * input;
            output_sum += output.voltage * output.voltage;
            count += 1.0;
        }
    }

    let input_rms = (input_sum / count).sqrt();
    let output_rms = (output_sum / count).sqrt();
    let gain = output_rms / input_rms;

    assert!(
        (10.0..13.5).contains(&gain),
        "input_rms={input_rms}, output_rms={output_rms}, gain={gain}"
    );
}

#[test]
fn minotaur_current_clip_knee_stays_below_silicon_range() {
    let clipped = diode_pair_clip(1.2, MinotaurCircuitParams::current().clip_knee_v);

    assert!((0.34..0.37).contains(&clipped), "clipped={clipped}");
}

#[test]
fn minotaur_current_soft_clip_candidate_is_finite_and_audible() {
    let mut pedal = Minotaur::new(48_000.0);
    let controls = MinotaurControls {
        gain: 0.42,
        treble: 0.70,
        output: 0.42,
    };
    let mut output_energy = 0.0_f32;

    for sample_idx in 0..24_000 {
        let t = sample_idx as f32 / 48_000.0;
        let input = (std::f32::consts::TAU * 110.0 * t).sin() * 0.055
            + (std::f32::consts::TAU * 880.0 * t).sin() * 0.026
            + (std::f32::consts::TAU * 3_520.0 * t).sin() * 0.012;
        let source = ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS);
        let output = pedal.process(source, controls);
        assert!(output.voltage.is_finite());
        if sample_idx >= 4_800 {
            output_energy += output.voltage * output.voltage;
        }
    }

    assert!(
        output_energy > 1e-5,
        "current Minotaur produced suspicious silence: output_energy={output_energy}"
    );
}

#[test]
fn minotaur_current_path_exports_stage_boundary_states() {
    let mut pedal = Minotaur::new(48_000.0);
    let mut stages = MinotaurNodeVoltages::default();
    for sample_idx in 0..2_400 {
        let input = (std::f32::consts::TAU * 440.0 * sample_idx as f32 / 48_000.0).sin() * 0.12;
        let loaded_input = pedal.input_connection.drive_load(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            Load::new(Minotaur::INPUT_IMPEDANCE_OHMS),
        );
        stages = pedal
            .process_loaded_voltage_with_stages(
                loaded_input,
                MinotaurControls::default(),
                MinotaurCircuitParams::current(),
            )
            .stages;
    }

    let boundaries = minotaur_boundaries(stages);
    assert_eq!(boundaries[0].id, "input_load");
    assert_eq!(boundaries[7].id, "output_driver");
    assert_eq!(
        boundaries[7].source_impedance_ohms,
        Minotaur::OUTPUT_IMPEDANCE_OHMS
    );
    assert!(boundaries.iter().all(|boundary| {
        boundary.voltage_v.is_finite()
            && boundary.source_impedance_ohms.is_finite()
            && boundary.load_impedance_ohms.is_finite()
    }));
    assert!(boundaries
        .iter()
        .any(|boundary| boundary.nominal_level_v > 0.0));
}

#[test]
fn monarch_exposes_buffered_output_impedance() {
    let mut pedal = Monarch::new(48_000.0);
    let output = pedal.process(
        ElectricalSignal::new(0.1, GUITAR_SOURCE_IMPEDANCE_OHMS),
        MonarchControls::default(),
    );

    assert_eq!(output.source_impedance_ohms, Monarch::OUTPUT_IMPEDANCE_OHMS);
    assert!(output.voltage.is_finite());
}

#[test]
fn monarch_gain_changes_dual_clip_drive() {
    let mut low_gain = Monarch::new(48_000.0);
    let mut high_gain = Monarch::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.10;
        let low_output = low_gain.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MonarchControls {
                gain: 0.05,
                tone: 0.5,
                output: 0.55,
            },
        );
        let high_output = high_gain.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MonarchControls {
                gain: 0.92,
                tone: 0.5,
                output: 0.55,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (high_output.voltage - low_output.voltage).abs();
        }
    }

    assert!(difference_sum > 8.0);
}

#[test]
fn monarch_tone_changes_high_band() {
    let mut dark = Monarch::new(48_000.0);
    let mut bright = Monarch::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 1_600.0 * sample_idx as f32 / 48_000.0).sin() * 0.05;
        let dark_output = dark.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MonarchControls {
                gain: 0.45,
                tone: 0.05,
                output: 0.55,
            },
        );
        let bright_output = bright.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MonarchControls {
                gain: 0.45,
                tone: 0.95,
                output: 0.55,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (bright_output.voltage - dark_output.voltage).abs();
        }
    }

    assert!(difference_sum > 2.0);
}

#[test]
fn godess_one_exposes_boss_style_buffered_output_impedance() {
    let mut pedal = GodessOne::new(48_000.0);
    let output = pedal.process(
        ElectricalSignal::new(0.1, GUITAR_SOURCE_IMPEDANCE_OHMS),
        GodessOneControls::default(),
    );

    assert_eq!(
        output.source_impedance_ohms,
        GodessOne::OUTPUT_IMPEDANCE_OHMS
    );
    assert!(output.voltage.is_finite());
}

#[test]
fn godess_one_distortion_changes_hard_clip_drive() {
    let mut low_distortion = GodessOne::new(48_000.0);
    let mut high_distortion = GodessOne::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.10;
        let low_output = low_distortion.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.08,
                tone: 0.5,
                level: 0.55,
                mode: GodessOneMode::Standard,
            },
        );
        let high_output = high_distortion.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.95,
                tone: 0.5,
                level: 0.55,
                mode: GodessOneMode::Standard,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (high_output.voltage - low_output.voltage).abs();
        }
    }

    assert!(difference_sum > 8.0);
}

#[test]
fn godess_one_tone_changes_bright_edge() {
    let mut dark = GodessOne::new(48_000.0);
    let mut bright = GodessOne::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 1_800.0 * sample_idx as f32 / 48_000.0).sin() * 0.05;
        let dark_output = dark.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.55,
                tone: 0.05,
                level: 0.55,
                mode: GodessOneMode::Standard,
            },
        );
        let bright_output = bright.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.55,
                tone: 0.95,
                level: 0.55,
                mode: GodessOneMode::Standard,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (bright_output.voltage - dark_output.voltage).abs();
        }
    }

    assert!(difference_sum > 2.0);
}

#[test]
fn godess_one_custom_mode_changes_voice() {
    let mut standard = GodessOne::new(48_000.0);
    let mut custom = GodessOne::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..9_600 {
        let input = (std::f32::consts::TAU * 165.0 * sample_idx as f32 / 48_000.0).sin() * 0.11;
        let standard_output = standard.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.62,
                tone: 0.48,
                level: 0.55,
                mode: GodessOneMode::Standard,
            },
        );
        let custom_output = custom.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            GodessOneControls {
                distortion: 0.62,
                tone: 0.48,
                level: 0.55,
                mode: GodessOneMode::Custom,
            },
        );
        if sample_idx >= 4_800 {
            difference_sum += (custom_output.voltage - standard_output.voltage).abs();
        }
    }

    assert!(difference_sum > 4.0);
}

#[test]
fn dartford_depth_modulates_level() {
    let mut dry = Dartford::new(48_000.0);
    let mut wet = Dartford::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.2;
        let dry_output = dry.process(
            ElectricalSignal::new(input, 1_000.0),
            DartfordControls {
                rate_hz: 5.0,
                depth: 0.0,
                level: 1.0,
                wave: DartfordWave::Sine,
            },
        );
        let wet_output = wet.process(
            ElectricalSignal::new(input, 1_000.0),
            DartfordControls {
                rate_hz: 5.0,
                depth: 0.85,
                level: 1.0,
                wave: DartfordWave::Sine,
            },
        );
        if sample_idx >= 24_000 {
            difference_sum += (wet_output.voltage - dry_output.voltage).abs();
        }
    }

    assert!(difference_sum > 500.0);
}

#[test]
fn tron_depth_moves_phase_notches() {
    let mut dry = Tron::new(48_000.0);
    let mut wet = Tron::new(48_000.0);
    let mut difference_sum = 0.0;
    let mut wet_sum = 0.0;

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 330.0 * sample_idx as f32 / 48_000.0).sin() * 0.14
            + (std::f32::consts::TAU * 880.0 * sample_idx as f32 / 48_000.0).sin() * 0.08;
        let dry_output = dry.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            TronControls {
                rate_hz: 0.8,
                depth: 0.0,
                feedback: 0.2,
                mix: 0.5,
            },
        );
        let wet_output = wet.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            TronControls {
                rate_hz: 0.8,
                depth: 0.85,
                feedback: 0.45,
                mix: 0.7,
            },
        );
        if sample_idx >= 24_000 {
            difference_sum += (wet_output.voltage - dry_output.voltage).abs();
            wet_sum += wet_output.voltage.abs();
        }
    }

    assert!(difference_sum > 100.0, "difference_sum={difference_sum}");
    assert!(wet_sum > 50.0, "wet_sum={wet_sum}");
}

#[test]
fn jetstream_depth_sweeps_short_delay_comb() {
    let mut shallow = Jetstream::new(48_000.0);
    let mut deep = Jetstream::new(48_000.0);
    let mut difference_sum = 0.0;
    let mut deep_sum = 0.0;

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.10
            + (std::f32::consts::TAU * 880.0 * sample_idx as f32 / 48_000.0).sin() * 0.08;
        let shallow_output = shallow.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            JetstreamControls {
                manual: 0.42,
                rate_hz: 0.35,
                depth: 0.05,
                feedback: 0.18,
                mix: 0.56,
            },
        );
        let deep_output = deep.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            JetstreamControls {
                manual: 0.42,
                rate_hz: 0.35,
                depth: 0.85,
                feedback: 0.56,
                mix: 0.64,
            },
        );
        if sample_idx >= 24_000 {
            difference_sum += (deep_output.voltage - shallow_output.voltage).abs();
            deep_sum += deep_output.voltage.abs();
        }
    }

    assert!(difference_sum > 150.0, "difference_sum={difference_sum}");
    assert!(deep_sum > 50.0, "deep_sum={deep_sum}");
}

#[test]
fn celeste_depth_adds_modulated_bbd_chorus() {
    let mut shallow = Celeste::new(48_000.0);
    let mut deep = Celeste::new(48_000.0);
    let mut difference_sum = 0.0;
    let mut deep_sum = 0.0;

    for sample_idx in 0..48_000 {
        let input = (std::f32::consts::TAU * 247.0 * sample_idx as f32 / 48_000.0).sin() * 0.10
            + (std::f32::consts::TAU * 741.0 * sample_idx as f32 / 48_000.0).sin() * 0.06;
        let shallow_output = shallow.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            CelesteControls {
                rate_hz: 0.62,
                depth: 0.05,
                tone: 0.55,
                mix: 0.42,
            },
        );
        let deep_output = deep.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            CelesteControls {
                rate_hz: 0.62,
                depth: 0.82,
                tone: 0.62,
                mix: 0.50,
            },
        );
        if sample_idx >= 24_000 {
            difference_sum += (deep_output.voltage - shallow_output.voltage).abs();
            deep_sum += deep_output.voltage.abs();
        }
    }

    assert!(difference_sum > 80.0, "difference_sum={difference_sum}");
    assert!(deep_sum > 40.0, "deep_sum={deep_sum}");
}

#[test]
fn brigade_repeats_create_dark_delay_tail() {
    let mut dry = Brigade::new(48_000.0);
    let mut echo = Brigade::new(48_000.0);
    let mut dry_tail_sum = 0.0;
    let mut echo_tail_sum = 0.0;

    for sample_idx in 0..36_000 {
        let input = if sample_idx < 400 {
            (std::f32::consts::TAU * 180.0 * sample_idx as f32 / 48_000.0).sin() * 0.2
        } else {
            0.0
        };
        let dry_output = dry.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            BrigadeControls {
                time_ms: 160.0,
                repeats: 0.0,
                tone: 0.45,
                mix: 0.0,
            },
        );
        let echo_output = echo.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            BrigadeControls {
                time_ms: 160.0,
                repeats: 0.56,
                tone: 0.36,
                mix: 0.45,
            },
        );
        if sample_idx >= 7_680 {
            dry_tail_sum += dry_output.voltage.abs();
            echo_tail_sum += echo_output.voltage.abs();
        }
    }

    assert!(
        echo_tail_sum > dry_tail_sum + 10.0,
        "dry_tail_sum={dry_tail_sum}, echo_tail_sum={echo_tail_sum}"
    );
}

#[test]
fn lumen_peak_reduction_levels_loud_guitar_segments() {
    let mut open = Lumen::new(48_000.0);
    let mut compressed = Lumen::new(48_000.0);
    let mut open_quiet_sum = 0.0;
    let mut open_loud_sum = 0.0;
    let mut compressed_quiet_sum = 0.0;
    let mut compressed_loud_sum = 0.0;

    for sample_idx in 0..96_000 {
        let loud = sample_idx >= 48_000;
        let amplitude = if loud { 0.22 } else { 0.035 };
        let input =
            (std::f32::consts::TAU * 196.0 * sample_idx as f32 / 48_000.0).sin() * amplitude;
        let open_output = open.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            LumenControls {
                peak_reduction: 0.0,
                gain: 0.5,
                emphasis: 0.44,
                mix: 0.0,
            },
        );
        let compressed_output = compressed.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            LumenControls {
                peak_reduction: 0.74,
                gain: 0.52,
                emphasis: 0.48,
                mix: 1.0,
            },
        );
        if (36_000..48_000).contains(&sample_idx) {
            open_quiet_sum += open_output.voltage.abs();
            compressed_quiet_sum += compressed_output.voltage.abs();
        } else if sample_idx >= 84_000 {
            open_loud_sum += open_output.voltage.abs();
            compressed_loud_sum += compressed_output.voltage.abs();
        }
    }

    let open_ratio = open_loud_sum / open_quiet_sum.max(1e-6);
    let compressed_ratio = compressed_loud_sum / compressed_quiet_sum.max(1e-6);
    assert!(
        compressed_ratio < open_ratio * 0.72,
        "open_ratio={open_ratio}, compressed_ratio={compressed_ratio}"
    );
    assert!(
        compressed_loud_sum > 40.0,
        "compressed_loud_sum={compressed_loud_sum}"
    );
}

#[test]
fn muon_envelope_opens_filter_on_guitar_attacks() {
    let mut subtle = Muon::new(48_000.0);
    let mut open = Muon::new(48_000.0);
    let mut difference_sum = 0.0;
    let mut open_sum = 0.0;

    for sample_idx in 0..48_000 {
        let burst = if sample_idx % 12_000 < 2_200 {
            1.0
        } else {
            0.18
        };
        let decay = (1.0 - (sample_idx % 12_000) as f32 / 12_000.0).clamp(0.0, 1.0);
        let amplitude = 0.035 + burst * decay * 0.12;
        let input = (std::f32::consts::TAU * 164.0 * sample_idx as f32 / 48_000.0).sin()
            * amplitude
            + (std::f32::consts::TAU * 492.0 * sample_idx as f32 / 48_000.0).sin()
                * amplitude
                * 0.55;
        let subtle_output = subtle.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuonControls {
                sensitivity: 0.18,
                range: 0.26,
                resonance: 0.16,
                mix: 0.25,
            },
        );
        let open_output = open.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            MuonControls {
                sensitivity: 0.72,
                range: 0.78,
                resonance: 0.60,
                mix: 0.90,
            },
        );
        if sample_idx >= 12_000 {
            difference_sum += (open_output.voltage - subtle_output.voltage).abs();
            open_sum += open_output.voltage.abs();
        }
    }

    assert!(difference_sum > 60.0, "difference_sum={difference_sum}");
    assert!(open_sum > 20.0, "open_sum={open_sum}");
}

#[test]
fn springfield_mix_adds_spring_tail() {
    let mut dry = Springfield::new(48_000.0);
    let mut wet = Springfield::new(48_000.0);
    let mut dry_sum = 0.0;
    let mut wet_tail_sum = 0.0;

    for sample_idx in 0..12_000 {
        let input = if sample_idx == 0 { 0.8 } else { 0.0 };
        let dry_output = dry.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            SpringfieldControls {
                dwell: 0.45,
                tone: 0.5,
                mix: 0.0,
            },
        );
        let wet_output = wet.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            SpringfieldControls {
                dwell: 0.65,
                tone: 0.58,
                mix: 0.55,
            },
        );
        if sample_idx < 512 {
            dry_sum += dry_output.voltage.abs();
        }
        if sample_idx > 4_000 {
            wet_tail_sum += wet_output.voltage.abs();
        }
    }

    assert!(dry_sum > 0.1, "dry_sum={dry_sum}");
    assert!(wet_tail_sum > 0.05, "wet_tail_sum={wet_tail_sum}");
}

#[test]
fn springfield_tone_changes_tail_color() {
    let mut dark = Springfield::new(48_000.0);
    let mut bright = Springfield::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..16_000 {
        let input = if sample_idx % 997 == 0 { 0.35 } else { 0.0 };
        let dark_output = dark.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            SpringfieldControls {
                dwell: 0.55,
                tone: 0.15,
                mix: 0.45,
            },
        );
        let bright_output = bright.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            SpringfieldControls {
                dwell: 0.55,
                tone: 0.9,
                mix: 0.45,
            },
        );
        if sample_idx > 4_000 {
            difference_sum += (bright_output.voltage - dark_output.voltage).abs();
        }
    }

    assert!(difference_sum > 0.2, "difference_sum={difference_sum}");
}

#[test]
fn springfield_current_recovery_is_finite_and_audible() {
    let mut pedal = Springfield::new(48_000.0);
    let controls = SpringfieldControls {
        dwell: 0.58,
        tone: 0.62,
        mix: 0.42,
    };
    let mut output_sum = 0.0;

    for sample_idx in 0..24_000 {
        let input = (std::f32::consts::TAU * 147.0 * sample_idx as f32 / 48_000.0).sin() * 0.08
            + if sample_idx % 1_507 == 0 { 0.32 } else { 0.0 };
        let output = pedal.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            controls,
        );
        assert!(output.voltage.is_finite());
        if sample_idx > 6_000 {
            output_sum += output.voltage.abs();
        }
    }

    assert!(output_sum > 5.0, "output_sum={output_sum}");
}

#[test]
fn springfield_current_path_exports_stage_boundary_states() {
    let mut pedal = Springfield::new(48_000.0);
    let mut stages = SpringfieldStageVoltages::default();
    for sample_idx in 0..4_096 {
        let input = if sample_idx == 0 { 0.7 } else { 0.0 };
        let loaded_input = pedal.input_connection.drive_load(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            Load::new(Springfield::INPUT_IMPEDANCE_OHMS),
        );
        let result = pedal.process_loaded_voltage_with_stages(
            loaded_input,
            SpringfieldControls {
                dwell: 0.65,
                tone: 0.58,
                mix: 0.55,
            },
            SpringfieldCircuitParams::current(),
        );
        assert!(
            result.signal.voltage.is_finite(),
            "output={:?}",
            result.signal
        );
        stages = result.stages;
    }

    let states = springfield_boundaries(stages);
    assert_eq!(states[0].id, "input_load");
    assert_eq!(states[2].id, "dwell_driver");
    assert_eq!(states[3].id, "spring_ir_tank");
    assert_eq!(states[7].id, "output_driver");
    assert!(
        states.iter().any(|state| state.nominal_level_v > 0.0),
        "states={states:?}"
    );
}

#[test]
fn auralith_mix_adds_dense_space_tail() {
    let mut dry = Auralith::new(48_000.0);
    let mut wet = Auralith::new(48_000.0);
    let mut dry_tail_sum = 0.0;
    let mut wet_tail_sum = 0.0;
    let mut wet_peak = 0.0_f32;

    for sample_idx in 0..24_000 {
        let input = if sample_idx == 0 { 0.8 } else { 0.0 };
        let dry_output = dry.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            AuralithControls {
                mix: 0.0,
                ..AuralithControls::default()
            },
        );
        let wet_output = wet.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            AuralithControls {
                decay: 0.62,
                size: 0.66,
                texture: 0.72,
                tone: 0.58,
                low_cut: 0.34,
                mix: 0.55,
            },
        );
        if sample_idx > 7_000 {
            dry_tail_sum += dry_output.voltage.abs();
            wet_tail_sum += wet_output.voltage.abs();
            wet_peak = wet_peak.max(wet_output.voltage.abs());
        }
    }

    assert!(
        wet_tail_sum > dry_tail_sum + 0.15,
        "dry_tail_sum={dry_tail_sum}, wet_tail_sum={wet_tail_sum}"
    );
    assert!(wet_peak < 1.0, "wet_peak={wet_peak}");
}

#[test]
fn studioverb_mix_adds_room_tail() {
    let mut dry = StudioVerb::new(48_000.0);
    let mut wet = StudioVerb::new(48_000.0);
    let mut dry_tail_sum = 0.0;
    let mut wet_tail_sum = 0.0;
    let mut wet_peak = 0.0_f32;

    for sample_idx in 0..18_000 {
        let input = if sample_idx == 0 { 0.8 } else { 0.0 };
        let dry_output = dry.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            StudioVerbControls {
                algorithm: StudioVerbAlgorithm::Room,
                decay: 0.55,
                size: 0.5,
                pre_delay_ms: 10.0,
                diffusion: 0.7,
                tone: 0.55,
                low_cut: 0.45,
                mod_depth: 0.15,
                mix: 0.0,
            },
        );
        let wet_output = wet.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            StudioVerbControls {
                algorithm: StudioVerbAlgorithm::Room,
                decay: 0.55,
                size: 0.5,
                pre_delay_ms: 10.0,
                diffusion: 0.7,
                tone: 0.55,
                low_cut: 0.45,
                mod_depth: 0.15,
                mix: 0.45,
            },
        );
        if sample_idx > 6_000 {
            dry_tail_sum += dry_output.voltage.abs();
            wet_tail_sum += wet_output.voltage.abs();
            wet_peak = wet_peak.max(wet_output.voltage.abs());
        }
    }

    assert!(
        wet_tail_sum > dry_tail_sum + 0.12,
        "dry_tail_sum={dry_tail_sum}, wet_tail_sum={wet_tail_sum}"
    );
    assert!(wet_peak < 1.0, "wet_peak={wet_peak}");
}

#[test]
fn studioverb_plate_and_room_have_distinct_tails() {
    let mut room = StudioVerb::new(48_000.0);
    let mut plate = StudioVerb::new(48_000.0);
    let mut difference_sum = 0.0;

    for sample_idx in 0..22_000 {
        let input = if sample_idx % 1_337 == 0 { 0.45 } else { 0.0 };
        let room_output = room.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            StudioVerbControls {
                algorithm: StudioVerbAlgorithm::Room,
                decay: 0.58,
                size: 0.54,
                pre_delay_ms: 12.0,
                diffusion: 0.72,
                tone: 0.55,
                low_cut: 0.4,
                mod_depth: 0.16,
                mix: 0.42,
            },
        );
        let plate_output = plate.process(
            ElectricalSignal::new(input, GUITAR_SOURCE_IMPEDANCE_OHMS),
            StudioVerbControls {
                algorithm: StudioVerbAlgorithm::Plate,
                decay: 0.58,
                size: 0.54,
                pre_delay_ms: 12.0,
                diffusion: 0.72,
                tone: 0.55,
                low_cut: 0.4,
                mod_depth: 0.16,
                mix: 0.42,
            },
        );
        if sample_idx > 5_000 {
            difference_sum += (plate_output.voltage - room_output.voltage).abs();
        }
    }

    assert!(difference_sum > 0.4, "difference_sum={difference_sum}");
}

#[test]
fn studiodelay_mix_adds_repeats_after_delay_time() {
    let mut dry = StudioDelay::new(48_000.0);
    let mut wet = StudioDelay::new(48_000.0);
    let mut dry_tail = 0.0;
    let mut wet_tail = 0.0;

    for index in 0..30_000 {
        let input = if index == 0 { 1.0 } else { 0.0 };
        let dry_out = dry
            .process_loaded_voltage(
                input,
                StudioDelayControls {
                    mix: 0.0,
                    ..StudioDelayControls::default()
                },
            )
            .voltage;
        let wet_out = wet
            .process_loaded_voltage(
                input,
                StudioDelayControls {
                    time_ms: 120.0,
                    feedback: 0.38,
                    mix: 0.35,
                    ..StudioDelayControls::default()
                },
            )
            .voltage;
        if index > 6_000 {
            dry_tail += dry_out.abs();
            wet_tail += wet_out.abs();
        }
    }

    assert!(wet_tail.is_finite());
    assert!(
        wet_tail > dry_tail + 0.01,
        "dry_tail={dry_tail}, wet_tail={wet_tail}"
    );
}
