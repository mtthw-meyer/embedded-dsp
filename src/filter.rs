#![allow(unused_mut)]
#![allow(unused_variables)]
use core::cmp::min;
use core::f32::consts::PI;
use micromath::F32Ext;
use ordered_float::OrderedFloat;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
}

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
        self.z1 = input * self.a0 + self.z1 * self.b1;
        self.z1
    }
}

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
    use super::*;

    use audio_visualizer::spectrum::staticc::plotters_png_file::spectrum_static_plotters_png_visualize;
    use audio_visualizer::test_support::TEST_OUT_DIR;
    use rand::distributions::{Distribution, Uniform};
    use spectrum_analyzer::windows::{blackman_harris_4term, hamming_window, hann_window};
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling, ComplexSpectrumScalingFunction, FrequencyLimit,
    };

    fn scale_to_log() -> ComplexSpectrumScalingFunction {
        Box::new(move |_min: f32, max: f32, _average: f32, _median: f32| {
            Box::new(move |x| (x + 1.0).log10())
        })
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
            FrequencyLimit::Max(2000.0),
            Some(&scaling::basic::scale_20_times_log10),
            None,
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_white_noise.png"),
        );
    }

    #[test]
    fn test_onepole_low() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = OnePoleLowPass::new(44100.0);
        filter.set_freq(100.0);
        for item in &mut white_noise {
            // *item = filter.process(between.sample(&mut rng));
            *item = filter.process(*item);
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(2000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_onepole_low.png"),
        );
    }

    #[test]
    fn test_onepole_high() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = OnePoleLowPass::new(44100.0);
        filter.set_freq(8000.0);
        for item in &mut white_noise {
            *item = *item - filter.process(*item);
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_onepole_high.png"),
        );
    }

    #[test]
    fn test_svf_lpf() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        // filter.set_resonance(0.5);
        for item in &mut white_noise {
            filter.process(*item);
            *item = filter.get_low_pass();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_svf_low.png"),
        );
    }

    #[test]
    fn test_svf_hpf() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        // filter.set_resonance(0.5);
        for item in &mut white_noise {
            filter.process(*item);
            *item = filter.get_high_pass();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_svf_high.png"),
        );
    }

    #[test]
    fn test_svf_band() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        filter.set_resonance(0.01);
        for item in &mut white_noise {
            filter.process(*item);
            *item = filter.get_band_pass();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_svf_band.png"),
        );
    }

    #[test]
    fn test_svf_notch() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        filter.set_resonance(0.5);
        for item in &mut white_noise {
            filter.process(*item);
            *item = filter.get_notch();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_svf_notch.png"),
        );
    }

    #[test]
    fn test_svf_peak() {
        let between = Uniform::new_inclusive(-1.0, 1.0);
        let mut rng = rand::thread_rng();

        let mut white_noise: [f32; 4096] = [0.0; 4096];
        white_noise[0] = 1.0;
        let mut filter = StateVariable::new(44100.0);
        filter.set_freq(1000.0);
        filter.set_resonance(0.5);
        for item in &mut white_noise {
            filter.process(*item);
            *item = filter.get_peak();
        }

        // calc spectrum
        let spectrum = samples_fft_to_spectrum(
            &white_noise,
            44100,
            FrequencyLimit::Max(10_000.0),
            None,
            Some(scale_to_log()),
        );

        spectrum_static_plotters_png_visualize(
            &spectrum.to_map(None),
            TEST_OUT_DIR,
            &format!("test_svf_peak.png"),
        );
    }
}
