#[derive(Clone, Copy)]
pub struct PushPullEl84Params {
    pub sample_rate: f32,
    pub nominal_supply_voltage: f32,
    pub supply_resistance: f32,
    pub supply_capacitance: f32,
    pub cathode_resistance: f32,
    pub cathode_capacitance: f32,
    pub idle_current: f32,
    pub drive_gain: f32,
    pub current_gain: f32,
    pub compression: f32,
    pub output_scale: f32,
}

pub struct PushPullEl84Stage {
    params: PushPullEl84Params,
    supply_voltage: f32,
    cathode_bias_voltage: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PushPullEl84OperatingPoint {
    pub supply_voltage: f32,
    pub cathode_bias_voltage: f32,
    pub positive_current: f32,
    pub negative_current: f32,
}

impl PushPullEl84Stage {
    pub fn new(params: PushPullEl84Params) -> Self {
        let idle_bias = params.idle_current * params.cathode_resistance;
        Self {
            params,
            supply_voltage: params.nominal_supply_voltage,
            cathode_bias_voltage: idle_bias,
        }
    }

    pub fn reset(&mut self) {
        self.supply_voltage = self.params.nominal_supply_voltage;
        self.cathode_bias_voltage = self.params.idle_current * self.params.cathode_resistance;
    }

    pub fn operating_point(&self) -> PushPullEl84OperatingPoint {
        let supply_ratio = self.supply_ratio();
        PushPullEl84OperatingPoint {
            supply_voltage: self.supply_voltage,
            cathode_bias_voltage: self.cathode_bias_voltage,
            positive_current: self.tube_current(0.0, supply_ratio),
            negative_current: self.tube_current(0.0, supply_ratio),
        }
    }

    pub fn process(&mut self, drive: f32, sag: f32) -> f32 {
        let supply_ratio = self.supply_ratio();
        let drive = drive * self.params.drive_gain * supply_ratio;
        let bias_offset = (self.cathode_bias_voltage
            - self.params.idle_current * self.params.cathode_resistance)
            * 0.030;
        let positive_current = self.tube_current(drive - bias_offset, supply_ratio);
        let negative_current = self.tube_current(-drive - bias_offset, supply_ratio);
        let total_current = positive_current + negative_current;

        self.update_supply(total_current, sag);
        self.update_cathode_bias(total_current);

        (positive_current - negative_current) * self.params.output_scale * self.supply_ratio()
    }

    fn tube_current(&self, grid_drive: f32, supply_ratio: f32) -> f32 {
        let threshold = -0.18 - self.cathode_bias_voltage * 0.0025;
        let conducting = (grid_drive - threshold).max(0.0);
        let compressed =
            conducting * self.params.current_gain / (1.0 + conducting * self.params.compression);
        (self.params.idle_current * 0.50 + compressed.tanh() * 0.020) * supply_ratio
    }

    fn update_supply(&mut self, total_current: f32, sag: f32) {
        let effective_current = total_current * (0.18 + sag.clamp(0.0, 1.0) * 1.20);
        let target =
            self.params.nominal_supply_voltage - effective_current * self.params.supply_resistance;
        let coefficient = 1.0
            - (-1.0
                / (self.params.sample_rate
                    * self.params.supply_resistance
                    * self.params.supply_capacitance))
                .exp();
        self.supply_voltage += coefficient * (target - self.supply_voltage);
        self.supply_voltage = self.supply_voltage.clamp(
            self.params.nominal_supply_voltage * 0.45,
            self.params.nominal_supply_voltage,
        );
    }

    fn update_cathode_bias(&mut self, total_current: f32) {
        let target = total_current * self.params.cathode_resistance;
        let coefficient = 1.0
            - (-1.0
                / (self.params.sample_rate
                    * self.params.cathode_resistance
                    * self.params.cathode_capacitance))
                .exp();
        self.cathode_bias_voltage += coefficient * (target - self.cathode_bias_voltage);
    }

    fn supply_ratio(&self) -> f32 {
        (self.supply_voltage / self.params.nominal_supply_voltage).clamp(0.45, 1.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage() -> PushPullEl84Stage {
        PushPullEl84Stage::new(PushPullEl84Params {
            sample_rate: 48_000.0,
            nominal_supply_voltage: 320.0,
            supply_resistance: 360.0,
            supply_capacitance: 32e-6,
            cathode_resistance: 130.0,
            cathode_capacitance: 50e-6,
            idle_current: 0.040,
            drive_gain: 1.58,
            current_gain: 0.92,
            compression: 0.22,
            output_scale: 36.0,
        })
    }

    #[test]
    fn silence_stays_centered_and_finite() {
        let mut stage = stage();
        for _ in 0..2048 {
            let output = stage.process(0.0, 0.5);
            assert!(output.is_finite());
            assert!(output.abs() < 1e-5, "output={output}");
        }
    }

    #[test]
    fn output_is_odd_symmetric_for_small_signal() {
        let mut positive = stage();
        let mut negative = stage();
        for _ in 0..1024 {
            positive.process(0.0, 0.0);
            negative.process(0.0, 0.0);
        }

        let up = positive.process(0.05, 0.0);
        let down = negative.process(-0.05, 0.0);

        assert!((up + down).abs() < up.abs() * 0.08, "up={up}, down={down}");
    }

    #[test]
    fn sustained_drive_drops_supply_voltage() {
        let mut stage = stage();
        let idle_supply = stage.operating_point().supply_voltage;
        for sample_idx in 0..48_000 {
            let input = (std::f32::consts::TAU * 110.0 * sample_idx as f32 / 48_000.0).sin() * 0.7;
            stage.process(input, 1.0);
        }

        let driven_supply = stage.operating_point().supply_voltage;
        assert!(
            driven_supply < idle_supply - 1.0,
            "idle_supply={idle_supply}, driven_supply={driven_supply}"
        );
    }

    #[test]
    fn cathode_bias_recovers_after_overload() {
        let mut stage = stage();
        for sample_idx in 0..12_000 {
            let input = (std::f32::consts::TAU * 110.0 * sample_idx as f32 / 48_000.0).sin() * 1.4;
            stage.process(input, 0.5);
        }
        let overloaded_bias = stage.operating_point().cathode_bias_voltage;

        for _ in 0..48_000 {
            stage.process(0.0, 0.5);
        }
        let recovered_bias = stage.operating_point().cathode_bias_voltage;
        let idle_bias = stage.params.idle_current * stage.params.cathode_resistance;

        assert!(
            (recovered_bias - idle_bias).abs() < (overloaded_bias - idle_bias).abs(),
            "idle_bias={idle_bias}, overloaded_bias={overloaded_bias}, recovered_bias={recovered_bias}"
        );
    }
}
