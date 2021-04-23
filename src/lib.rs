#![cfg_attr(not(test), no_std)]
pub mod filter;
pub mod reverb;
pub mod synthesis;

pub mod delay {
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

        pub fn get(&self, index: usize) -> f32 {
            self.inner[index % self.inner.len()]
        }

        pub fn read(&self) -> f32 {
            self.inner[self.index]
        }

        pub fn write(&mut self, input: f32) {
            self.inner[self.index] = input;
            self.index = (self.index + 1) % self.inner.len();
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
