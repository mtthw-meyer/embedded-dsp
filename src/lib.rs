#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_imports)]
pub mod filter;

mod delay {
    use core::ops::{Index, IndexMut};

    pub struct DelayLine {
        inner: &'static mut [f32],
        index: usize,
    }

    impl DelayLine {
        pub fn new(inner: &'static mut [f32]) -> DelayLine {
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

    impl Index<usize> for DelayLine {
        type Output = f32;

        fn index(&self, index: usize) -> &Self::Output {
            &self.inner[index]
        }
    }

    impl IndexMut<usize> for DelayLine {
        fn index_mut(&mut self, index: usize) -> &mut Self::Output {
            &mut self.inner[index]
        }
    }
}

// mod reverb {
//     pub struct Dattorro {}

//     impl Dattorro {}
// }
