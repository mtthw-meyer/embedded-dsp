#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_imports)]
pub mod filter;

mod delay {
    use core::ops::{Index, IndexMut};

    pub struct DelayLine<'a> {
        inner: &'a mut [f32],
        index: usize,
    }

    impl<'a> DelayLine<'a> {
        pub fn new(inner: &'a mut [f32]) -> DelayLine {
            DelayLine { inner, index: 0 }
        }

        pub fn process(&mut self, input: f32) -> f32 {
            let output = self.inner[self.index];
            self.index = (self.index + 1) % self.inner.len();
            self.inner[self.index] = input;
            output
        }

        pub fn read(self, index: usize) -> f32 {
            self.inner[index % self.inner.len()]
        }

        pub fn write(&mut self, input: f32) {
            self.index = (self.index + 1) % self.inner.len();
            self.inner[self.index] = input;
        }

        pub fn len(&self) -> usize {
            self.inner.len()
        }
    }

    impl Index<usize> for DelayLine<'_> {
        type Output = f32;

        fn index(&self, index: usize) -> &Self::Output {
            &self.inner[index]
        }
    }

    impl IndexMut<usize> for DelayLine<'_> {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.inner[index]
        }
    }
}

// mod reverb {
//     pub struct Dattorro {}

//     impl Dattorro {}
// }

mod synthesis {
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
        PolyBLEPSqaure,
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
                WaveType::PolyBLEPSqaure => {
                    let t = self.phase * TWO_PI_RECIP;
                    let mut out = if self.phase < PI { 1.0 } else { -1.0 };
                    out += poly_blep(self.phase_inc, t);
                    out -= poly_blep(self.phase_inc, (t + 0.5) % 1.0);
                    out *= 0.707;
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
}
