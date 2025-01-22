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

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_fraction_at() {
        let c = CoupleInfo {
            g: 0.5,
            mu: 0.5,
            center: 0.5,
            period: 1.0,
        };
        use assert_approx_eq::assert_approx_eq;
        for i in 0..100 {
            let (a1, b1) = c.fraction_at(i * 2);
            let (a2, b2) = c.fraction_at(i * 2 + 1);
            assert_approx_eq!(a1, b2);
            assert_approx_eq!(a2, -b1);
            let (a3, b3) = c.fraction_at(-i * 2);
            let (a4, b4) = c.fraction_at(-i * 2 + 1);
            assert_approx_eq!(a3, b4);
            assert_approx_eq!(a4, -b3);

            assert_approx_eq!(a1, -b3);
            assert_approx_eq!(b1, -a3);
            assert_approx_eq!(a2, b4);
            assert_approx_eq!(b2, a4);
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
