use std::f64::{self, consts::PI};

use lle::num_complex::Complex;
use num_traits::Zero;
use static_assertions::assert_impl_all;

use crate::FftSource;

use super::singularity_point;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoupleInfo {
    pub g: f64,
    pub mu: f64,
    pub center: f64,
    pub period: f64,
}

impl CoupleInfo {
    pub fn m(&self, freq: i32) -> i32 {
        let freq = (freq as f64) - self.center;
        let offset = freq + freq.signum() * self.period / 4.;
        (offset / (self.period / 2.)).trunc() as i32
    }

    pub fn fraction_at(&self, mode: i32) -> ((f64, f64), (f64, f64)) {
        let m = mode as f64;

        let phi_m = 2. * PI * (m - self.center) / self.period;
        let alpha = (self.g.cos() * phi_m.cos()).acos();
        let sign = if self.m(mode) % 2 == 0 { 1. } else { -1. };
        //let denominator = 2. * alpha.sin() * phi_m.cos();
        let cp_angle = f64::atan2(
            (alpha + phi_m).sin().abs().sqrt(),
            (alpha - phi_m).sin().abs().sqrt() * sign,
        );
        let ret = (
            (cp_angle.cos(), -cp_angle.sin()),
            (cp_angle.sin(), cp_angle.cos()),
        );
        if singularity_point(mode, self.center, self.period) {
            let (a, b) = if ret.0 .0.abs() > ret.0 .1.abs() {
                (1., 0.)
            } else {
                (0., 1.)
            };
            (
                (ret.0 .0.signum() * a, ret.0 .1.signum() * b),
                (ret.1 .0.signum() * b, ret.1 .1.signum() * a),
            )
        } else {
            ret
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
            let ((a1, b1), (a2, b2)) = c.fraction_at(i * 2);
            assert_approx_eq!(a1, b2);
            assert_approx_eq!(a2, -b1);
            let ((a3, b3), (a4, b4)) = c.fraction_at(-i * 2);
            assert_approx_eq!(a3, b4);
            assert_approx_eq!(a4, -b3);
            assert_approx_eq!(a1, -b3);
            assert_approx_eq!(b1, -a3);
            assert_approx_eq!(a2, b4);
            assert_approx_eq!(b2, a4);
        }
    }
    #[test]
    fn test_m() {
        let cp = CoupleInfo {
            g: 0.5,
            mu: 0.5,
            center: 0.,
            period: 10.0,
        };
        assert_eq!(cp.m(0), 0);
        assert_eq!(cp.m(2), 0);
        assert_eq!(cp.m(3), 1);
        assert_eq!(cp.m(4), 1);
        assert_eq!(cp.m(7), 1);
        assert_eq!(cp.m(8), 2);
        assert_eq!(cp.m(-8), -2);
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
