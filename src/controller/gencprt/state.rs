use lle::num_complex::Complex;
use num_traits::Zero;
use static_assertions::assert_impl_all;

use crate::FftSource;

pub type CoupleInfo = super::cprt_disper::CprtDispersionFrac;

#[cfg(test)]
mod test {

    use crate::controller::cprt2::CoupleStrength;

    use super::*;
    #[test]
    fn test_fraction_at() {
        let c = CoupleInfo {
            couple_strength: CoupleStrength::default(),
            center_pos: 0.,
            period: 1.0,
            frac_d1_2pi: 0.5,
        };
        use assert_approx_eq::assert_approx_eq;
        for i in 0..100 {
            let ((a1, b1), (a2, b2)) = c.fraction_at(i);
            assert_approx_eq!(a1, b2);
            assert_approx_eq!(a2, -b1);
            let ((a3, b3), (a4, b4)) = c.fraction_at(-i);
            assert_approx_eq!(a3, b4);
            assert_approx_eq!(a4, -b3);
            assert_approx_eq!(a1, b3);
            assert_approx_eq!(b1, a3);
            assert_approx_eq!(a2, -b4);
            assert_approx_eq!(b2, -a4);
            assert_approx_eq!(a1.powi(2) + a2.powi(2), 1.);
            assert_approx_eq!(a1 * b1 + a2 * b2, 0.);
            assert_approx_eq!(b2.powi(2) + b2.powi(2), 1.);
        }
    }
    #[test]
    fn test_m() {
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 1.5,
            period: 10.0,
            frac_d1_2pi: 0.5,
        };
        assert_eq!(cp.m_original(0 * 2), 0);
        assert_eq!(cp.m_original(2 * 2), 0);
        assert_eq!(cp.m_original(3 * 2), 1);
        assert_eq!(cp.m_original(4 * 2), 1);
        assert_eq!(cp.m_original(7 * 2), 1);
        assert_eq!(cp.m_original(8 * 2), 2);
        assert_eq!(cp.m_original(-8 * 2), -2);
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
