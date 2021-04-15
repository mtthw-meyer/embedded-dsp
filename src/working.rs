pub struct AllPass {
    buffer: &'static mut [f32],
    index: usize,
}

impl AllPass {
    pub fn new(sample_rate: u32, buffer: &'static mut [f32]) -> AllPass {
        let max_loop_time: f32 = buffer.len() as f32 / AUDIO_SAMPLE_RATE - 0.01;

        AllPass { buffer, index: 0 }
    }

    pub fn process(&mut self, _input: f32) -> f32 {
        0.0
    }
}

const SVF_MAX_FREQ: f32 = AUDIO_SAMPLE_RATE / 3.0;
pub struct StateVariable {
    sample_rate: f32,
    low_pass: f32,
    high_pass: f32,
    band_pass: f32,
    notch: f32,
    peak: f32,
    freq: f32,
    resonance: f32,
    pre_drive: f32,
    drive: f32,
    damp: f32,
    out_low_pass: f32,
    out_high_pass: f32,
    out_band_pass: f32,
    out_notch: f32,
    out_peak: f32,
    previous: f32,
}

impl StateVariable {
    pub fn new(sample_rate: f32) -> StateVariable {
        StateVariable {
            sample_rate,
            low_pass: 0.0,
            high_pass: 0.0,
            band_pass: 0.0,
            notch: 0.0,
            peak: 0.0,
            freq: 0.0,
            resonance: 0.5,
            pre_drive: 0.0,
            drive: 0.0,
            damp: 0.0,
            out_low_pass: 0.0,
            out_high_pass: 0.0,
            out_band_pass: 0.0,
            out_notch: 0.0,
            out_peak: 0.0,
            previous: 0.0,
        }
    }

    fn pass(&mut self, input: f32) {
        self.notch = input - self.damp * self.band_pass;
        self.low_pass = self.low_pass + self.freq * self.band_pass;
        self.high_pass = self.notch - self.low_pass;
        self.band_pass =
            self.freq * self.high_pass + self.band_pass - self.drive * self.band_pass.powi(3);
    }

    fn calc_damp(&mut self) {
        self.damp = min(
            OrderedFloat(2.0 * (1.0 - self.resonance.powf(0.25))),
            min(
                OrderedFloat(2.0),
                OrderedFloat(2.0 / self.freq - self.freq * 0.5),
            ),
        )
        .0;
    }

    pub fn process(&mut self, input: f32) {
        // First pass
        self.pass(input);
        self.out_low_pass = 0.5 * self.low_pass;
        self.out_high_pass = 0.5 * self.high_pass;
        self.out_band_pass = 0.5 * self.band_pass;
        self.out_peak = 0.5 * (self.low_pass - self.high_pass);
        self.out_notch = 0.5 * self.notch;
        // Second pass
        self.pass(input);
        self.out_low_pass += 0.5 * self.low_pass;
        self.out_high_pass += 0.5 * self.high_pass;
        self.out_band_pass += 0.5 * self.band_pass;
        self.out_peak += 0.5 * (self.low_pass - self.high_pass);
        self.out_notch += 0.5 * self.notch;
    }

    // pub fn process2(&mut self, input: f32) {
    //     // First pass
    //     self.notch = input - self.damp * self.band_pass;
    //     self.low_pass = self.low_pass + self.freq * self.band_pass;
    //     //self.high_pass = self.notch - self.low_pass;
    //     self.high_pass = 0.5 * g? * (self.previous + input) - self.low_pass - q*self.band_pass;
    //     self.band_pass = self.freq * self.high_pass + self.band_pass;

    //     self.previous = input;

    // }

    /// Set the cutoff frequency
    pub fn set_freq(&mut self, freq: f32) {
        let freq = freq.clamp(0.0, AUDIO_SAMPLE_RATE / 3.0);
        self.freq = 2.0
            * (PI
                * min(
                    OrderedFloat(0.25),
                    OrderedFloat(freq / AUDIO_SAMPLE_RATE * 2.0),
                )
                .0)
                .sin();
        self.calc_damp();
    }

    /// Set filter resonance, clamped to [0.0-1.0].
    pub fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
        // Recalculate damp and drive
        self.calc_damp();
        self.drive = self.pre_drive * self.resonance;
    }

    /// Set internal distortion, clamped to [0.0-1.0].
    pub fn set_drive(&mut self, drive: f32) {
        // Actual value is clamped from [0.0-0.1]
        self.pre_drive = (drive * 0.1).clamp(0.0, 0.1);
        self.drive = self.pre_drive * self.resonance;
    }

    pub fn get_low_pass(self) -> f32 {
        self.out_low_pass
    }

    pub fn get_high_pass(self) -> f32 {
        self.out_high_pass
    }

    pub fn get_band_pass(self) -> f32 {
        self.out_band_pass
    }

    pub fn get_notch(self) -> f32 {
        self.out_notch
    }

    pub fn get_peak(self) -> f32 {
        self.out_peak
    }
}