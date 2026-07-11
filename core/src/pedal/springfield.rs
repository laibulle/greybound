use super::common::*;
use super::*;

const SPRINGFIELD_TANK_IR_BYTES: &[u8] =
    include_bytes!("../../../lab/references/spring-irs/smac2023/fig7a-full-modal-model.wav");

#[derive(Clone, Copy, Debug)]
pub struct SpringfieldControls {
    pub dwell: f32,
    pub tone: f32,
    pub mix: f32,
}

impl Default for SpringfieldControls {
    fn default() -> Self {
        Self {
            dwell: 0.42,
            tone: 0.54,
            mix: 0.24,
        }
    }
}

pub struct Springfield {
    pub(super) input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    pre_emphasis: OnePoleHighpass,
    tank_lowpass: OnePoleLowpass,
    bright_highpass: OnePoleHighpass,
    space_lowpass: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    delays: [SpringDelay; 4],
    space_delays: [SpringDelay; 4],
    tank_ir: Option<SpeakerStage>,
    feedback: f32,
    space_feedback: f32,
    last_boundary_states: [StageBoundaryState; 8],
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SpringfieldStageVoltages {
    loaded_input: f32,
    coupled: f32,
    tank_drive: f32,
    ir_tank: f32,
    splash: f32,
    voiced: f32,
    wet: f32,
    output: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SpringfieldProcessResult {
    pub(super) signal: ElectricalSignal,
    pub(super) stages: SpringfieldStageVoltages,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SpringfieldCircuitParams {
    tank_drive_gain: f32,
    splash_gain: f32,
    recovery_brightness: f32,
    wet_makeup_gain: f32,
}

impl Springfield {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        let tank_ir = SpeakerStage::from_wav_bytes(
            SPRINGFIELD_TANK_IR_BYTES,
            sample_rate.round().max(1.0) as u32,
        )
        .ok();
        Self {
            input_connection: ConnectionState::new(sample_rate, 150e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 55.0),
            pre_emphasis: OnePoleHighpass::new(sample_rate, 1_100.0),
            tank_lowpass: OnePoleLowpass::new(sample_rate, 4_200.0),
            bright_highpass: OnePoleHighpass::new(sample_rate, 1_850.0),
            space_lowpass: OnePoleLowpass::new(sample_rate, 3_400.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 9_500.0),
            delays: [
                SpringDelay::new(sample_rate, 0.029),
                SpringDelay::new(sample_rate, 0.037),
                SpringDelay::new(sample_rate, 0.053),
                SpringDelay::new(sample_rate, 0.071),
            ],
            space_delays: [
                SpringDelay::new(sample_rate, 0.089),
                SpringDelay::new(sample_rate, 0.127),
                SpringDelay::new(sample_rate, 0.173),
                SpringDelay::new(sample_rate, 0.211),
            ],
            tank_ir,
            feedback: 0.0,
            space_feedback: 0.0,
            last_boundary_states: springfield_boundaries(SpringfieldStageVoltages::default()),
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.pre_emphasis.reset();
        self.tank_lowpass.reset();
        self.bright_highpass.reset();
        self.space_lowpass.reset();
        self.output_lowpass.reset();
        for delay in &mut self.delays {
            delay.reset();
        }
        for delay in &mut self.space_delays {
            delay.reset();
        }
        if let Some(tank_ir) = &mut self.tank_ir {
            tank_ir.reset();
        }
        self.feedback = 0.0;
        self.space_feedback = 0.0;
        self.last_boundary_states = springfield_boundaries(SpringfieldStageVoltages::default());
    }

    pub fn process(
        &mut self,
        input: ElectricalSignal,
        controls: SpringfieldControls,
    ) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: SpringfieldControls,
    ) -> ElectricalSignal {
        let result = self.process_loaded_voltage_with_stages(
            loaded_input,
            controls,
            SpringfieldCircuitParams::current(),
        );
        self.last_boundary_states = springfield_boundaries(result.stages);
        result.signal
    }

    pub fn boundary_states(&self) -> [StageBoundaryState; 8] {
        self.last_boundary_states
    }

    pub(super) fn process_loaded_voltage_with_stages(
        &mut self,
        loaded_input: f32,
        controls: SpringfieldControls,
        params: SpringfieldCircuitParams,
    ) -> SpringfieldProcessResult {
        let dwell = controls.dwell.clamp(0.0, 1.0);
        let tone = controls.tone.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        let dry = loaded_input;
        let coupled = self.input_coupling.process(loaded_input);
        let excited = coupled + self.pre_emphasis.process(coupled) * (0.08 + tone * 0.24);
        let driver_gain = (0.30 + dwell * 0.95) * params.tank_drive_gain;
        let driver_feedback = self.feedback * (0.035 + dwell * 0.10);
        let tank_drive = (excited * driver_gain + driver_feedback).tanh();

        let a = self.delays[0].process(tank_drive + self.feedback * 0.06);
        let b = self.delays[1].process(-tank_drive * 0.74 + a * 0.38);
        let c = self.delays[2].process(tank_drive * 0.52 - b * 0.31);
        let d = self.delays[3].process(-tank_drive * 0.46 + c * 0.27);
        let splash = a * 0.34 - b * 0.29 + c * 0.25 - d * 0.22;
        self.feedback = (splash * (0.15 + dwell * 0.11)).clamp(-0.58, 0.58);

        let ir_drive = tank_drive * (0.010 + dwell * 0.017);
        let ir_tank = self
            .tank_ir
            .as_mut()
            .map_or(splash, |tank_ir| tank_ir.process(ir_drive, true));
        let spring_tank = ir_tank + splash * (0.11 + tone * 0.045) * params.splash_gain;
        let space_a = self.space_delays[0].process(spring_tank * 0.42 + self.space_feedback * 0.18);
        let space_b = self.space_delays[1].process(-spring_tank * 0.31 + space_a * 0.36);
        let space_c = self.space_delays[2].process(spring_tank * 0.24 - space_b * 0.32);
        let space_d = self.space_delays[3].process(-spring_tank * 0.19 + space_c * 0.28);
        let space_raw = space_a * 0.36 - space_b * 0.31 + space_c * 0.27 - space_d * 0.23;
        self.space_feedback = (space_raw * (0.20 + dwell * 0.18)).clamp(-0.54, 0.54);
        let space = self.space_lowpass.process(space_raw + spring_tank * 0.08);
        let tank = spring_tank + space * (0.18 + dwell * 0.12);

        let dark = self.tank_lowpass.process(tank);
        let bright = self.bright_highpass.process(tank);
        let brightness = (tone + params.recovery_brightness).clamp(0.0, 1.0);
        let voiced = dark * (1.20 - brightness * 0.38) + bright * (0.08 + brightness * 0.54);
        let wet = self
            .output_lowpass
            .process(voiced * (0.28 + dwell * 0.18) * params.wet_makeup_gain)
            .clamp(-1.0, 1.0);
        let output = dry + wet * mix * 1.8;

        let output = output.clamp(-32.0, 32.0);
        SpringfieldProcessResult {
            signal: ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS),
            stages: SpringfieldStageVoltages {
                loaded_input,
                coupled,
                tank_drive,
                ir_tank,
                splash,
                voiced,
                wet,
                output,
            },
        }
    }
}

impl SpringfieldCircuitParams {
    pub(super) fn current() -> Self {
        Self {
            tank_drive_gain: 1.015,
            splash_gain: 0.82,
            recovery_brightness: -0.015,
            wet_makeup_gain: 0.96,
        }
    }
}

pub(super) fn springfield_boundaries(stages: SpringfieldStageVoltages) -> [StageBoundaryState; 8] {
    [
        pedal_boundary_state(
            "input_load",
            stages.loaded_input,
            AMP_INPUT_IMPEDANCE_OHMS,
            Springfield::INPUT_IMPEDANCE_OHMS,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "input_coupling",
            stages.coupled,
            1_000.0,
            470_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "dwell_driver",
            stages.tank_drive,
            8_200.0,
            8.0,
            StageCoupling::Buffered,
            1.0,
        ),
        pedal_boundary_state(
            "spring_ir_tank",
            stages.ir_tank,
            8.0,
            600.0,
            StageCoupling::AcCoupled,
            1.0,
        ),
        pedal_boundary_state(
            "splash_diffusion",
            stages.splash,
            22_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "recovery_tone",
            stages.voiced,
            47_000.0,
            100_000.0,
            StageCoupling::AcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "wet_dry_mixer",
            stages.wet,
            10_000.0,
            100_000.0,
            StageCoupling::DcCoupled,
            9.0,
        ),
        pedal_boundary_state(
            "output_driver",
            stages.output,
            Springfield::OUTPUT_IMPEDANCE_OHMS,
            AMP_INPUT_IMPEDANCE_OHMS,
            StageCoupling::Buffered,
            9.0,
        ),
    ]
}
