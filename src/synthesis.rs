use core::f32::consts::PI;
use micromath::F32Ext;

const TWO_PI: f32 = PI * 2.0;
const TWO_PI_RECIP: f32 = 1.0 / TWO_PI;

pub enum WaveType {
    Sine,
    Triangle,
    Saw,
    Ramp,
    Square,
    PolyBLEPTri,
    PolyBLEPSaw,
    PolyBLEPSquare,
}

/// Implemented based on code from
/// https://github.com/electro-smith/DaisySP/blob/master/Source/Synthesis/oscillator.h
pub struct Oscillator {
    wave_type: WaveType,
    sample_rate: f32,
    amplitude: f32,
    frequency: f32,
    phase: f32,
    phase_inc: f32,
    last: f32,
}

impl Oscillator {
    pub fn new(wave_type: WaveType, sample_rate: f32, frequency: f32) -> Self {
        let mut sine = Self {
            wave_type: wave_type,
            sample_rate,
            amplitude: 1.0,
            frequency,
            phase: 0.0,
            phase_inc: 0.0,
            last: 0.0,
        };
        sine.calc_phase_inc();
        sine
    }

    /// Processes the waveform to be generated, returning one sample. This should be called once per sample period.
    pub fn process(&mut self) -> f32 {
        let out = match self.wave_type {
            WaveType::Sine => self.phase.sin(),
            WaveType::Triangle => {
                let t = (self.phase * TWO_PI_RECIP * 2.0) - 1.0;
                2.0 * (t.abs() - 0.5)
            }
            WaveType::Saw => -1.0 * ((self.phase * TWO_PI_RECIP * 2.0) - 1.0),
            WaveType::Ramp => (self.phase * TWO_PI_RECIP * 2.0) - 1.0,
            WaveType::Square => {
                if self.phase < PI {
                    1.0
                } else {
                    -1.0
                }
            }
            WaveType::PolyBLEPTri => {
                let t = self.phase * TWO_PI_RECIP;
                let mut out = if self.phase < PI { 1.0 } else { -1.0 };
                out += poly_blep(self.phase_inc, t);
                out -= poly_blep(self.phase_inc, (t + 0.5) % 1.0);
                // Leaky Integrator:
                // y[n] = A + x[n] + (1 - A) * y[n-1]
                out = self.phase_inc * out + (1.0 - self.phase_inc) * self.last;
                self.last = out;
                out
            }
            WaveType::PolyBLEPSaw => {
                let t = self.phase * TWO_PI_RECIP;
                let mut out = (2.0 * t) - 1.0;
                out -= poly_blep(self.phase_inc, t);
                out *= -1.0;
                out
            }
            WaveType::PolyBLEPSquare => {
                let t = self.phase * TWO_PI_RECIP;
                let mut out = if self.phase < PI { 1.0 } else { -1.0 };
                out += poly_blep(self.phase_inc, t);
                out -= poly_blep(self.phase_inc, (t + 0.5) % 1.0);
                out
            }
        };
        self.phase += self.phase_inc;
        if self.phase > TWO_PI {
            self.phase -= TWO_PI;
        }
        out * self.amplitude
    }

    fn calc_phase_inc(&mut self) {
        self.phase_inc = TWO_PI * self.frequency / self.sample_rate;
    }

    /// Set the frequency.
    pub fn set_freq(&mut self, frequency: f32) {
        self.frequency = frequency;
        self.calc_phase_inc();
    }

    /// Set the amplitude.
    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.amplitude = amplitude;
    }

    /// Set the phase to value, clamped to 0.0-1.0.
    pub fn set_phase(&mut self, phase: f32) {
        self.phase = phase.clamp(0.0, 1.0) * TWO_PI;
    }
}

// Polynomial bandlimited step calculator
fn poly_blep(phase_inc: f32, t: f32) -> f32 {
    let dt = phase_inc * TWO_PI_RECIP;
    let mut t = t;
    if t < dt {
        t /= dt;
        return t + t - t * t - 1.0;
    } else if t > (1.0 - dt) {
        t = (t - 1.0) / dt;
        return t * t + t + t + 1.0;
    }
    0.0
}

#[cfg(test)]
mod tests {
    const SAMPLE_RATE: f32 = 44100.0;
    use super::*;

    use plotters::prelude::*;
    use spectrum_analyzer::windows::{blackman_harris_4term, hamming_window, hann_window};
    use spectrum_analyzer::{
        samples_fft_to_spectrum, scaling, ComplexSpectrumScalingFunction, FrequencyLimit,
    };

    #[test]
    fn test_sine() {
        let mut oscillator = Oscillator::new(WaveType::Sine, SAMPLE_RATE, 1.5);
        let root = BitMapBackend::new("test_sine.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("y=sin x", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_triangle() {
        let mut oscillator = Oscillator::new(WaveType::Triangle, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_triangle.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("Triangle", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_ramp() {
        let mut oscillator = Oscillator::new(WaveType::Ramp, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_ramp.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("Ramp", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_saw() {
        let mut oscillator = Oscillator::new(WaveType::Saw, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_saw.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("Saw", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_square() {
        let mut oscillator = Oscillator::new(WaveType::Square, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_square.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("Square", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_poly_square() {
        let mut oscillator = Oscillator::new(WaveType::PolyBLEPSquare, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_poly_square.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("PolyBLEP Square", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_poly_triangle() {
        let mut oscillator = Oscillator::new(WaveType::PolyBLEPTri, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_poly_triangle.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("PolyBLEP Triangle", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }

    #[test]
    fn test_poly_saw() {
        let mut oscillator = Oscillator::new(WaveType::PolyBLEPSaw, SAMPLE_RATE, 1.5);

        let root = BitMapBackend::new("test_poly_saw.png", (640, 480)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let mut chart = ChartBuilder::on(&root)
            .caption("PolyBLEP Saw", ("sans-serif", 50).into_font())
            .margin(5)
            .x_label_area_size(30)
            .y_label_area_size(30)
            .build_cartesian_2d(0f32..5.5f32, -1.2f32..1.2f32)
            .unwrap();

        chart.configure_mesh().draw().unwrap();

        chart
            .draw_series(LineSeries::new(
                (0..(SAMPLE_RATE as u32 * 5))
                    .map(|x| (x as f32 / SAMPLE_RATE, oscillator.process())),
                &RED,
            ))
            .unwrap();
    }
}
