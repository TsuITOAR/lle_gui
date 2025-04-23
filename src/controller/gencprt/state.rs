use lle::num_complex::{Complex, Complex64};
use num_traits::Zero;
use static_assertions::assert_impl_all;

use crate::FftSource;

pub type CoupleInfo = super::cprt_disper::CprtDispersionFrac;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub data: Vec<Complex64>,
    pub(crate) cp: CoupleInfo,
    #[serde(default)]
    pub(crate) time: f64,
}

impl State {
    pub(crate) fn coupling_iter_positive(&self) -> CouplingStateIterPositive<'_> {
        let cp = self.cp.clone();
        let len = self.data.len();
        let (state_a, state_b) = self.data.split_at(len / 2);
        let (state_a_p, _) = state_a.split_at(len / 4);
        let (state_b_p, _) = state_b.split_at(len / 4);
        CouplingStateIterPositive {
            state_a_p: state_a_p.into(),
            state_b_p: state_b_p.into(),
            cp,
            len,
            cur_freq: 0,
        }
    }
    pub(crate) fn coupling_iter_negative(&self) -> CouplingStateIterNegative<'_> {
        let cp = self.cp.clone();
        let len = self.data.len();
        let (state_a, state_b) = self.data.split_at(len / 2);
        let (_, state_a_n) = state_a.split_at(len / 4);
        let (_, state_b_n) = state_b.split_at(len / 4);
        CouplingStateIterNegative {
            state_a_n: state_a_n.into(),
            state_b_n: state_b_n.into(),
            cp,
            len,
            cur_freq: -1,
        }
    }

    pub(crate) fn coupling_iter_mut(
        &mut self,
    ) -> (
        CouplingStateIterMutPositive<'_>,
        CouplingStateIterMutNegative<'_>,
    ) {
        let cp = self.cp.clone();
        let len = self.data.len();
        let (state_a, state_b) = self.data.split_at_mut(len / 2);
        let (state_a_p, state_a_n) = state_a.split_at_mut(len / 4);
        let (state_b_p, state_b_n) = state_b.split_at_mut(len / 4);
        (
            CouplingStateIterMutPositive {
                state_a_p: state_a_p.into(),
                state_b_p: state_b_p.into(),
                cp: cp.clone(),
                len,
                cur_freq: 0,
            },
            CouplingStateIterMutNegative {
                state_a_n: state_a_n.into(),
                state_b_n: state_b_n.into(),
                cp,
                len,
                cur_freq: -1,
            },
        )
    }

    pub(crate) fn decoupling_iter_positive(&self) -> DecouplingStateIterPositive<'_> {
        let cp = self.cp.clone();
        let len = self.data.len();
        let (state_p, _) = self.data.split_at(len / 2);
        DecouplingStateIterPositive {
            state_p: state_p.into(),
            cp,
            len,
            cur_freq: 0,
        }
    }
    pub(crate) fn decoupling_iter_negative(&self) -> DecouplingStateIterNegative<'_> {
        let cp = self.cp.clone();
        let len = self.data.len();
        let (_, state_n) = self.data.split_at(len / 2);
        DecouplingStateIterNegative {
            state_n: state_n.into(),
            cp,
            len,
            cur_freq: -1,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModeMeta {
    pub(crate) m: i32,
    pub(crate) freq: lle::Freq,
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Single {
        amp: Complex64,
        meta: ModeMeta,
    },
    Pair {
        amp1: Complex64,
        amp2: Complex64,
        meta: ModeMeta,
    },
}

#[allow(unused)]
impl Mode {
    pub(crate) fn meta(&self) -> ModeMeta {
        match self {
            Mode::Single { meta, .. } => *meta,
            Mode::Pair { meta, .. } => *meta,
        }
    }
    pub(crate) fn m(&self) -> i32 {
        self.meta().m
    }
}

#[derive(Debug)]
pub enum ModeMut<'a> {
    Single {
        amp: Option<&'a mut Complex64>,
        meta: ModeMeta,
    },
    Pair {
        amp1: Option<&'a mut Complex64>,
        amp2: Option<&'a mut Complex64>,
        meta: ModeMeta,
    },
}

impl<'a> ModeMut<'a> {
    pub(crate) fn meta(&self) -> ModeMeta {
        match self {
            ModeMut::Single { meta, .. } => *meta,
            ModeMut::Pair { meta, .. } => *meta,
        }
    }
    pub(crate) fn m(&self) -> i32 {
        self.meta().m
    }
}

#[derive(Debug, Clone)]
pub struct CouplingStateIterPositive<'a> {
    state_a_p: MySliceIter<'a>,
    state_b_p: MySliceIter<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for CouplingStateIterPositive<'a> {
    type Item = Mode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_freq > self.len as _ {
            return None;
        }
        let real_number = self.state_a_p.cur as i32 + self.state_b_p.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq += 1;
            let amp = self.state_a_p.next().unwrap_or_default();
            Some(Mode::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq += 2;
            let amp1 = self.state_a_p.next().unwrap_or_default();
            let amp2 = self.state_b_p.next().unwrap_or_default();
            Some(Mode::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct CouplingStateIterNegative<'a> {
    state_a_n: MySliceIterRev<'a>,
    state_b_n: MySliceIterRev<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for CouplingStateIterNegative<'a> {
    type Item = Mode;

    fn next(&mut self) -> Option<Self::Item> {
        if -self.cur_freq > self.len as i32 - 1 {
            return None;
        }
        let real_number = -1i32 - self.state_a_n.cur as i32 - self.state_b_n.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq -= 1;
            let amp = self.state_a_n.next().unwrap_or_default();
            Some(Mode::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq -= 2;
            let amp1 = self.state_a_n.next().unwrap_or_default();
            let amp2 = self.state_b_n.next().unwrap_or_default();
            Some(Mode::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

#[derive(Debug)]
pub struct CouplingStateIterMutPositive<'a> {
    state_a_p: MySliceIterMut<'a>,
    state_b_p: MySliceIterMut<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for CouplingStateIterMutPositive<'a> {
    type Item = ModeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_freq > self.len as _ {
            return None;
        }
        let real_number = self.state_a_p.cur as i32 + self.state_b_p.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq += 1;
            let amp = self.state_a_p.next();
            Some(ModeMut::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq += 2;
            let amp1 = self.state_a_p.next();
            let amp2 = self.state_b_p.next();
            Some(ModeMut::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

#[derive(Debug)]
pub struct CouplingStateIterMutNegative<'a> {
    state_a_n: MySliceIterMutRev<'a>,
    state_b_n: MySliceIterMutRev<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for CouplingStateIterMutNegative<'a> {
    type Item = ModeMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if -self.cur_freq > self.len as i32 - 1 {
            return None;
        }
        let real_number = -1i32 - self.state_a_n.cur as i32 - self.state_b_n.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq -= 1;
            let amp = self.state_a_n.next();
            Some(ModeMut::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq -= 2;
            let amp1 = self.state_a_n.next();
            let amp2 = self.state_b_n.next();
            Some(ModeMut::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecouplingStateIterPositive<'a> {
    state_p: MySliceIter<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for DecouplingStateIterPositive<'a> {
    type Item = Mode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_freq > self.len as _ {
            return None;
        }
        let real_number = self.state_p.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq += 1;
            let amp = self.state_p.next().unwrap_or_default();
            Some(Mode::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq += 2;
            let amp1 = self.state_p.next().unwrap_or_default();
            let amp2 = self.state_p.next().unwrap_or_default();
            Some(Mode::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecouplingStateIterNegative<'a> {
    state_n: MySliceIterRev<'a>,
    pub(crate) cp: CoupleInfo,
    pub(crate) len: usize,
    pub(crate) cur_freq: lle::Freq,
}

impl<'a> Iterator for DecouplingStateIterNegative<'a> {
    type Item = Mode;

    fn next(&mut self) -> Option<Self::Item> {
        if -self.cur_freq > self.len as i32 - 1 {
            return None;
        }
        let real_number = -1i32 - self.state_n.cur as i32;
        let m = self.cp.m_original(real_number);
        let freq = self.cur_freq;
        if self.cp.singularity_point(real_number) {
            self.cur_freq -= 1;
            let amp = self.state_n.next().unwrap_or_default();
            Some(Mode::Single {
                amp,
                meta: ModeMeta { m, freq },
            })
        } else {
            self.cur_freq -= 2;
            let amp1 = self.state_n.next().unwrap_or_default();
            let amp2 = self.state_n.next().unwrap_or_default();
            Some(Mode::Pair {
                amp1,
                amp2,
                meta: ModeMeta { m, freq },
            })
        }
    }
}

assert_impl_all!(State:FftSource);

impl From<Vec<Complex<f64>>> for State {
    fn from(c: Vec<Complex<f64>>) -> Self {
        Self {
            data: c,
            cp: super::GenCprtDisperSubController::default().get_coup_info(),
            time: 0.0,
        }
    }
}

impl State {
    pub fn new(len: usize, cp: CoupleInfo, time: f64) -> Self {
        Self {
            data: vec![Complex::zero(); len],
            cp,
            time,
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

#[derive(Debug, Clone)]
struct MySliceIter<'a> {
    slice: &'a [Complex64],
    cur: usize,
}

impl<'a> From<&'a [Complex64]> for MySliceIter<'a> {
    fn from(slice: &'a [Complex64]) -> Self {
        Self { slice, cur: 0 }
    }
}

impl Iterator for MySliceIter<'_> {
    type Item = Complex64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.slice.len() {
            self.cur += 1;
            return None;
        }
        let item = self.slice[self.cur];
        self.cur += 1;
        Some(item)
    }
}

#[derive(Debug, Clone)]
struct MySliceIterRev<'a> {
    slice: &'a [Complex64],
    cur: usize,
}

impl<'a> From<&'a [Complex64]> for MySliceIterRev<'a> {
    fn from(slice: &'a [Complex64]) -> Self {
        Self { slice, cur: 0 }
    }
}

impl Iterator for MySliceIterRev<'_> {
    type Item = Complex64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.slice.len() {
            self.cur += 1;
            return None;
        }
        let item = self.slice[self.slice.len() - 1 - self.cur];
        self.cur += 1;
        Some(item)
    }
}

#[derive(Debug)]
struct MySliceIterMut<'a> {
    slice: &'a mut [Complex64],
    cur: usize,
}

impl<'a> From<&'a mut [Complex64]> for MySliceIterMut<'a> {
    fn from(slice: &'a mut [Complex64]) -> Self {
        Self { slice, cur: 0 }
    }
}

impl<'a> Iterator for MySliceIterMut<'a> {
    type Item = &'a mut Complex64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.slice.len() {
            self.cur += 1;
            return None;
        }
        let item = unsafe { &mut *(self.slice.as_mut_ptr().add(self.cur)) };
        self.cur += 1;
        Some(item)
    }
}

#[derive(Debug)]
struct MySliceIterMutRev<'a> {
    slice: &'a mut [Complex64],
    cur: usize,
}

impl<'a> From<&'a mut [Complex64]> for MySliceIterMutRev<'a> {
    fn from(slice: &'a mut [Complex64]) -> Self {
        Self { slice, cur: 0 }
    }
}

impl<'a> Iterator for MySliceIterMutRev<'a> {
    type Item = &'a mut Complex64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur >= self.slice.len() {
            self.cur += 1;
            return None;
        }
        let item = unsafe { &mut *(self.slice.as_mut_ptr().add(self.slice.len() - 1 - self.cur)) };
        self.cur += 1;
        Some(item)
    }
}

#[cfg(test)]
mod test {
    use core::f64;

    use crate::controller::cprt2::CoupleStrength;

    use super::*;
    #[test]
    fn test_fraction_at() {
        let c = CoupleInfo {
            couple_strength: CoupleStrength {
                couple_strength: 1.,
                decay: f64::INFINITY,
            },
            center_pos: 0.,
            period: 5.0,
            frac_d1_2pi: 100.,
        };
        use assert_approx_eq::assert_approx_eq;
        use lle::num_complex::ComplexFloat;
        let s = (Complex64::i(), Complex64::new(1., 0.));
        for i in 0..100 {
            let ((a1, b1), (a2, b2)) = c.fraction_at(i, 1, 1.1);
            assert_approx_eq!(a1, b2.conj());
            assert_approx_eq!(a2, -b1.conj());
            let ((a3, b3), (a4, b4)) = c.fraction_at(-i, -1, 1.1);
            assert_approx_eq!(a3, b4.conj());
            assert_approx_eq!(a4, -b3.conj());
            assert_approx_eq!(a1, b3);
            assert_approx_eq!(b1, a3);
            assert_approx_eq!(a2, -b4);
            assert_approx_eq!(b2, -a4);
            assert_approx_eq!(a1.norm_sqr() + a2.norm_sqr(), 1.);
            assert_approx_eq!(a1 * b1.conj() + a2 * b2.conj(), 0.);
            assert_approx_eq!(b2.norm_sqr() + b4.norm_sqr(), 1.);
            let b = (s.0 * a1 + s.1 * b1, s.0 * a2 + s.1 * b2);
            let c = (
                b.0 * a1.conj() + b.1 * a2.conj(),
                b.0 * b1.conj() + b.1 * b2.conj(),
            );
            assert_approx_eq!(c.0, s.0);
            assert_approx_eq!(c.1, s.1);
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
        assert_eq!(cp.m_original(0), 0);
        assert_eq!(cp.m_original(2 * 2), 0);
        assert_eq!(cp.m_original(3 * 2), 0);
        assert_eq!(cp.m_original(4 * 2), 1);
        assert_eq!(cp.m_original(7 * 2), 1);
        assert_eq!(cp.m_original(8 * 2), 1);
        assert_eq!(cp.m_original(-8 * 2), -2);
    }
    #[test]
    fn test_state_iter() {
        let data = DATA;
        let state = State {
            data: data.to_vec(),
            cp: CoupleInfo {
                couple_strength: Default::default(),
                center_pos: 1.5,
                period: 7.1,
                frac_d1_2pi: 2.1,
            },
            time: 0.,
        };
        let de_freq_pos = state.decoupling_iter_positive();
        let de_freq_neg = state.decoupling_iter_negative();
        let freq_pos = state.coupling_iter_positive();
        let freq_neg = state.coupling_iter_negative();

        freq_pos
            .zip(de_freq_pos)
            .enumerate()
            .for_each(|(i, (a, b))| {
                assert_eq!(a.meta().m, b.meta().m);
                assert_eq!(a.meta().freq, b.meta().freq);
                match (a, b) {
                    (Mode::Single { .. }, Mode::Single { .. }) => {}
                    (Mode::Pair { .. }, Mode::Pair { .. }) => {}
                    _ => {
                        panic!("mismatch at {i}\n a: {a:#?}\n b: {b:#?}");
                    }
                }
            });
        freq_neg
            .zip(de_freq_neg)
            .enumerate()
            .for_each(|(i, (a, b))| {
                assert_eq!(a.meta().m, b.meta().m);
                assert_eq!(a.meta().freq, b.meta().freq);
                match (a, b) {
                    (Mode::Single { .. }, Mode::Single { .. }) => {}
                    (Mode::Pair { .. }, Mode::Pair { .. }) => {}
                    _ => {
                        panic!("mismatch at {i}\n a: {a:#?}\n b: {b:#?}");
                    }
                }
            });
    }
    const DATA: [Complex64; 32] = [
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(3., 0.),
        Complex64::new(4., 0.),
        Complex64::new(5., 0.),
        Complex64::new(6., 0.),
        Complex64::new(7., 0.),
        Complex64::new(8., 0.),
        Complex64::new(9., 0.),
        Complex64::new(10., 0.),
        Complex64::new(11., 0.),
        Complex64::new(12., 0.),
        Complex64::new(13., 0.),
        Complex64::new(14., 0.),
        Complex64::new(15., 0.),
        Complex64::new(16., 0.),
        Complex64::new(17., 0.),
        Complex64::new(18., 0.),
        Complex64::new(19., 0.),
        Complex64::new(20., 0.),
        Complex64::new(21., 0.),
        Complex64::new(22., 0.),
        Complex64::new(23., 0.),
        Complex64::new(24., 0.),
        Complex64::new(25., 0.),
        Complex64::new(26., 0.),
        Complex64::new(27., 0.),
        Complex64::new(28., 0.),
        Complex64::new(29., 0.),
        Complex64::new(30., 0.),
        Complex64::new(31., 0.),
        Complex64::new(32., 0.),
    ];
}
