use std::{f64::consts::PI, ops::Rem};

use lle::{freq_at, num_complex::Complex};
use num_traits::Zero;
use static_assertions::assert_impl_all;

use crate::FftSource;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoupleInfo {
    pub g: f64,
    pub mu: f64,
    pub center: f64,
    pub period: f64,
}

impl CoupleInfo {
    pub fn fraction_at(&self, mode: i32) -> (f64, f64) {
        let branch = mode.rem(2);
        let m = mode.div_euclid(2) as f64;
        let phi_m = 2. * PI * m / self.period;
        let alpha = (self.g.cos() * phi_m.cos()).acos();

        let denominator = 2. * alpha.sin() * phi_m.cos();
        if branch == 0 {
            (
                ((alpha + phi_m).sin() / denominator).sqrt(),
                -((alpha - phi_m).sin() / denominator).sqrt(),
            )
        } else {
            (
                ((alpha - phi_m).sin() / denominator).sqrt(),
                ((alpha + phi_m).sin() / denominator).sqrt(),
            )
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub data: Vec<Complex<f64>>,
    pub(crate) cp: CoupleInfo,
}

assert_impl_all!(State:FftSource);

impl From<Vec<Complex<f64>>> for State {
    fn from(c: Vec<Complex<f64>>) -> Self {
        Self {
            data: c,
            cp: super::GenCprtDisperSubController::default().get_coup_info(),
        }
    }
}

impl State {
    pub fn new(len: usize, cp: CoupleInfo) -> Self {
        Self {
            data: vec![Complex::zero(); len],
            cp,
        }
    }
}

impl AsRef<[Complex<f64>]> for State {
    fn as_ref(&self) -> &[Complex<f64>] {
        &self.data
    }
}

impl AsMut<[Complex<f64>]> for State {
    fn as_mut(&mut self) -> &mut [Complex<f64>] {
        &mut self.data
    }
}

pub struct CprtFft {
    fft: (lle::BufferedFft<f64>, lle::BufferedFft<f64>),
}

impl lle::FftSource<f64> for State {
    type FftProcessor = CprtFft;
    fn fft_len(&self) -> usize {
        debug_assert!(self.data.len() % 2 == 0);
        self.data.len() / 2
    }
    fn default_fft(len: usize) -> Self::FftProcessor {
        let fft = lle::BufferedFft::new(len);
        CprtFft { fft }
    }
    fn scale_factor(&self) -> f64 {
        self.data.len() as f64 / 2.
    }
    fn fft_process_forward(&mut self, fft: &mut Self::FftProcessor) {
        let len = self.data.len();
        let (s1, s2) = self.data.split_at_mut(len / 2);
        fft.fft.0.fft_process(s1);
        fft.fft.0.fft_process(s2);
        let mut new = vec![Complex::zero(); len];
        s1.iter()
            .zip(s2.iter())
            .enumerate()
            .for_each(|(i, (a, b))| {
                let freq = freq_at(len / 2, i);
                let frac1 = self.cp.fraction_at(freq * 2);
                let frac2 = self.cp.fraction_at(freq * 2 + 1);
                new[2 * i] = *a * frac1.0 + *b * frac1.1;
                new[2 * i + 1] = *a * frac2.0 + *b * frac2.1;
            });
        self.data.copy_from_slice(&new);
    }
    fn fft_process_inverse(&mut self, fft: &mut Self::FftProcessor) {
        let len = self.data.len();
        let mut new = vec![Complex::zero(); len];
        self.data
            .array_chunks::<2>()
            .enumerate()
            .for_each(|(i, chunk)| {
                let freq = freq_at(len / 2, i);
                let frac1 = self.cp.fraction_at(freq * 2);
                let frac2 = self.cp.fraction_at(freq * 2 + 1);
                new[i] = chunk[0] * frac1.0 + chunk[1] * frac2.0;
                new[i + len / 2] = chunk[0] * frac1.1 + chunk[1] * frac2.1;
            });
        self.data.copy_from_slice(&new);
        let (s1, s2) = self.data.split_at_mut(len / 2);
        fft.fft.1.fft_process(s1);
        fft.fft.1.fft_process(s2);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn fft_consistent() {
        let data = vec![
            Complex::new(1., 0.),
            Complex::new(2., 0.),
            Complex::new(4., 0.),
            Complex::new(1., 0.),
            Complex::new(1., 1.),
            Complex::new(2., 3.),
            Complex::new(1., 4.),
            Complex::new(1., 4.),
        ];
        let cp = CoupleInfo {
            g: 0.5,
            mu: 0.5,
            center: 0.5,
            period: 3.,
        };
        use lle::FftSource;
        let mut state = State {
            data: data.clone(),
            cp,
        };
        let mut fft = State::default_fft(state.fft_len());
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        let scale = state.scale_factor();
        for (a, b) in state.data.iter().zip(data.iter()) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re * scale);
            assert_approx_eq!(a.im, b.im * scale);
        }
    }
}
