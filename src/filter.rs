use core::cmp::{max, min};
use core::f32::consts::PI;
use micromath::F32Ext;
use ordered_float::OrderedFloat;

use crate::delay::DelayLine;

pub struct OnePoleLowPass {
    sample_rate: f32,
    a0: f32,
    b1: f32,
    z1: f32,
}

impl OnePoleLowPass {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            a0: 1.0,
            b1: 0.0,
            z1: 0.0,
        }
    }

    pub fn set_freq(&mut self, freq: f32) {
        let freq = freq / self.sample_rate;
        self.b1 = (-2.0 * PI * freq).exp();
        self.a0 = 1.0 - self.b1;
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.z1 = (input * self.a0) + (self.z1 * self.b1);
        self.z1
    }
}

pub struct AllPassSP<'a> {
    sample_rate: f32,
    delay_line: DelayLine<'a>,
    reverb_time: f32,
    max_loop_time: f32,
    loop_time: f32,
    rollover: usize,
    coef: f32,
}

impl<'a> AllPassSP<'a> {
    pub fn new(sample_rate: f32, delay_line: DelayLine<'a>) -> Self {
        let max_loop_time: f32 = delay_line.len() as f32 / sample_rate - 0.01;
        let rollover = (max_loop_time * sample_rate) as usize;

        let mut all_pass = Self {
            sample_rate,
            delay_line,
            loop_time: max_loop_time,
            max_loop_time,
            rollover,
            coef: 0.0,
            reverb_time: 0.0,
        };
        all_pass.calc_reverb();
        all_pass
    }

    fn calc_reverb(&mut self) {
        self.coef = (-6.9078 * self.loop_time / self.reverb_time).exp();
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let y = self.delay_line.read();
        let z = (self.coef * y) + input;
        self.delay_line.write(z);

        y - (self.coef * z)
    }

    pub fn set_freq(&mut self, delay: f32) {
        self.loop_time = max(
            min(OrderedFloat(delay), OrderedFloat(self.max_loop_time)),
            OrderedFloat(0.0001),
        )
        .0;
        self.rollover = max((self.loop_time * self.sample_rate) as usize, 0);
        self.calc_reverb();
    }

    pub fn set_reverb_time(&mut self, reverb_time: f32) {
        self.reverb_time = reverb_time;
        self.calc_reverb();
    }
}

pub struct AllPass<'a> {
    sample_rate: f32,
    delay_line: DelayLine<'a>,
    k1: f32,
}

impl<'a> AllPass<'a> {
    pub fn new(sample_rate: f32, delay_line: DelayLine<'a>) -> Self {
        Self {
            sample_rate,
            k1: 0.0,
            delay_line,
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let z1 = self.delay_line.read();
        let x = (self.k1 * z1) + input;
        self.delay_line.write(x);

        z1 - (self.k1 * x)
    }

    pub fn set_freq(&mut self, freq: f32) {
        let freq = PI * freq / self.sample_rate;
        self.k1 = (1.0 - freq) / (1.0 + freq);
    }
}

pub struct StateVariable {
    sample_rate: f32,
    low_pass: f32,
    high_pass: f32,
    band_pass: f32,
    notch: f32,
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
            freq: 0.0,
            resonance: 0.0,
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

    // pub fn process(&mut self, input: f32) {
    //     // First pass
    //     self.pass(input);
    //     self.out_low_pass = 0.5 * self.low_pass;
    //     self.out_high_pass = 0.5 * self.high_pass;
    //     self.out_band_pass = 0.5 * self.band_pass;
    //     self.out_peak = 0.5 * (self.low_pass - self.high_pass);
    //     self.out_notch = 0.5 * self.notch;
    //     // Second pass
    //     self.pass(input);
    //     self.out_low_pass += 0.5 * self.low_pass;
    //     self.out_high_pass += 0.5 * self.high_pass;
    //     self.out_band_pass += 0.5 * self.band_pass;
    //     self.out_peak += 0.5 * (self.low_pass - self.high_pass);
    //     self.out_notch += 0.5 * self.notch;
    // }

    pub fn process(&mut self, input: f32) {
        self.pass(self.previous);
        self.pass(input);
        self.out_low_pass = self.low_pass;
        self.out_high_pass = self.high_pass;
        self.out_band_pass = self.band_pass;
        self.out_peak = self.low_pass - self.high_pass;
        self.out_notch = self.notch;
        self.previous = input;
    }

    /// Set the cutoff frequency
    pub fn set_freq(&mut self, freq: f32) {
        let freq = freq.clamp(0.0, self.sample_rate / 3.0);
        self.freq = 2.0
            * (PI
                * min(
                    OrderedFloat(0.25),
                    OrderedFloat(freq / (self.sample_rate * 2.0)),
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

    pub fn get_low_pass(&self) -> f32 {
        self.out_low_pass
    }

    pub fn get_high_pass(&self) -> f32 {
        self.out_high_pass
    }

    pub fn get_band_pass(&self) -> f32 {
        self.out_band_pass
    }

    pub fn get_notch(&self) -> f32 {
        self.out_notch
    }

    pub fn get_peak(&self) -> f32 {
        self.out_peak
    }
}

#[cfg(test)]
mod tests {
    const SAMPLE_RATE_F: f32 = 44100.0;
    const SAMPLE_RATE: u32 = 44100;
    const NYQUIST: f32 = SAMPLE_RATE_F / 2.0;

    use super::*;

    use plotters::prelude::*;
    use rand::distributions::{Distribution, Uniform};
    use spectrum_analyzer::{samples_fft_to_spectrum, scaling, FrequencyLimit};

    fn graph_log_log(data: Vec<(f32, f32)>, label: &str, path: &str) {
        let root = BitMapBackend::new(path, (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption(label, ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(LogRange(0.1..NYQUIST), -51f32..11f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();
        chart.draw_series(LineSeries::new(data, &RED)).unwrap();
    }

    #[test]
    fn test_white_noise() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        for item in &mut white_noise {
            *item = between.sample(&mut rng);
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            // samples
            &white_noise,
            // sampling rate
            44100,
            // optional frequency limit: e.g. only interested in frequencies 50 <= f <= 150?
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "White Noise", "test_white_noise.png");
    }

    #[test]
    fn test_onepole_low() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = OnePoleLowPass::new(SAMPLE_RATE_F);
        filter.set_freq(100.0);
        for item in &mut instant {
            *item = filter.process(*item);
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        for (hz, db) in &data {
            if *hz < 100.0 {
                assert!(*db > -3.0);
            } else if *hz < 120.0 {
                assert!(*db < -3.0);
            } else {
                break;
            }
        }
        graph_log_log(data, "LPF 100 Hz", "test_onepole_low.png");
    }

    #[test]
    fn test_onepole_high() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = OnePoleLowPass::new(SAMPLE_RATE_F);
        filter.set_freq(100.0);
        for item in &mut instant {
            *item -= filter.process(*item);
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        for (hz, db) in &data {
            if *hz < 100.0 {
                assert!(*db < -3.0);
            } else if *hz < 120.0 {
                assert!(*db > -3.0);
            } else {
                break;
            }
        }

        graph_log_log(data, "HPF 100 Hz", "test_onepole_high.png");
    }

    #[test]
    fn test_svf_lpf() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(100.0);
        filter.set_resonance(0.0);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.get_low_pass();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "SVF LP 100 Hz", "test_svf_lpf.png");
    }

    #[test]
    fn test_svf_hpf() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(100.0);
        filter.set_resonance(0.0);
        // filter.set_resonance(0.5);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.get_high_pass();
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "SVF HP 100 Hz", "test_svf_hpf.png");
    }

    #[test]
    fn test_svf_band() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(800.0);
        filter.set_resonance(0.01);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.get_band_pass();
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "SVF Band 100 Hz", "test_svf_band.png");
    }

    #[test]
    fn test_svf_notch() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        filter.set_resonance(0.5);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.get_notch();
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "SVF Notch 100 Hz", "test_svf_notch.png");
    }

    #[test]
    fn test_svf_peak() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        instant[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        filter.set_resonance(0.5);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.get_peak();
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "SVF Peak", "test_svf_peak.png");
    }

    #[test]
    fn test_all_pass_1() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        let mut buffer: [f32; 1] = [0.0; 1];
        let delay_line = DelayLine::new(&mut buffer);
        instant[0] = 1.0;
        let mut filter = AllPass::new(SAMPLE_RATE_F, delay_line);
        filter.set_freq(100.0);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.process(*item);
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "All Pass - 1", "test_all_pass_1.png");
    }

    #[test]
    fn test_all_pass_512() {
        let mut instant: [f32; 4096] = [0.0; 4096];
        let mut buffer: [f32; 512] = [0.0; 512];
        let delay_line = DelayLine::new(&mut buffer);
        instant[0] = 1.0;
        let mut filter = AllPass::new(SAMPLE_RATE_F, delay_line);
        filter.set_freq(1000.0);
        for item in &mut instant {
            filter.process(*item);
            *item = filter.process(*item);
        }

        let spectrum = samples_fft_to_spectrum(
            &instant,
            SAMPLE_RATE,
            FrequencyLimit::Max(NYQUIST),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        let data: Vec<(f32, f32)> = spectrum
            .to_map(None)
            .iter()
            .map(|(x, y)| (*x as f32, *y))
            .collect();

        graph_log_log(data, "All Pass - 512", "test_all_pass_512.png");
    }
}
