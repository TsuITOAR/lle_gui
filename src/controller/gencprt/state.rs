use std::f64::consts::PI;

use lle::num_complex::Complex;
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
        let branch = mode.rem_euclid(2);
        debug_assert!(branch == 0 || branch == 1);
        let m = mode.div_euclid(2) as f64;
        let phi_m = 2. * PI * (m - self.center) / self.period;
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
