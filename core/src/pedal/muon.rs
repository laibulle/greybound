use super::common::*;

#[derive(Clone, Copy, Debug)]
pub struct MuonControls {
    pub sensitivity: f32,
    pub range: f32,
    pub resonance: f32,
    pub mix: f32,
}

impl Default for MuonControls {
    fn default() -> Self {
        Self {
            sensitivity: 0.58,
            range: 0.62,
            resonance: 0.46,
            mix: 0.82,
        }
    }
}

pub struct Muon {
    input_connection: ConnectionState,
    input_coupling: OnePoleHighpass,
    sidechain_lowpass: OnePoleLowpass,
    output_lowpass: OnePoleLowpass,
    filter: StateVariableFilter,
    sample_rate: f32,
    envelope: f32,
}

impl Muon {
    pub const INPUT_IMPEDANCE_OHMS: f32 = 1_000_000.0;
    pub const OUTPUT_IMPEDANCE_OHMS: f32 = 1_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            input_connection: ConnectionState::new(sample_rate, 180e-12),
            input_coupling: OnePoleHighpass::new(sample_rate, 24.0),
            sidechain_lowpass: OnePoleLowpass::new(sample_rate, 18.0),
            output_lowpass: OnePoleLowpass::new(sample_rate, 14_500.0),
            filter: StateVariableFilter::default(),
            sample_rate,
            envelope: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.input_connection.reset();
        self.input_coupling.reset();
        self.sidechain_lowpass.reset();
        self.output_lowpass.reset();
        self.filter.reset();
        self.envelope = 0.0;
    }

    pub fn process(&mut self, input: ElectricalSignal, controls: MuonControls) -> ElectricalSignal {
        let loaded_input = self
            .input_connection
            .drive_load(input, Load::new(Self::INPUT_IMPEDANCE_OHMS));
        self.process_loaded_voltage(loaded_input, controls)
    }

    pub fn process_loaded_voltage(
        &mut self,
        loaded_input: f32,
        controls: MuonControls,
    ) -> ElectricalSignal {
        let sensitivity = controls.sensitivity.clamp(0.0, 1.0);
        let range = controls.range.clamp(0.0, 1.0);
        let resonance = controls.resonance.clamp(0.0, 1.0);
        let mix = controls.mix.clamp(0.0, 1.0);

        let input = self.input_coupling.process(loaded_input);
        let rectified = self.sidechain_lowpass.process(input.abs());
        let target = (rectified * (5.5 + sensitivity * 48.0)).clamp(0.0, 1.0);
        let coefficient = if target > self.envelope {
            time_coefficient(self.sample_rate, 4.5)
        } else {
            time_coefficient(self.sample_rate, 135.0 + range * 120.0)
        };
        self.envelope += (target - self.envelope) * coefficient;

        let sweep = self.envelope.powf(0.56 + (1.0 - range) * 0.42);
        let base_hz = 180.0 + range * 240.0;
        let top_hz = 1_450.0 + range * 2_850.0;
        let center_hz = (base_hz * (top_hz / base_hz).powf(sweep)).clamp(80.0, 6_000.0);
        let q = 0.58 + resonance * 7.2;
        let band = self
            .filter
            .process_bandpass(input, center_hz, q, self.sample_rate);

        let body = input * (1.0 - mix * 0.42);
        let quack = band * mix * (1.85 + resonance * 1.20);
        let output = self
            .output_lowpass
            .process((body + quack).tanh() * (1.02 + sensitivity * 0.08))
            .clamp(-32.0, 32.0);

        ElectricalSignal::new(output, Self::OUTPUT_IMPEDANCE_OHMS)
    }
}
