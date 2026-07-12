#[derive(Clone, Copy)]
pub struct BrightVolumeInputParams {
    pub sample_rate: f32,
    pub input_resistance: f32,
    pub input_coupling_capacitance: f32,
    pub bright_cutoff_hz: f32,
    pub bright_bypass_gain: f32,
}

pub struct BrightVolumeInputStage {
    params: BrightVolumeInputParams,
    input_lowpass: OnePole,
    bright_lowpass: OnePole,
}

#[derive(Clone, Copy)]
pub struct CutPresenceParams {
    pub sample_rate: f32,
    pub min_cutoff_hz: f32,
    pub max_cutoff_hz: f32,
    pub presence_gain: f32,
}

pub struct CutPresenceStage {
    params: CutPresenceParams,
    cut_lowpass: VariableOnePole,
}

impl BrightVolumeInputStage {
    pub fn new(params: BrightVolumeInputParams) -> Self {
        let input_cutoff = 1.0
            / (std::f32::consts::TAU * params.input_resistance * params.input_coupling_capacitance);
        Self {
            params,
            input_lowpass: OnePole::new(params.sample_rate, input_cutoff),
            bright_lowpass: OnePole::new(params.sample_rate, params.bright_cutoff_hz),
        }
    }

    pub fn reset(&mut self) {
        self.input_lowpass.reset();
        self.bright_lowpass.reset();
    }

    pub fn process(&mut self, input: f32, volume: f32) -> f32 {
        let coupled = input - self.input_lowpass.process(input);
        let volume = volume.clamp(0.0, 1.0);
        let volume_gain = volume * volume;
        let bright = coupled - self.bright_lowpass.process(coupled);

        coupled * volume_gain + bright * (1.0 - volume_gain) * self.params.bright_bypass_gain
    }
}

impl CutPresenceStage {
    pub fn new(params: CutPresenceParams) -> Self {
        Self {
            params,
            cut_lowpass: VariableOnePole::new(params.sample_rate, params.max_cutoff_hz),
        }
    }

    pub fn reset(&mut self) {
        self.cut_lowpass.reset();
    }

    pub fn process(&mut self, input: f32, cut: f32, presence: f32) -> f32 {
        let cut = cut.clamp(0.0, 1.0);
        let cutoff_ratio = self.params.max_cutoff_hz / self.params.min_cutoff_hz.max(1.0);
        let cutoff_hz = self.params.max_cutoff_hz / cutoff_ratio.powf(cut);
        self.cut_lowpass
            .set_cutoff(self.params.sample_rate, cutoff_hz);
        let cut_output = self.cut_lowpass.process(input);
        let presence = presence.clamp(0.0, 1.0);

        cut_output + (input - cut_output) * presence * self.params.presence_gain
    }
}

const CLASSIC_TMB_NODES: usize = 6;
const SSS002_HIGH_FILTER_NODES: usize = 3;
const SSS002_LOW_FILTER_NODES: usize = 8;
const SSS002_HIGH_LOW_FILTER_NODES: usize = 11;
const SSS002_DRAWING_HIGH_LOW_NODES: usize = 15;

/// Passive low-plate Fender/Marshall-style three-band tone stack.
///
/// The cell keeps the `38 kOhm` source and `470 kOhm` recovery-grid boundaries
/// explicit. Its component values are a Daybreaker topology hypothesis, not a
/// claim about the external NAM capture or any particular Dumble revision.
pub struct ClassicTmbToneStack {
    treble_capacitor: ClassicTmbCapacitor,
    bass_capacitor: ClassicTmbCapacitor,
    mid_capacitor: ClassicTmbCapacitor,
    inverse_matrix: [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    bass: f32,
    mid: f32,
    treble: f32,
    source_impedance_ohms: f32,
    load_impedance_ohms: f32,
}

impl ClassicTmbToneStack {
    pub const DEFAULT_SOURCE_IMPEDANCE_OHMS: f32 = 38_000.0;
    pub const DEFAULT_LOAD_IMPEDANCE_OHMS: f32 = 470_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            treble_capacitor: ClassicTmbCapacitor::new(250e-12, sample_rate),
            bass_capacitor: ClassicTmbCapacitor::new(22e-9, sample_rate),
            mid_capacitor: ClassicTmbCapacitor::new(22e-9, sample_rate),
            inverse_matrix: [[0.0; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
            bass: f32::NAN,
            mid: f32::NAN,
            treble: f32::NAN,
            source_impedance_ohms: f32::NAN,
            load_impedance_ohms: f32::NAN,
        }
    }

    pub fn reset(&mut self) {
        self.treble_capacitor.reset();
        self.bass_capacitor.reset();
        self.mid_capacitor.reset();
    }

    #[inline]
    pub fn process(&mut self, input: f32, bass: f32, mid: f32, treble: f32) -> f32 {
        self.process_with_boundary(
            input,
            bass,
            mid,
            treble,
            Self::DEFAULT_SOURCE_IMPEDANCE_OHMS,
            Self::DEFAULT_LOAD_IMPEDANCE_OHMS,
        )
    }

    #[inline]
    pub fn process_with_boundary(
        &mut self,
        input: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) -> f32 {
        const INPUT: usize = 0;
        const TREBLE_TOP: usize = 2;
        const TONE: usize = 1;
        const BASS: usize = 4;
        const MID: usize = 5;
        const OUTPUT: usize = 3;

        let source_impedance_ohms = source_impedance_ohms.max(1.0);
        let load_impedance_ohms = load_impedance_ohms.max(1.0);
        if bass != self.bass
            || mid != self.mid
            || treble != self.treble
            || source_impedance_ohms != self.source_impedance_ohms
            || load_impedance_ohms != self.load_impedance_ohms
        {
            self.update_matrix(
                bass,
                mid,
                treble,
                source_impedance_ohms,
                load_impedance_ohms,
            );
        }

        let mut rhs = [0.0; CLASSIC_TMB_NODES];
        rhs[INPUT] = input / source_impedance_ohms;
        self.treble_capacitor.stamp_rhs(&mut rhs, INPUT, TREBLE_TOP);
        self.bass_capacitor.stamp_rhs(&mut rhs, TONE, BASS);
        self.mid_capacitor.stamp_rhs(&mut rhs, TONE, MID);

        let voltages = multiply_classic_tmb(self.inverse_matrix, rhs);
        self.treble_capacitor
            .update(voltages[INPUT], voltages[TREBLE_TOP]);
        self.bass_capacitor.update(voltages[TONE], voltages[BASS]);
        self.mid_capacitor.update(voltages[TONE], voltages[MID]);
        voltages[OUTPUT]
    }

    fn update_matrix(
        &mut self,
        bass: f32,
        mid: f32,
        treble: f32,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) {
        const INPUT: usize = 0;
        const TONE: usize = 1;
        const TREBLE_TOP: usize = 2;
        const OUTPUT: usize = 3;
        const BASS: usize = 4;
        const MID: usize = 5;
        const POT_OHMS: f32 = 1_000_000.0;

        let mut matrix = [[0.0; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES];
        stamp_classic_tmb_to_ground(&mut matrix, INPUT, source_impedance_ohms);
        stamp_classic_tmb(&mut matrix, INPUT, TONE, 100_000.0);
        stamp_classic_tmb_to_ground(&mut matrix, OUTPUT, load_impedance_ohms);

        let treble_taper = audio_taper(treble);
        stamp_classic_tmb(
            &mut matrix,
            TREBLE_TOP,
            OUTPUT,
            pot_segment(POT_OHMS * (1.0 - treble_taper)),
        );
        stamp_classic_tmb_to_ground(&mut matrix, OUTPUT, pot_segment(POT_OHMS * treble_taper));

        let bass_taper = audio_taper(bass);
        stamp_classic_tmb_to_ground(&mut matrix, BASS, pot_segment(POT_OHMS * bass_taper));
        stamp_classic_tmb_to_ground(&mut matrix, MID, 6_800.0 + 18_200.0 * mid_taper(mid));

        self.treble_capacitor
            .stamp_conductance(&mut matrix, INPUT, TREBLE_TOP);
        self.bass_capacitor
            .stamp_conductance(&mut matrix, TONE, BASS);
        self.mid_capacitor.stamp_conductance(&mut matrix, TONE, MID);

        self.inverse_matrix = invert_classic_tmb(matrix);
        self.bass = bass;
        self.mid = mid;
        self.treble = treble;
        self.source_impedance_ohms = source_impedance_ohms;
        self.load_impedance_ohms = load_impedance_ohms;
    }
}

/// SSS #002-style stepped High filter with explicit electrical boundaries.
///
/// This cell models the documented `C37`/`R34` input boundary, the fixed
/// `C44` shunt, and the `R70`/`R71` High switch bank. It intentionally does
/// not infer which switch position was used for an external NAM capture.
/// Callers must keep its source and load impedances explicit so its response
/// remains directly comparable to `daybreaker_sss002_high_low_filters.cir`.
pub struct Sss002HighFilter {
    coupling_capacitor: ClassicTmbCapacitor,
    fixed_shunt_capacitor: ClassicTmbCapacitor,
    bypass_capacitor: ClassicTmbCapacitor,
    inverse_matrix: [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    position: usize,
    source_impedance_ohms: f32,
    load_impedance_ohms: f32,
}

impl Sss002HighFilter {
    pub const DEFAULT_SOURCE_IMPEDANCE_OHMS: f32 = 1_000.0;
    pub const DEFAULT_LOAD_IMPEDANCE_OHMS: f32 = 1_000_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            coupling_capacitor: ClassicTmbCapacitor::new(1e-9, sample_rate),
            fixed_shunt_capacitor: ClassicTmbCapacitor::new(3e-9, sample_rate),
            bypass_capacitor: ClassicTmbCapacitor::new(150e-12, sample_rate),
            inverse_matrix: [[0.0; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
            position: 0,
            source_impedance_ohms: f32::NAN,
            load_impedance_ohms: f32::NAN,
        }
    }

    pub fn reset(&mut self) {
        self.coupling_capacitor.reset();
        self.fixed_shunt_capacitor.reset();
        self.bypass_capacitor.reset();
    }

    #[inline]
    pub fn process(&mut self, input: f32, position: usize) -> f32 {
        self.process_with_boundary(
            input,
            position,
            Self::DEFAULT_SOURCE_IMPEDANCE_OHMS,
            Self::DEFAULT_LOAD_IMPEDANCE_OHMS,
        )
    }

    #[inline]
    pub fn process_with_boundary(
        &mut self,
        input: f32,
        position: usize,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) -> f32 {
        const COUPLED: usize = 0;
        const FILTER_INPUT: usize = 1;
        const OUTPUT: usize = 2;

        let position = position.clamp(1, 7);
        let source_impedance_ohms = source_impedance_ohms.max(1.0);
        let load_impedance_ohms = load_impedance_ohms.max(1.0);
        if position != self.position
            || source_impedance_ohms != self.source_impedance_ohms
            || load_impedance_ohms != self.load_impedance_ohms
        {
            self.update_matrix(position, source_impedance_ohms, load_impedance_ohms);
        }

        let mut rhs = [0.0; SSS002_HIGH_FILTER_NODES];
        rhs[COUPLED] = input / source_impedance_ohms;
        self.coupling_capacitor
            .stamp_rhs_sss002_high(&mut rhs, COUPLED, FILTER_INPUT);
        self.fixed_shunt_capacitor
            .stamp_rhs_to_ground(&mut rhs, FILTER_INPUT);
        if position != 1 {
            self.bypass_capacitor
                .stamp_rhs_sss002_high(&mut rhs, FILTER_INPUT, OUTPUT);
        }

        let voltages = multiply_sss002_high(self.inverse_matrix, rhs);
        self.coupling_capacitor
            .update(voltages[COUPLED], voltages[FILTER_INPUT]);
        self.fixed_shunt_capacitor
            .update_to_ground(voltages[FILTER_INPUT]);
        if position != 1 {
            self.bypass_capacitor
                .update(voltages[FILTER_INPUT], voltages[OUTPUT]);
        }
        voltages[OUTPUT]
    }

    fn update_matrix(
        &mut self,
        position: usize,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) {
        const COUPLED: usize = 0;
        const FILTER_INPUT: usize = 1;
        const OUTPUT: usize = 2;

        let mut matrix = [[0.0; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES];
        stamp_sss002_high_to_ground(&mut matrix, COUPLED, source_impedance_ohms);
        stamp_sss002_high_to_ground(&mut matrix, FILTER_INPUT, 470_000.0);
        stamp_sss002_high(&mut matrix, FILTER_INPUT, OUTPUT, 820_000.0);
        stamp_sss002_high_to_ground(&mut matrix, OUTPUT, 100_000.0);
        stamp_sss002_high_to_ground(&mut matrix, OUTPUT, load_impedance_ohms);
        self.coupling_capacitor
            .stamp_conductance_sss002_high(&mut matrix, COUPLED, FILTER_INPUT);
        self.fixed_shunt_capacitor
            .stamp_conductance_to_ground_sss002_high(&mut matrix, FILTER_INPUT);

        if position == 1 {
            // 1 Ohm is a numerically well-conditioned closed-contact
            // approximation and matches the SPICE fixture.
            stamp_sss002_high(&mut matrix, FILTER_INPUT, OUTPUT, 1.0);
        } else {
            self.bypass_capacitor = ClassicTmbCapacitor::new(
                sss002_high_capacitance(position),
                self.bypass_capacitor.sample_rate(),
            );
            self.bypass_capacitor
                .stamp_conductance_sss002_high(&mut matrix, FILTER_INPUT, OUTPUT);
        }

        self.inverse_matrix = invert_sss002_high(matrix);
        self.position = position;
        self.source_impedance_ohms = source_impedance_ohms;
        self.load_impedance_ohms = load_impedance_ohms;
    }
}

fn sss002_high_capacitance(position: usize) -> f32 {
    match position {
        2 => 150e-12,
        3 => 330e-12,
        4 => 1e-9,
        5 => 2.4e-9,
        6 => 5.1e-9,
        7 => 10e-9,
        _ => unreachable!("position is clamped before selecting the SSS #002 High capacitor"),
    }
}

/// SSS #002-style stepped Low filter with its selected resistor-ladder tap.
///
/// `R79` is retained as the source resistor, positions 1 through 7 select the
/// successive `R72..R78` ladder taps, and `C45` bridges the ladder endpoints.
/// The switch position is intentionally an explicit caller choice: it is not
/// inferred from any external NAM capture.
pub struct Sss002LowFilter {
    bridge_capacitor: ClassicTmbCapacitor,
    inverse_matrix: [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    position: usize,
    load_impedance_ohms: f32,
}

impl Sss002LowFilter {
    pub const DEFAULT_LOAD_IMPEDANCE_OHMS: f32 = 1_000_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            bridge_capacitor: ClassicTmbCapacitor::new(10e-9, sample_rate),
            inverse_matrix: [[0.0; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
            position: 0,
            load_impedance_ohms: f32::NAN,
        }
    }

    pub fn reset(&mut self) {
        self.bridge_capacitor.reset();
    }

    #[inline]
    pub fn process(&mut self, input: f32, position: usize) -> f32 {
        self.process_with_load(input, position, Self::DEFAULT_LOAD_IMPEDANCE_OHMS)
    }

    #[inline]
    pub fn process_with_load(
        &mut self,
        input: f32,
        position: usize,
        load_impedance_ohms: f32,
    ) -> f32 {
        const OUTPUT: usize = 0;
        const LADDER_TOP: usize = 1;
        const LADDER_BOTTOM: usize = 7;

        let position = position.clamp(1, 7);
        let load_impedance_ohms = load_impedance_ohms.max(1.0);
        if position != self.position || load_impedance_ohms != self.load_impedance_ohms {
            self.update_matrix(position, load_impedance_ohms);
        }

        let mut rhs = [0.0; SSS002_LOW_FILTER_NODES];
        rhs[OUTPUT] = input / 270_000.0;
        self.bridge_capacitor
            .stamp_rhs_sss002_low(&mut rhs, LADDER_TOP, LADDER_BOTTOM);
        let voltages = multiply_sss002_low(self.inverse_matrix, rhs);
        self.bridge_capacitor
            .update(voltages[LADDER_TOP], voltages[LADDER_BOTTOM]);
        voltages[OUTPUT]
    }

    fn update_matrix(&mut self, position: usize, load_impedance_ohms: f32) {
        const OUTPUT: usize = 0;
        const LADDER_TOP: usize = 1;
        const LADDER_BOTTOM: usize = 7;

        let mut matrix = [[0.0; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES];
        stamp_sss002_low_to_ground(&mut matrix, OUTPUT, 270_000.0);
        stamp_sss002_low_to_ground(&mut matrix, OUTPUT, load_impedance_ohms);
        stamp_sss002_low(&mut matrix, OUTPUT, position, 1.0);

        let ladder_resistors = [
            39_000.0, 68_000.0, 100_000.0, 180_000.0, 270_000.0, 390_000.0,
        ];
        for (index, resistance) in ladder_resistors.into_iter().enumerate() {
            stamp_sss002_low(
                &mut matrix,
                LADDER_TOP + index,
                LADDER_TOP + index + 1,
                resistance,
            );
        }
        stamp_sss002_low_to_ground(&mut matrix, LADDER_BOTTOM, 12_000.0);
        self.bridge_capacitor
            .stamp_conductance_sss002_low(&mut matrix, LADDER_TOP, LADDER_BOTTOM);

        self.inverse_matrix = invert_sss002_low(matrix);
        self.position = position;
        self.load_impedance_ohms = load_impedance_ohms;
    }
}

/// Full SSS #002-style High/Low passive chain.
///
/// Unlike [`Sss002LowFilter`], which is retained only for isolated ladder
/// measurement, this cell follows the ASC routing: the High output remains the
/// audio output while `R79` feeds the selected Low ladder as a shunt load.
/// Its source/load values are explicit fixture boundaries, not NAM settings.
pub struct Sss002HighLowFilter {
    coupling_capacitor: ClassicTmbCapacitor,
    fixed_shunt_capacitor: ClassicTmbCapacitor,
    high_bypass_capacitor: ClassicTmbCapacitor,
    low_bridge_capacitor: ClassicTmbCapacitor,
    inverse_matrix: [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    high_position: usize,
    low_position: usize,
    source_impedance_ohms: f32,
    load_impedance_ohms: f32,
}

impl Sss002HighLowFilter {
    /// R69's documented 68 kOhm plate-load boundary. The effective impedance
    /// of the complete triode stage must still be established before runtime
    /// integration.
    pub const DEFAULT_SOURCE_IMPEDANCE_OHMS: f32 = 68_000.0;
    pub const DEFAULT_LOAD_IMPEDANCE_OHMS: f32 = 1_000_000.0;

    pub fn new(sample_rate: f32) -> Self {
        Self {
            coupling_capacitor: ClassicTmbCapacitor::new(1e-9, sample_rate),
            fixed_shunt_capacitor: ClassicTmbCapacitor::new(3e-9, sample_rate),
            high_bypass_capacitor: ClassicTmbCapacitor::new(150e-12, sample_rate),
            low_bridge_capacitor: ClassicTmbCapacitor::new(10e-9, sample_rate),
            inverse_matrix: [[0.0; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
            high_position: 0,
            low_position: 0,
            source_impedance_ohms: f32::NAN,
            load_impedance_ohms: f32::NAN,
        }
    }

    pub fn reset(&mut self) {
        self.coupling_capacitor.reset();
        self.fixed_shunt_capacitor.reset();
        self.high_bypass_capacitor.reset();
        self.low_bridge_capacitor.reset();
    }

    #[inline]
    pub fn process(&mut self, input: f32, high_position: usize, low_position: usize) -> f32 {
        self.process_with_boundary(
            input,
            high_position,
            low_position,
            Self::DEFAULT_SOURCE_IMPEDANCE_OHMS,
            Self::DEFAULT_LOAD_IMPEDANCE_OHMS,
        )
    }

    #[inline]
    pub fn process_with_boundary(
        &mut self,
        input: f32,
        high_position: usize,
        low_position: usize,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) -> f32 {
        const COUPLED: usize = 0;
        const HIGH_INPUT: usize = 1;
        const OUTPUT: usize = 2;
        const LOW_TOP: usize = 4;
        const LOW_BOTTOM: usize = 10;

        let high_position = high_position.clamp(1, 7);
        let low_position = low_position.clamp(1, 7);
        let source_impedance_ohms = source_impedance_ohms.max(1.0);
        let load_impedance_ohms = load_impedance_ohms.max(1.0);
        if high_position != self.high_position
            || low_position != self.low_position
            || source_impedance_ohms != self.source_impedance_ohms
            || load_impedance_ohms != self.load_impedance_ohms
        {
            self.update_matrix(
                high_position,
                low_position,
                source_impedance_ohms,
                load_impedance_ohms,
            );
        }

        let mut rhs = [0.0; SSS002_HIGH_LOW_FILTER_NODES];
        rhs[COUPLED] = input / source_impedance_ohms;
        self.coupling_capacitor
            .stamp_rhs_sss002_high_low(&mut rhs, COUPLED, HIGH_INPUT);
        self.fixed_shunt_capacitor
            .stamp_rhs_to_ground_sss002_high_low(&mut rhs, HIGH_INPUT);
        if high_position != 1 {
            self.high_bypass_capacitor
                .stamp_rhs_sss002_high_low(&mut rhs, HIGH_INPUT, OUTPUT);
        }
        self.low_bridge_capacitor
            .stamp_rhs_sss002_high_low(&mut rhs, LOW_TOP, LOW_BOTTOM);

        let voltages = multiply_sss002_high_low(self.inverse_matrix, rhs);
        self.coupling_capacitor
            .update(voltages[COUPLED], voltages[HIGH_INPUT]);
        self.fixed_shunt_capacitor
            .update_to_ground(voltages[HIGH_INPUT]);
        if high_position != 1 {
            self.high_bypass_capacitor
                .update(voltages[HIGH_INPUT], voltages[OUTPUT]);
        }
        self.low_bridge_capacitor
            .update(voltages[LOW_TOP], voltages[LOW_BOTTOM]);
        voltages[OUTPUT]
    }

    fn update_matrix(
        &mut self,
        high_position: usize,
        low_position: usize,
        source_impedance_ohms: f32,
        load_impedance_ohms: f32,
    ) {
        const COUPLED: usize = 0;
        const HIGH_INPUT: usize = 1;
        const OUTPUT: usize = 2;
        const LOW_COMMON: usize = 3;
        const LOW_TOP: usize = 4;
        const LOW_BOTTOM: usize = 10;

        let mut matrix = [[0.0; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES];
        stamp_sss002_high_low_to_ground(&mut matrix, COUPLED, source_impedance_ohms);
        stamp_sss002_high_low_to_ground(&mut matrix, HIGH_INPUT, 470_000.0);
        stamp_sss002_high_low(&mut matrix, HIGH_INPUT, OUTPUT, 820_000.0);
        stamp_sss002_high_low_to_ground(&mut matrix, OUTPUT, 100_000.0);
        stamp_sss002_high_low_to_ground(&mut matrix, OUTPUT, load_impedance_ohms);
        stamp_sss002_high_low(&mut matrix, OUTPUT, LOW_COMMON, 270_000.0);
        stamp_sss002_high_low(&mut matrix, LOW_COMMON, LOW_TOP + low_position - 1, 1.0);

        let ladder_resistors = [
            39_000.0, 68_000.0, 100_000.0, 180_000.0, 270_000.0, 390_000.0,
        ];
        for (index, resistance) in ladder_resistors.into_iter().enumerate() {
            stamp_sss002_high_low(
                &mut matrix,
                LOW_TOP + index,
                LOW_TOP + index + 1,
                resistance,
            );
        }
        stamp_sss002_high_low_to_ground(&mut matrix, LOW_BOTTOM, 12_000.0);
        self.coupling_capacitor
            .stamp_conductance_sss002_high_low(&mut matrix, COUPLED, HIGH_INPUT);
        self.fixed_shunt_capacitor
            .stamp_conductance_to_ground_sss002_high_low(&mut matrix, HIGH_INPUT);
        self.low_bridge_capacitor.stamp_conductance_sss002_high_low(
            &mut matrix,
            LOW_TOP,
            LOW_BOTTOM,
        );
        if high_position == 1 {
            stamp_sss002_high_low(&mut matrix, HIGH_INPUT, OUTPUT, 1.0);
        } else {
            self.high_bypass_capacitor = ClassicTmbCapacitor::new(
                sss002_high_capacitance(high_position),
                self.high_bypass_capacitor.sample_rate(),
            );
            self.high_bypass_capacitor
                .stamp_conductance_sss002_high_low(&mut matrix, HIGH_INPUT, OUTPUT);
        }

        self.inverse_matrix = invert_sss002_high_low(matrix);
        self.high_position = high_position;
        self.low_position = low_position;
        self.source_impedance_ohms = source_impedance_ohms;
        self.load_impedance_ohms = load_impedance_ohms;
    }
}

/// Drawing-default SSS #002 High/Low network including the U4 plate-side
/// C6/R53/L2/R34 branch. This is the integration reference; the older
/// `Sss002HighLowFilter` remains only as a reduced downstream bench cell.
pub struct Sss002DrawingHighLowFilter {
    c6: ClassicTmbCapacitor,
    c37: ClassicTmbCapacitor,
    c44: ClassicTmbCapacitor,
    c45: ClassicTmbCapacitor,
    l2: DrawingInductor,
    inverse_matrix: [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
}

impl Sss002DrawingHighLowFilter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            c6: ClassicTmbCapacitor::new(10e-9, sample_rate),
            c37: ClassicTmbCapacitor::new(1e-9, sample_rate),
            c44: ClassicTmbCapacitor::new(3e-9, sample_rate),
            c45: ClassicTmbCapacitor::new(10e-9, sample_rate),
            l2: DrawingInductor::new(300e-3, sample_rate),
            inverse_matrix: [[0.0; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
        };
        filter.update_matrix();
        filter
    }

    pub fn reset(&mut self) {
        self.c6.reset();
        self.c37.reset();
        self.c44.reset();
        self.c45.reset();
        self.l2.reset();
    }

    #[inline]
    pub fn process(&mut self, input_v: f32) -> f32 {
        const U4_PLATE: usize = 0;
        const PRE_FILTER: usize = 1;
        const HIGH_INPUT: usize = 2;
        const L2_SERIES: usize = 4;
        const L2_OUT: usize = 5;
        const LOW_TOP: usize = 8;
        const LOW_BOTTOM: usize = 14;

        let mut rhs = [0.0; SSS002_DRAWING_HIGH_LOW_NODES];
        rhs[U4_PLATE] = input_v / 100_000.0;
        self.c6
            .stamp_rhs_sss002_drawing(&mut rhs, U4_PLATE, PRE_FILTER);
        self.c37
            .stamp_rhs_sss002_drawing(&mut rhs, PRE_FILTER, HIGH_INPUT);
        self.c44
            .stamp_rhs_to_ground_sss002_drawing(&mut rhs, HIGH_INPUT);
        self.c45
            .stamp_rhs_sss002_drawing(&mut rhs, LOW_TOP, LOW_BOTTOM);
        self.l2.stamp_rhs(&mut rhs, L2_SERIES, L2_OUT);

        let voltages = multiply_sss002_drawing(self.inverse_matrix, rhs);
        self.c6.update(voltages[U4_PLATE], voltages[PRE_FILTER]);
        self.c37.update(voltages[PRE_FILTER], voltages[HIGH_INPUT]);
        self.c44.update_to_ground(voltages[HIGH_INPUT]);
        self.c45.update(voltages[LOW_TOP], voltages[LOW_BOTTOM]);
        self.l2.update(voltages[L2_SERIES], voltages[L2_OUT]);
        voltages[6]
    }

    fn update_matrix(&mut self) {
        const U4_PLATE: usize = 0;
        const PRE_FILTER: usize = 1;
        const HIGH_INPUT: usize = 2;
        const R53_OUT: usize = 3;
        const L2_SERIES: usize = 4;
        const L2_OUT: usize = 5;
        const OUTPUT: usize = 6;
        const LOW_COMMON: usize = 7;
        const LOW_TOP: usize = 8;
        const LOW_BOTTOM: usize = 14;

        let mut matrix = [[0.0; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES];
        stamp_sss002_drawing_to_ground(&mut matrix, U4_PLATE, 100_000.0);
        stamp_sss002_drawing(&mut matrix, PRE_FILTER, R53_OUT, 100_000.0);
        stamp_sss002_drawing(&mut matrix, R53_OUT, L2_SERIES, 59.0);
        stamp_sss002_drawing(&mut matrix, L2_OUT, HIGH_INPUT, 470_000.0);
        stamp_sss002_drawing(&mut matrix, HIGH_INPUT, OUTPUT, 820_000.0);
        stamp_sss002_drawing(&mut matrix, HIGH_INPUT, OUTPUT, 1.0);
        stamp_sss002_drawing_to_ground(&mut matrix, OUTPUT, 100_000.0);
        stamp_sss002_drawing_to_ground(&mut matrix, OUTPUT, 1_000_000.0);
        stamp_sss002_drawing(&mut matrix, OUTPUT, LOW_COMMON, 270_000.0);
        stamp_sss002_drawing(&mut matrix, LOW_COMMON, LOW_TOP, 1.0);
        for (index, resistance) in [
            39_000.0, 68_000.0, 100_000.0, 180_000.0, 270_000.0, 390_000.0,
        ]
        .into_iter()
        .enumerate()
        {
            stamp_sss002_drawing(
                &mut matrix,
                LOW_TOP + index,
                LOW_TOP + index + 1,
                resistance,
            );
        }
        stamp_sss002_drawing_to_ground(&mut matrix, LOW_BOTTOM, 12_000.0);
        self.c6
            .stamp_conductance_sss002_drawing(&mut matrix, U4_PLATE, PRE_FILTER);
        self.c37
            .stamp_conductance_sss002_drawing(&mut matrix, PRE_FILTER, HIGH_INPUT);
        self.c44
            .stamp_conductance_to_ground_sss002_drawing(&mut matrix, HIGH_INPUT);
        self.c45
            .stamp_conductance_sss002_drawing(&mut matrix, LOW_TOP, LOW_BOTTOM);
        self.l2.stamp_conductance(&mut matrix, L2_SERIES, L2_OUT);
        self.inverse_matrix = invert_sss002_drawing(matrix);
    }
}

/// The SSS #002 U5 audio-taper volume control at a zero-ohm source boundary.
///
/// The source drawing declares `pot_pow`, `Rtot = 1 Mohm`, `Rtap = 100 kOhm`,
/// and `tap = 0.5`. Its wiper drives R69/U4. The returned voltage is therefore
/// valid only while the upstream source is modeled as ideal; the exposed leg
/// resistances let a later joined tone-stack MNA solve the actual loading.
pub struct Sss002VolumeControl;

impl Sss002VolumeControl {
    const TOTAL_RESISTANCE_OHMS: f32 = 1_000_000.0;
    const TAP_RESISTANCE_OHMS: f32 = 100_000.0;
    const TAP_POSITION: f32 = 0.5;

    #[inline]
    pub fn new() -> Self {
        Self
    }

    /// Returns `(source_to_wiper, wiper_to_ground)` in ohms.
    #[inline]
    pub fn leg_resistances(&self, travel: f32) -> (f32, f32) {
        let ratio = Self::wiper_ratio(travel);
        (
            Self::TOTAL_RESISTANCE_OHMS * (1.0 - ratio),
            Self::TOTAL_RESISTANCE_OHMS * ratio,
        )
    }

    /// Applies U5 with an ideal voltage source at its top terminal.
    #[inline]
    pub fn process_ideal_source(&self, input_v: f32, travel: f32) -> f32 {
        input_v * Self::wiper_ratio(travel)
    }

    #[inline]
    fn wiper_ratio(travel: f32) -> f32 {
        // This is the exact `pot_pow` law in potentiometer_standard.lib.
        let travel = travel.clamp(0.000_01, 0.999_99);
        let exponent = (Self::TAP_RESISTANCE_OHMS / Self::TOTAL_RESISTANCE_OHMS).ln()
            / Self::TAP_POSITION.ln();
        travel.powf(exponent)
    }
}

struct DrawingInductor {
    conductance: f32,
    previous_voltage: f32,
    previous_current: f32,
}

impl DrawingInductor {
    fn new(inductance_h: f32, sample_rate: f32) -> Self {
        Self {
            conductance: 1.0 / (2.0 * inductance_h * sample_rate),
            previous_voltage: 0.0,
            previous_current: 0.0,
        }
    }

    fn reset(&mut self) {
        self.previous_voltage = 0.0;
        self.previous_current = 0.0;
    }

    fn stamp_conductance(
        &self,
        matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_sss002_drawing_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_rhs(&self, rhs: &mut [f32; SSS002_DRAWING_HIGH_LOW_NODES], a: usize, b: usize) {
        let history_current = self.previous_current + self.conductance * self.previous_voltage;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn update(&mut self, a: f32, b: f32) {
        let voltage = a - b;
        self.previous_current += self.conductance * (voltage + self.previous_voltage);
        self.previous_voltage = voltage;
    }
}

/// The drawn U37 recovery stage after the SSS #002-style High/Low filter.
///
/// This cell accepts and emits physical volts. Its passive source boundary,
/// `R80 = 100 kOhm` plate, `R81/C46 = 1 kOhm / 1 uF` cathode network, and
/// C47/470 kOhm output boundary mirror the matching SPICE fixture. It is kept
/// outside the normalized Daybreaker model until the DI-to-voltage mapping is
/// established.
pub struct Sss002HighLowU37RecoveryStage {
    filter: Sss002DrawingHighLowFilter,
    recovery: CommonCathodeStage,
    output_coupling: OutputAcCoupling,
}

impl Sss002HighLowU37RecoveryStage {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            filter: Sss002DrawingHighLowFilter::new(sample_rate),
            recovery: CommonCathodeStage::new(CommonCathodeParams {
                sample_rate,
                // R71 in the preceding High network is the actual 100 kOhm
                // grid return. Avoid stamping a second grid load in the stage.
                grid_leak_resistance: 1.0e9,
                // C37 already provides the physical coupling boundary.
                input_coupling_capacitance: 22e-6,
                plate_resistance: 100_000.0,
                cathode_resistance: 1_000.0,
                cathode_bypass_capacitance: Some(1e-6),
                supply_resistance: 1.0,
                supply_capacitance: 22e-6,
                nominal_supply_voltage: 300.0,
                input_gain: 1.0,
                output_scale: 1.0,
                // U37 is a 7025 in the source drawing, not a generic ECC83.
                triode: TriodeParams::TUBE_7025,
            }),
            // C47 = 0.22 uF sees the 100 kOhm plate source and 470 kOhm
            // following grid load; the load divider is applied below.
            output_coupling: OutputAcCoupling::new(sample_rate, 570_000.0, 0.22e-6),
        }
    }

    pub fn reset(&mut self) {
        self.filter.reset();
        self.recovery.reset();
        self.output_coupling.reset();
    }

    #[inline]
    pub fn process(&mut self, input_v: f32) -> f32 {
        let filter_output = self.filter.process(input_v);
        let plate_ac = self.recovery.process(filter_output);
        self.output_coupling
            .process(plate_ac * (470_000.0 / 570_000.0))
    }

    pub fn operating_point(&self) -> crate::circuit::triode::CommonCathodeOperatingPoint {
        self.recovery.operating_point()
    }
}

/// Passive classic TMB followed by its loaded ECC83 recovery stage.
///
/// Signals use physical volts at the tone-stack source and recovery plate
/// boundary. Model-specific normalized-voltage conversion belongs outside this
/// cell so it remains comparable to the matching SPICE fixture.
pub struct ClassicTmbRecoveryStage {
    tone_stack: ClassicTmbToneStack,
    recovery: CommonCathodeStage,
    output_coupling: OutputAcCoupling,
}

impl ClassicTmbRecoveryStage {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            tone_stack: ClassicTmbToneStack::new(sample_rate),
            recovery: CommonCathodeStage::new(CommonCathodeParams {
                sample_rate,
                grid_leak_resistance: 470_000.0,
                input_coupling_capacitance: 22e-9,
                plate_resistance: 100_000.0,
                cathode_resistance: 2_200.0,
                cathode_bypass_capacitance: Some(1e-6),
                supply_resistance: 12_000.0,
                supply_capacitance: 22e-6,
                nominal_supply_voltage: 280.0,
                input_gain: 1.0,
                output_scale: 1.0,
                triode: TriodeParams::ECC83,
            }),
            // 22 nF plate coupling capacitor driving the following 1 MOhm
            // grid load through the recovery stage's 100 kOhm plate source.
            output_coupling: OutputAcCoupling::new(sample_rate, 1_100_000.0, 22e-9),
        }
    }

    pub fn reset(&mut self) {
        self.tone_stack.reset();
        self.recovery.reset();
        self.output_coupling.reset();
    }

    #[inline]
    pub fn process(&mut self, input_v: f32, bass: f32, mid: f32, treble: f32) -> f32 {
        let plate = self
            .recovery
            .process(self.tone_stack.process(input_v, bass, mid, treble));
        // The 1 MOhm load appears in parallel with the 100 kOhm plate resistor
        // at audio frequencies. The following high-pass removes the plate DC
        // operating point before it crosses the stage boundary.
        self.output_coupling
            .process(plate * (1_000_000.0 / 1_100_000.0))
    }

    pub fn operating_point(&self) -> crate::circuit::triode::CommonCathodeOperatingPoint {
        self.recovery.operating_point()
    }
}

struct OutputAcCoupling {
    coefficient: f32,
    lowpass_state: f32,
}

impl OutputAcCoupling {
    fn new(sample_rate: f32, resistance: f32, capacitance: f32) -> Self {
        let cutoff = 1.0 / (std::f32::consts::TAU * resistance * capacitance);
        Self {
            coefficient: 1.0 - (-std::f32::consts::TAU * cutoff / sample_rate).exp(),
            lowpass_state: 0.0,
        }
    }

    fn reset(&mut self) {
        self.lowpass_state = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        self.lowpass_state += self.coefficient * (input - self.lowpass_state);
        input - self.lowpass_state
    }
}

struct ClassicTmbCapacitor {
    conductance: f32,
    sample_rate: f32,
    previous_voltage: f32,
    previous_current: f32,
}

impl ClassicTmbCapacitor {
    fn new(capacitance: f32, sample_rate: f32) -> Self {
        Self {
            conductance: 2.0 * capacitance * sample_rate,
            sample_rate,
            previous_voltage: 0.0,
            previous_current: 0.0,
        }
    }

    fn reset(&mut self) {
        self.previous_voltage = 0.0;
        self.previous_current = 0.0;
    }

    fn stamp_conductance(
        &self,
        matrix: &mut [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_classic_tmb_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_conductance_sss002_high(
        &self,
        matrix: &mut [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_sss002_high_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_conductance_to_ground_sss002_high(
        &self,
        matrix: &mut [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
        node: usize,
    ) {
        matrix[node][node] += self.conductance;
    }

    fn stamp_conductance_sss002_low(
        &self,
        matrix: &mut [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_sss002_low_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_conductance_sss002_high_low(
        &self,
        matrix: &mut [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_sss002_high_low_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_conductance_to_ground_sss002_high_low(
        &self,
        matrix: &mut [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
        node: usize,
    ) {
        matrix[node][node] += self.conductance;
    }

    fn stamp_conductance_sss002_drawing(
        &self,
        matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
        a: usize,
        b: usize,
    ) {
        stamp_sss002_drawing_conductance(matrix, a, b, self.conductance);
    }

    fn stamp_conductance_to_ground_sss002_drawing(
        &self,
        matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
        node: usize,
    ) {
        matrix[node][node] += self.conductance;
    }

    fn stamp_rhs(&self, rhs: &mut [f32; CLASSIC_TMB_NODES], a: usize, b: usize) {
        let history_current = -self.conductance * self.previous_voltage - self.previous_current;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn stamp_rhs_sss002_high(&self, rhs: &mut [f32; SSS002_HIGH_FILTER_NODES], a: usize, b: usize) {
        let history_current = -self.conductance * self.previous_voltage - self.previous_current;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn stamp_rhs_sss002_low(&self, rhs: &mut [f32; SSS002_LOW_FILTER_NODES], a: usize, b: usize) {
        let history_current = -self.conductance * self.previous_voltage - self.previous_current;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn stamp_rhs_sss002_high_low(
        &self,
        rhs: &mut [f32; SSS002_HIGH_LOW_FILTER_NODES],
        a: usize,
        b: usize,
    ) {
        let history_current = -self.conductance * self.previous_voltage - self.previous_current;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn stamp_rhs_sss002_drawing(
        &self,
        rhs: &mut [f32; SSS002_DRAWING_HIGH_LOW_NODES],
        a: usize,
        b: usize,
    ) {
        let history_current = -self.conductance * self.previous_voltage - self.previous_current;
        rhs[a] -= history_current;
        rhs[b] += history_current;
    }

    fn stamp_rhs_to_ground(&self, rhs: &mut [f32; SSS002_HIGH_FILTER_NODES], node: usize) {
        rhs[node] -= -self.conductance * self.previous_voltage - self.previous_current;
    }

    fn stamp_rhs_to_ground_sss002_high_low(
        &self,
        rhs: &mut [f32; SSS002_HIGH_LOW_FILTER_NODES],
        node: usize,
    ) {
        rhs[node] -= -self.conductance * self.previous_voltage - self.previous_current;
    }

    fn stamp_rhs_to_ground_sss002_drawing(
        &self,
        rhs: &mut [f32; SSS002_DRAWING_HIGH_LOW_NODES],
        node: usize,
    ) {
        rhs[node] -= -self.conductance * self.previous_voltage - self.previous_current;
    }

    fn update(&mut self, a: f32, b: f32) {
        let voltage = a - b;
        self.previous_current =
            self.conductance * (voltage - self.previous_voltage) - self.previous_current;
        self.previous_voltage = voltage;
    }

    fn update_to_ground(&mut self, voltage: f32) {
        self.previous_current =
            self.conductance * (voltage - self.previous_voltage) - self.previous_current;
        self.previous_voltage = voltage;
    }

    fn sample_rate(&self) -> f32 {
        self.sample_rate
    }
}

fn audio_taper(position: f32) -> f32 {
    10.0_f32.powf(2.0 * position.clamp(0.0, 1.0) - 2.0)
}

fn mid_taper(position: f32) -> f32 {
    audio_taper(position)
}

fn pot_segment(resistance: f32) -> f32 {
    resistance.max(1.0)
}

fn stamp_classic_tmb(
    matrix: &mut [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_classic_tmb_conductance(matrix, a, b, 1.0 / resistance);
}

fn stamp_classic_tmb_conductance(
    matrix: &mut [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn stamp_classic_tmb_to_ground(
    matrix: &mut [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    node: usize,
    resistance: f32,
) {
    matrix[node][node] += 1.0 / resistance;
}

fn solve_classic_tmb(
    mut matrix: [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    mut rhs: [f32; CLASSIC_TMB_NODES],
) -> [f32; CLASSIC_TMB_NODES] {
    for pivot in 0..CLASSIC_TMB_NODES {
        let mut best_row = pivot;
        for row in pivot + 1..CLASSIC_TMB_NODES {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs() {
                best_row = row;
            }
        }
        if best_row != pivot {
            matrix.swap(pivot, best_row);
            rhs.swap(pivot, best_row);
        }

        let inverse_pivot = 1.0 / matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value *= inverse_pivot;
        }
        rhs[pivot] *= inverse_pivot;
        let pivot_row = matrix[pivot];
        for row in 0..CLASSIC_TMB_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_row[pivot..]) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn invert_classic_tmb(
    matrix: [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
) -> [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES] {
    let mut inverse = [[0.0; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES];
    for column in 0..CLASSIC_TMB_NODES {
        let mut basis = [0.0; CLASSIC_TMB_NODES];
        basis[column] = 1.0;
        let solution = solve_classic_tmb(matrix, basis);
        for (row, value) in solution.into_iter().enumerate() {
            inverse[row][column] = value;
        }
    }
    inverse
}

fn multiply_classic_tmb(
    matrix: [[f32; CLASSIC_TMB_NODES]; CLASSIC_TMB_NODES],
    vector: [f32; CLASSIC_TMB_NODES],
) -> [f32; CLASSIC_TMB_NODES] {
    matrix.map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

fn stamp_sss002_high(
    matrix: &mut [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_sss002_high_conductance(matrix, a, b, 1.0 / resistance);
}

fn stamp_sss002_high_conductance(
    matrix: &mut [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn stamp_sss002_high_to_ground(
    matrix: &mut [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    node: usize,
    resistance: f32,
) {
    matrix[node][node] += 1.0 / resistance;
}

fn solve_sss002_high(
    mut matrix: [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    mut rhs: [f32; SSS002_HIGH_FILTER_NODES],
) -> [f32; SSS002_HIGH_FILTER_NODES] {
    for pivot in 0..SSS002_HIGH_FILTER_NODES {
        let mut best_row = pivot;
        for row in pivot + 1..SSS002_HIGH_FILTER_NODES {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs() {
                best_row = row;
            }
        }
        if best_row != pivot {
            matrix.swap(pivot, best_row);
            rhs.swap(pivot, best_row);
        }

        let inverse_pivot = 1.0 / matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value *= inverse_pivot;
        }
        rhs[pivot] *= inverse_pivot;
        let pivot_row = matrix[pivot];
        for row in 0..SSS002_HIGH_FILTER_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_row[pivot..]) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn invert_sss002_high(
    matrix: [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
) -> [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES] {
    let mut inverse = [[0.0; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES];
    for column in 0..SSS002_HIGH_FILTER_NODES {
        let mut basis = [0.0; SSS002_HIGH_FILTER_NODES];
        basis[column] = 1.0;
        let solution = solve_sss002_high(matrix, basis);
        for (row, value) in solution.into_iter().enumerate() {
            inverse[row][column] = value;
        }
    }
    inverse
}

fn multiply_sss002_high(
    matrix: [[f32; SSS002_HIGH_FILTER_NODES]; SSS002_HIGH_FILTER_NODES],
    vector: [f32; SSS002_HIGH_FILTER_NODES],
) -> [f32; SSS002_HIGH_FILTER_NODES] {
    matrix.map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

fn stamp_sss002_low(
    matrix: &mut [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_sss002_low_conductance(matrix, a, b, 1.0 / resistance);
}

fn stamp_sss002_low_conductance(
    matrix: &mut [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn stamp_sss002_low_to_ground(
    matrix: &mut [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    node: usize,
    resistance: f32,
) {
    matrix[node][node] += 1.0 / resistance;
}

fn solve_sss002_low(
    mut matrix: [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    mut rhs: [f32; SSS002_LOW_FILTER_NODES],
) -> [f32; SSS002_LOW_FILTER_NODES] {
    for pivot in 0..SSS002_LOW_FILTER_NODES {
        let mut best_row = pivot;
        for row in pivot + 1..SSS002_LOW_FILTER_NODES {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs() {
                best_row = row;
            }
        }
        if best_row != pivot {
            matrix.swap(pivot, best_row);
            rhs.swap(pivot, best_row);
        }

        let inverse_pivot = 1.0 / matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value *= inverse_pivot;
        }
        rhs[pivot] *= inverse_pivot;
        let pivot_row = matrix[pivot];
        for row in 0..SSS002_LOW_FILTER_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_row[pivot..]) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn invert_sss002_low(
    matrix: [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
) -> [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES] {
    let mut inverse = [[0.0; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES];
    for column in 0..SSS002_LOW_FILTER_NODES {
        let mut basis = [0.0; SSS002_LOW_FILTER_NODES];
        basis[column] = 1.0;
        let solution = solve_sss002_low(matrix, basis);
        for (row, value) in solution.into_iter().enumerate() {
            inverse[row][column] = value;
        }
    }
    inverse
}

fn multiply_sss002_low(
    matrix: [[f32; SSS002_LOW_FILTER_NODES]; SSS002_LOW_FILTER_NODES],
    vector: [f32; SSS002_LOW_FILTER_NODES],
) -> [f32; SSS002_LOW_FILTER_NODES] {
    matrix.map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

fn stamp_sss002_high_low(
    matrix: &mut [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_sss002_high_low_conductance(matrix, a, b, 1.0 / resistance);
}

fn stamp_sss002_high_low_conductance(
    matrix: &mut [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn stamp_sss002_high_low_to_ground(
    matrix: &mut [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    node: usize,
    resistance: f32,
) {
    matrix[node][node] += 1.0 / resistance;
}

fn solve_sss002_high_low(
    mut matrix: [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    mut rhs: [f32; SSS002_HIGH_LOW_FILTER_NODES],
) -> [f32; SSS002_HIGH_LOW_FILTER_NODES] {
    for pivot in 0..SSS002_HIGH_LOW_FILTER_NODES {
        let mut best_row = pivot;
        for row in pivot + 1..SSS002_HIGH_LOW_FILTER_NODES {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs() {
                best_row = row;
            }
        }
        if best_row != pivot {
            matrix.swap(pivot, best_row);
            rhs.swap(pivot, best_row);
        }

        let inverse_pivot = 1.0 / matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value *= inverse_pivot;
        }
        rhs[pivot] *= inverse_pivot;
        let pivot_row = matrix[pivot];
        for row in 0..SSS002_HIGH_LOW_FILTER_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_row[pivot..]) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn invert_sss002_high_low(
    matrix: [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
) -> [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES] {
    let mut inverse = [[0.0; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES];
    for column in 0..SSS002_HIGH_LOW_FILTER_NODES {
        let mut basis = [0.0; SSS002_HIGH_LOW_FILTER_NODES];
        basis[column] = 1.0;
        let solution = solve_sss002_high_low(matrix, basis);
        for (row, value) in solution.into_iter().enumerate() {
            inverse[row][column] = value;
        }
    }
    inverse
}

fn multiply_sss002_high_low(
    matrix: [[f32; SSS002_HIGH_LOW_FILTER_NODES]; SSS002_HIGH_LOW_FILTER_NODES],
    vector: [f32; SSS002_HIGH_LOW_FILTER_NODES],
) -> [f32; SSS002_HIGH_LOW_FILTER_NODES] {
    matrix.map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

fn stamp_sss002_drawing(
    matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
    a: usize,
    b: usize,
    resistance: f32,
) {
    stamp_sss002_drawing_conductance(matrix, a, b, 1.0 / resistance);
}

fn stamp_sss002_drawing_conductance(
    matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
    a: usize,
    b: usize,
    conductance: f32,
) {
    matrix[a][a] += conductance;
    matrix[b][b] += conductance;
    matrix[a][b] -= conductance;
    matrix[b][a] -= conductance;
}

fn stamp_sss002_drawing_to_ground(
    matrix: &mut [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
    node: usize,
    resistance: f32,
) {
    matrix[node][node] += 1.0 / resistance;
}

fn solve_sss002_drawing(
    mut matrix: [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
    mut rhs: [f32; SSS002_DRAWING_HIGH_LOW_NODES],
) -> [f32; SSS002_DRAWING_HIGH_LOW_NODES] {
    for pivot in 0..SSS002_DRAWING_HIGH_LOW_NODES {
        let mut best_row = pivot;
        for row in pivot + 1..SSS002_DRAWING_HIGH_LOW_NODES {
            if matrix[row][pivot].abs() > matrix[best_row][pivot].abs() {
                best_row = row;
            }
        }
        if best_row != pivot {
            matrix.swap(pivot, best_row);
            rhs.swap(pivot, best_row);
        }
        let inverse_pivot = 1.0 / matrix[pivot][pivot];
        for value in &mut matrix[pivot][pivot..] {
            *value *= inverse_pivot;
        }
        rhs[pivot] *= inverse_pivot;
        let pivot_row = matrix[pivot];
        for row in 0..SSS002_DRAWING_HIGH_LOW_NODES {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for (value, pivot_value) in matrix[row][pivot..].iter_mut().zip(&pivot_row[pivot..]) {
                *value -= factor * pivot_value;
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn invert_sss002_drawing(
    matrix: [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
) -> [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES] {
    let mut inverse = [[0.0; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES];
    for column in 0..SSS002_DRAWING_HIGH_LOW_NODES {
        let mut basis = [0.0; SSS002_DRAWING_HIGH_LOW_NODES];
        basis[column] = 1.0;
        let solution = solve_sss002_drawing(matrix, basis);
        for (row, value) in solution.into_iter().enumerate() {
            inverse[row][column] = value;
        }
    }
    inverse
}

fn multiply_sss002_drawing(
    matrix: [[f32; SSS002_DRAWING_HIGH_LOW_NODES]; SSS002_DRAWING_HIGH_LOW_NODES],
    vector: [f32; SSS002_DRAWING_HIGH_LOW_NODES],
) -> [f32; SSS002_DRAWING_HIGH_LOW_NODES] {
    matrix.map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            coefficient: 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp(),
            state: 0.0,
        }
    }

    fn reset(&mut self) {
        self.state = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input - self.state);
        self.state
    }
}

struct VariableOnePole {
    coefficient: f32,
    cutoff_hz: f32,
    state: f32,
}

impl VariableOnePole {
    fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let mut filter = Self {
            coefficient: 0.0,
            cutoff_hz: f32::NAN,
            state: 0.0,
        };
        filter.set_cutoff(sample_rate, cutoff_hz);
        filter
    }

    fn set_cutoff(&mut self, sample_rate: f32, cutoff_hz: f32) {
        if cutoff_hz != self.cutoff_hz {
            self.coefficient = 1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate).exp();
            self.cutoff_hz = cutoff_hz;
        }
    }

    fn reset(&mut self) {
        self.state = 0.0;
    }

    fn process(&mut self, input: f32) -> f32 {
        self.state += self.coefficient * (input - self.state);
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage() -> BrightVolumeInputStage {
        BrightVolumeInputStage::new(BrightVolumeInputParams {
            sample_rate: 48_000.0,
            input_resistance: 1_000_000.0,
            input_coupling_capacitance: 47e-9,
            bright_cutoff_hz: 2_900.0,
            bright_bypass_gain: 0.18,
        })
    }

    fn cut_presence() -> CutPresenceStage {
        CutPresenceStage::new(CutPresenceParams {
            sample_rate: 48_000.0,
            min_cutoff_hz: 1_150.0,
            max_cutoff_hz: 13_500.0,
            presence_gain: 0.35,
        })
    }

    #[test]
    fn input_coupling_blocks_dc() {
        let mut stage = stage();
        let mut sum = 0.0;
        for sample_idx in 0..96_000 {
            let output = stage.process(0.4, 1.0);
            if sample_idx >= 95_000 {
                sum += output.abs();
            }
        }

        assert!(sum / 1_000.0 < 0.01, "settled_dc={}", sum / 1_000.0);
    }

    #[test]
    fn volume_reduces_midband_level() {
        let mut open = stage();
        let mut low = stage();
        let open_rms = sine_rms(&mut open, 1_000.0, 0.1, 1.0);
        let low_rms = sine_rms(&mut low, 1_000.0, 0.1, 0.35);

        assert!(open_rms > low_rms * 5.0, "open={open_rms}, low={low_rms}");
    }

    #[test]
    fn bright_path_keeps_highs_when_volume_is_low() {
        let mut low_frequency = stage();
        let mut high_frequency = stage();
        let low_rms = sine_rms(&mut low_frequency, 300.0, 0.1, 0.15);
        let high_rms = sine_rms(&mut high_frequency, 5_000.0, 0.1, 0.15);

        assert!(
            high_rms > low_rms * 1.8,
            "low_rms={low_rms}, high_rms={high_rms}"
        );
    }

    #[test]
    fn reset_clears_filter_history() {
        let mut stage = stage();
        for _ in 0..24_000 {
            stage.process(0.3, 0.8);
        }
        stage.reset();
        let first = stage.process(0.0, 0.8);

        assert!(first.abs() < 1e-6, "first={first}");
    }

    #[test]
    fn cut_control_reduces_high_frequency_level() {
        let mut open = cut_presence();
        let mut cut = cut_presence();
        let open_rms = cut_presence_sine_rms(&mut open, 6_000.0, 0.2, 0.0, 0.0);
        let cut_rms = cut_presence_sine_rms(&mut cut, 6_000.0, 0.2, 1.0, 0.0);

        assert!(
            open_rms > cut_rms * 2.0,
            "open_rms={open_rms}, cut_rms={cut_rms}"
        );
    }

    #[test]
    fn presence_restores_some_high_frequency_level() {
        let mut dark = cut_presence();
        let mut present = cut_presence();
        let dark_rms = cut_presence_sine_rms(&mut dark, 6_000.0, 0.2, 1.0, 0.0);
        let present_rms = cut_presence_sine_rms(&mut present, 6_000.0, 0.2, 1.0, 1.0);

        assert!(
            present_rms > dark_rms * 1.15,
            "dark_rms={dark_rms}, present_rms={present_rms}"
        );
    }

    #[test]
    fn cut_presence_reset_clears_history() {
        let mut stage = cut_presence();
        for _ in 0..24_000 {
            stage.process(0.2, 0.8, 0.4);
        }
        stage.reset();
        let first = stage.process(0.0, 0.8, 0.4);

        assert!(first.abs() < 1e-6, "first={first}");
    }

    #[test]
    fn classic_tmb_has_passive_insertion_loss() {
        let mut stack = ClassicTmbToneStack::new(48_000.0);
        let output_rms = classic_tmb_sine_rms(&mut stack, 1_000.0, 1.0, 0.46, 0.64, 0.70);

        // ngspice fixture at the same control point: -19.41 dB / 0.1069 V
        // peak from a 1 V source, or 0.0756 V RMS for a 1 V-peak sine.
        assert!(
            (output_rms - 0.0756).abs() < 0.005,
            "output_rms={output_rms}"
        );
    }

    #[test]
    fn classic_tmb_controls_change_the_expected_bands() {
        let mut bass_low = ClassicTmbToneStack::new(48_000.0);
        let mut bass_high = ClassicTmbToneStack::new(48_000.0);
        let bass_low_rms = classic_tmb_sine_rms(&mut bass_low, 100.0, 0.5, 0.0, 0.64, 0.70);
        let bass_high_rms = classic_tmb_sine_rms(&mut bass_high, 100.0, 0.5, 1.0, 0.64, 0.70);

        let mut treble_low = ClassicTmbToneStack::new(48_000.0);
        let mut treble_high = ClassicTmbToneStack::new(48_000.0);
        let treble_low_rms = classic_tmb_sine_rms(&mut treble_low, 4_000.0, 0.5, 0.46, 0.64, 0.0);
        let treble_high_rms = classic_tmb_sine_rms(&mut treble_high, 4_000.0, 0.5, 0.46, 0.64, 1.0);

        assert!(
            bass_high_rms > bass_low_rms * 1.02,
            "bass_low={bass_low_rms}, bass_high={bass_high_rms}"
        );
        assert!(
            treble_high_rms > treble_low_rms * 1.2,
            "treble_low={treble_low_rms}, treble_high={treble_high_rms}"
        );
    }

    #[test]
    fn sss002_high_filter_tracks_the_spice_switch_positions_at_1khz() {
        // Reference gains from daybreaker_sss002_high_low_filters.cir with its
        // documented 1 kOhm source and 1 MOhm load boundary.
        let spice_1khz_db = [-13.09, -30.38, -26.90, -20.11, -16.13, -14.40, -13.70];
        for (index, expected_db) in spice_1khz_db.into_iter().enumerate() {
            let mut filter = Sss002HighFilter::new(48_000.0);
            let output_rms = sss002_high_sine_rms(&mut filter, 1_000.0, 1.0, index + 1);
            let input_rms = 1.0 / 2.0_f32.sqrt();
            let actual_db = 20.0 * (output_rms / input_rms).max(1e-12).log10();
            assert!(
                (actual_db - expected_db).abs() < 0.35,
                "position={} actual_db={} expected_db={}",
                index + 1,
                actual_db,
                expected_db,
            );
        }
    }

    #[test]
    fn sss002_high_filter_reset_clears_capacitor_history() {
        let mut filter = Sss002HighFilter::new(48_000.0);
        for _ in 0..48_000 {
            filter.process(0.25, 4);
        }
        filter.reset();

        assert!(filter.process(0.0, 4).abs() < 1e-6);
    }

    #[test]
    fn sss002_low_filter_tracks_the_spice_switch_positions_at_1khz() {
        // Reference gains from daybreaker_sss002_high_low_filters.cir with its
        // documented 1 Mohm load boundary.
        let spice_1khz_db = [-23.08, -16.16, -11.46, -8.88, -7.32, -7.32, -27.52];
        for (index, expected_db) in spice_1khz_db.into_iter().enumerate() {
            let mut filter = Sss002LowFilter::new(48_000.0);
            let output_rms = sss002_low_sine_rms(&mut filter, 1_000.0, 1.0, index + 1);
            let input_rms = 1.0 / 2.0_f32.sqrt();
            let actual_db = 20.0 * (output_rms / input_rms).max(1e-12).log10();
            assert!(
                (actual_db - expected_db).abs() < 0.35,
                "position={} actual_db={} expected_db={}",
                index + 1,
                actual_db,
                expected_db,
            );
        }
    }

    #[test]
    fn sss002_low_filter_reset_clears_capacitor_history() {
        let mut filter = Sss002LowFilter::new(48_000.0);
        for _ in 0..48_000 {
            filter.process(0.25, 2);
        }
        filter.reset();

        assert!(filter.process(0.0, 2).abs() < 1e-6);
    }

    #[test]
    fn sss002_high_low_filter_tracks_the_source_drawing_default_at_1khz() {
        let mut filter = Sss002HighLowFilter::new(48_000.0);
        let output_rms = sss002_high_low_sine_rms(&mut filter, 1_000.0, 1.0, 1, 1);
        let input_rms = 1.0 / 2.0_f32.sqrt();
        let actual_db = 20.0 * (output_rms / input_rms).max(1e-12).log10();
        let spice_db = -14.55;
        assert!(
            (actual_db - spice_db).abs() < 0.35,
            "actual_db={} spice_db={}",
            actual_db,
            spice_db,
        );
    }

    #[test]
    fn sss002_drawing_high_low_tracks_spice_across_the_guitar_band() {
        // daybreaker_sss002_high_low_chain.cir, High-1/Low-1 default.
        for (frequency_hz, spice_db) in [(100.0, -20.56), (1_000.0, -15.82), (8_000.0, -24.05)] {
            let mut filter = Sss002DrawingHighLowFilter::new(48_000.0);
            let output_rms = sss002_drawing_high_low_sine_rms(&mut filter, frequency_hz, 1.0);
            let input_rms = 1.0 / 2.0_f32.sqrt();
            let actual_db = 20.0 * (output_rms / input_rms).max(1e-12).log10();
            assert!(
                (actual_db - spice_db).abs() < 1.0,
                "frequency_hz={} actual_db={} spice_db={}",
                frequency_hz,
                actual_db,
                spice_db,
            );
        }
    }

    #[test]
    fn sss002_u5_volume_control_tracks_the_drawing_default() {
        let volume = Sss002VolumeControl::new();
        let (source_to_wiper, wiper_to_ground) = volume.leg_resistances(0.5);

        assert!((source_to_wiper - 900_000.0).abs() < 1.0);
        assert!((wiper_to_ground - 100_000.0).abs() < 1.0);
        // daybreaker_sss002_u5_volume_u4.cir measures 0.099970x. The small
        // difference is U4's finite grid capacitance through R69.
        assert!((volume.process_ideal_source(1.0, 0.5) - 0.1).abs() < 1e-6);
    }

    #[test]
    fn sss002_high_low_filter_reset_clears_capacitor_history() {
        let mut filter = Sss002HighLowFilter::new(48_000.0);
        for _ in 0..48_000 {
            filter.process(0.25, 1, 1);
        }
        filter.reset();

        assert!(filter.process(0.0, 1, 1).abs() < 1e-6);
    }

    #[test]
    fn sss002_u37_recovery_tracks_spice_operating_point_and_gain() {
        let mut stage = Sss002HighLowU37RecoveryStage::new(48_000.0);
        let output_rms = sss002_u37_recovery_sine_rms(&mut stage, 1_000.0, 0.020);
        let operating_point = stage.operating_point();
        assert!(
            (operating_point.plate_voltage - 177.011).abs() < 0.1,
            "plate={}",
            operating_point.plate_voltage
        );
        assert!(
            (operating_point.cathode_voltage - 1.230).abs() < 0.01,
            "cathode={}",
            operating_point.cathode_voltage
        );
        assert!(
            (output_rms - 0.132_360).abs() < 0.015,
            "output_rms={output_rms}"
        );
    }

    #[test]
    fn classic_tmb_reset_clears_capacitor_history() {
        let mut stack = ClassicTmbToneStack::new(48_000.0);
        for _ in 0..24_000 {
            stack.process(0.3, 0.46, 0.64, 0.70);
        }
        stack.reset();

        assert!(stack.process(0.0, 0.46, 0.64, 0.70).abs() < 1e-6);
    }

    #[test]
    fn classic_tmb_recovery_tracks_spice_operating_point_and_gain() {
        let mut stage = ClassicTmbRecoveryStage::new(48_000.0);
        let output_rms =
            classic_tmb_recovery_sine_rms(&mut stage, 1_000.0, 0.020, 0.46, 0.64, 0.70);
        let operating_point = stage.operating_point();

        assert!(
            (operating_point.plate_voltage - 252.40).abs() < 4.0,
            "plate={}",
            operating_point.plate_voltage
        );
        assert!(
            (operating_point.cathode_voltage - 0.542).abs() < 0.10,
            "cathode={}",
            operating_point.cathode_voltage
        );
        // SPICE: 20.254 mV RMS after the output coupling/load and 60 ms
        // settling window.
        assert!(
            (output_rms - 0.02025).abs() < 0.006,
            "output_rms={output_rms}"
        );
    }

    #[test]
    fn classic_tmb_recovery_settles_after_a_bounded_attack() {
        let mut stage = ClassicTmbRecoveryStage::new(48_000.0);
        for sample_idx in 0..24_000 {
            let input =
                (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.030;
            stage.process(input, 0.46, 0.64, 0.70);
        }

        let mut sum_squares = 0.0;
        for sample_idx in 0..96_000 {
            let output = stage.process(0.0, 0.46, 0.64, 0.70);
            if sample_idx >= 48_000 {
                sum_squares += output * output;
            }
        }
        let tail_rms = (sum_squares / 48_000.0).sqrt();

        assert!(tail_rms < 1.0e-4, "tail_rms={tail_rms}");
    }

    #[test]
    fn ecc83_into_tmb_recovery_settles_after_a_bounded_attack() {
        let mut first_stage = CommonCathodeStage::new(CommonCathodeParams {
            sample_rate: 48_000.0,
            grid_leak_resistance: 1_000_000.0,
            input_coupling_capacitance: 22e-9,
            plate_resistance: 100_000.0,
            cathode_resistance: 1_500.0,
            cathode_bypass_capacitance: Some(25e-6),
            supply_resistance: 10_000.0,
            supply_capacitance: 22e-6,
            nominal_supply_voltage: 280.0,
            input_gain: 1.0,
            output_scale: 1.0,
            triode: TriodeParams::ECC83,
        });
        let mut recovery = ClassicTmbRecoveryStage::new(48_000.0);
        for sample_idx in 0..24_000 {
            let input =
                (std::f32::consts::TAU * 220.0 * sample_idx as f32 / 48_000.0).sin() * 0.030;
            recovery.process(first_stage.process(input), 0.46, 0.64, 0.70);
        }

        let mut sum_squares = 0.0;
        for sample_idx in 0..96_000 {
            let output = recovery.process(first_stage.process(0.0), 0.46, 0.64, 0.70);
            if sample_idx >= 48_000 {
                sum_squares += output * output;
            }
        }
        let tail_rms = (sum_squares / 48_000.0).sqrt();

        assert!(tail_rms < 1.0e-4, "tail_rms={tail_rms}");
    }

    fn sine_rms(
        stage: &mut BrightVolumeInputStage,
        frequency: f32,
        amplitude: f32,
        volume: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        for sample_idx in 0..48_000 {
            let input = (std::f32::consts::TAU * frequency * sample_idx as f32 / 48_000.0).sin()
                * amplitude;
            let output = stage.process(input, volume);
            if sample_idx >= 24_000 {
                sum += output * output;
                count += 1;
            }
        }
        (sum / count as f32).sqrt()
    }

    fn cut_presence_sine_rms(
        stage: &mut CutPresenceStage,
        frequency: f32,
        amplitude: f32,
        cut: f32,
        presence: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        for sample_idx in 0..48_000 {
            let input = (std::f32::consts::TAU * frequency * sample_idx as f32 / 48_000.0).sin()
                * amplitude;
            let output = stage.process(input, cut, presence);
            if sample_idx >= 24_000 {
                sum += output * output;
                count += 1;
            }
        }
        (sum / count as f32).sqrt()
    }

    fn classic_tmb_sine_rms(
        stack: &mut ClassicTmbToneStack,
        frequency: f32,
        amplitude: f32,
        bass: f32,
        mid: f32,
        treble: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        for sample_idx in 0..96_000 {
            let input = (std::f32::consts::TAU * frequency * sample_idx as f32 / 48_000.0).sin()
                * amplitude;
            let output = stack.process(input, bass, mid, treble);
            if sample_idx >= 48_000 {
                sum += output * output;
                count += 1;
            }
        }
        (sum / count as f32).sqrt()
    }

    fn sss002_high_sine_rms(
        filter: &mut Sss002HighFilter,
        frequency: f32,
        amplitude: f32,
        position: usize,
    ) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum_squares = 0.0;
        let mut samples = 0;
        for sample_idx in 0..sample_rate as usize * 3 {
            let time = sample_idx as f32 / sample_rate;
            let input = amplitude * (std::f32::consts::TAU * frequency * time).sin();
            let output = filter.process(input, position);
            if sample_idx >= sample_rate as usize {
                sum_squares += output * output;
                samples += 1;
            }
        }
        (sum_squares / samples as f32).sqrt()
    }

    fn sss002_low_sine_rms(
        filter: &mut Sss002LowFilter,
        frequency: f32,
        amplitude: f32,
        position: usize,
    ) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum_squares = 0.0;
        let mut samples = 0;
        for sample_idx in 0..sample_rate as usize * 3 {
            let time = sample_idx as f32 / sample_rate;
            let input = amplitude * (std::f32::consts::TAU * frequency * time).sin();
            let output = filter.process(input, position);
            if sample_idx >= sample_rate as usize {
                sum_squares += output * output;
                samples += 1;
            }
        }
        (sum_squares / samples as f32).sqrt()
    }

    fn sss002_high_low_sine_rms(
        filter: &mut Sss002HighLowFilter,
        frequency: f32,
        amplitude: f32,
        high_position: usize,
        low_position: usize,
    ) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum_squares = 0.0;
        let mut samples = 0;
        for sample_idx in 0..sample_rate as usize * 3 {
            let time = sample_idx as f32 / sample_rate;
            let input = amplitude * (std::f32::consts::TAU * frequency * time).sin();
            let output = filter.process(input, high_position, low_position);
            if sample_idx >= sample_rate as usize {
                sum_squares += output * output;
                samples += 1;
            }
        }
        (sum_squares / samples as f32).sqrt()
    }

    fn sss002_drawing_high_low_sine_rms(
        filter: &mut Sss002DrawingHighLowFilter,
        frequency: f32,
        amplitude: f32,
    ) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum_squares = 0.0;
        let mut samples = 0;
        for sample_idx in 0..sample_rate as usize * 4 {
            let time = sample_idx as f32 / sample_rate;
            let input = amplitude * (std::f32::consts::TAU * frequency * time).sin();
            let output = filter.process(input);
            if sample_idx >= sample_rate as usize * 2 {
                sum_squares += output * output;
                samples += 1;
            }
        }
        (sum_squares / samples as f32).sqrt()
    }

    fn sss002_u37_recovery_sine_rms(
        stage: &mut Sss002HighLowU37RecoveryStage,
        frequency: f32,
        amplitude: f32,
    ) -> f32 {
        let sample_rate = 48_000.0;
        let mut sum_squares = 0.0;
        let mut samples = 0;
        for sample_idx in 0..sample_rate as usize * 3 {
            let time = sample_idx as f32 / sample_rate;
            let input = amplitude * (std::f32::consts::TAU * frequency * time).sin();
            let output = stage.process(input);
            if sample_idx >= sample_rate as usize {
                sum_squares += output * output;
                samples += 1;
            }
        }
        (sum_squares / samples as f32).sqrt()
    }

    fn classic_tmb_recovery_sine_rms(
        stage: &mut ClassicTmbRecoveryStage,
        frequency: f32,
        amplitude: f32,
        bass: f32,
        mid: f32,
        treble: f32,
    ) -> f32 {
        let mut sum = 0.0;
        let mut sum_squares = 0.0;
        let mut count = 0;
        for sample_idx in 0..96_000 {
            let input = (std::f32::consts::TAU * frequency * sample_idx as f32 / 48_000.0).sin()
                * amplitude;
            let output = stage.process(input, bass, mid, treble);
            if sample_idx >= 48_000 {
                sum += output;
                sum_squares += output * output;
                count += 1;
            }
        }
        let mean = sum / count as f32;
        ((sum_squares / count as f32) - mean * mean).max(0.0).sqrt()
    }
}
use crate::circuit::triode::{CommonCathodeParams, CommonCathodeStage, TriodeParams};
