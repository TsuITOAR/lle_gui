use std::f64::consts::TAU;

use lle::{freq_at, num_complex::Complex64, Step};
use num_traits::Zero;

use crate::controller::gencprt::{cprt_disper::spatial_basis_move, state::Mode};

use super::state::State;

#[derive(Debug, Clone, serde::Serialize, serde:: Deserialize)]
pub struct PumpFreq {
    pub mode: i32,
    pub amp: f64,
}

impl lle::ConstOp<f64> for PumpFreq {
    fn skip(&self) -> bool {
        self.amp.is_zero()
    }
    fn get_value(&self, _cur_step: Step, pos: usize, state: &[Complex64]) -> Complex64 {
        let mut res = Complex64::zero();
        if self.mode == freq_at(state.len(), pos) as _ {
            res = Complex64::new(self.amp, 0.);
        }
        res
    }
    fn apply_const_op(
        &self,
        state: &mut [lle::num_complex::Complex<f64>],
        _cur_step: Step,
        step_dist: f64,
    ) {
        use lle::index_at;
        let len = state.len();
        state[index_at(len, self.mode)] += self.amp * len as f64 * step_dist;
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
        coupling_modes(self);
    }

    fn fft_process_inverse(&mut self, fft: &mut Self::FftProcessor) {
        let len = self.data.len();
        decoupling_modes(self);
        let (f_p, f_n) = self.data.split_at_mut(len / 2);
        fft.fft.1.fft_process(f_p);
        fft.fft.1.fft_process(f_n);
    }
}

pub(crate) fn coupling_modes(state: &mut State) {
    let time = state.time;
    let freq_pos = state.coupling_iter_positive();
    let freq_neg = state.coupling_iter_negative();

    let freq_pos: Vec<_> = freq_pos
        .flat_map(|x| match x {
            Mode::Single { amp, meta } => {
                let basis_move = spatial_basis_move(meta.m, state.cp.frac_d1_2pi * TAU, time);
                let amp = amp * basis_move;
                [Some(amp), None]
            }
            Mode::Pair { amp1, amp2, meta } => {
                let (frac1, frac2) = state.cp.fraction_at((meta.freq + meta.m) / 2, time);

                let a = amp1 * frac1.0 + amp2 * frac1.1;
                let b = amp1 * frac2.0 + amp2 * frac2.1;

                // let a = amp1;
                // let b = amp2;
                [Some(a), Some(b)]
            }
        })
        .flatten()
        .collect();

    let freq_neg: Vec<_> = freq_neg
        .flat_map(|x| match x {
            Mode::Single { amp, meta } => {
                let basis_move = spatial_basis_move(meta.m, state.cp.frac_d1_2pi * TAU, time);
                let amp = amp * basis_move.conj();
                [Some(amp), None]
            }
            Mode::Pair { amp1, amp2, meta } => {
                let (frac1, frac2) = state.cp.fraction_at((meta.freq + meta.m) / 2, time);
                let f1 = amp1;
                let f2 = amp2;
                let a = f1 * frac1.0 + f2 * frac1.1;
                let b = f1 * frac2.0 + f2 * frac2.1;
                // let a = f1;
                // let b = f2;
                [Some(b), Some(a)]
            }
        })
        .flatten()
        .collect();

    let mut ret = vec![Complex64::zero(); state.data.len()];

    let (ret_p, ret_n) = ret.split_at_mut(state.data.len() / 2);

    ret_p
        .iter_mut()
        .zip(freq_pos.iter())
        .for_each(|(a, b)| *a = *b);

    ret_n
        .iter_mut()
        .rev()
        .zip(freq_neg.iter())
        .for_each(|(a, b)| *a = *b);
    state.data.copy_from_slice(&ret);
}

pub(crate) fn decoupling_modes(state: &mut State) {
    let time = state.time;
    let freq_pos = state.decoupling_iter_positive();
    let freq_neg = state.decoupling_iter_negative();

    let (freq_pos_a, freq_pos_b): (Vec<_>, Vec<_>) = freq_pos
        .map(|x| match x {
            Mode::Single { amp, meta } => {
                let basis_move = spatial_basis_move(meta.m, state.cp.frac_d1_2pi * TAU, time);
                let amp = amp * basis_move.conj();
                (Some(amp), None)
            }
            Mode::Pair { amp1, amp2, meta } => {
                let (frac1, frac2) = state.cp.fraction_at((meta.freq + meta.m) / 2, time);

                let a = amp1 * frac1.0 + amp2 * frac2.0;
                let b = amp1 * frac1.1 + amp2 * frac2.1;
                // let a = amp1;
                // let b = amp2;
                (Some(a), Some(b))
            }
        })
        .unzip();

    let (freq_neg_a, freq_neg_b): (Vec<_>, Vec<_>) = freq_neg
        .map(|x| match x {
            Mode::Single { amp, meta } => {
                let basis_move = spatial_basis_move(meta.m, state.cp.frac_d1_2pi * TAU, time);
                let amp = amp * basis_move.conj();
                (Some(amp), None)
            }
            // amp2 freq lower than amp1
            Mode::Pair { amp1, amp2, meta } => {
                let (frac1, frac2) = state.cp.fraction_at((meta.freq + meta.m) / 2, time);
                let f1 = amp2;
                let f2 = amp1;
                let a = f1 * frac1.0.conj() + f2 * frac2.0.conj();
                let b = f1 * frac1.1.conj() + f2 * frac2.1.conj();

                // let a = f1;
                // let b = f2;

                (Some(a), Some(b))
            }
        })
        .unzip();

    let mut ret = vec![Complex64::zero(); state.data.len()];
    let (ret_a, ret_b) = ret.split_at_mut(state.data.len() / 2);
    let (ret_a_p, ret_a_n) = ret_a.split_at_mut(state.data.len() / 4);
    let (ret_b_p, ret_b_n) = ret_b.split_at_mut(state.data.len() / 4);
    ret_a_p
        .iter_mut()
        .zip(freq_pos_a.iter().flatten())
        .for_each(|(a, b)| *a = *b);
    ret_a_n
        .iter_mut()
        .rev()
        .zip(freq_neg_a.iter().flatten())
        .for_each(|(a, b)| *a = *b);
    ret_b_p
        .iter_mut()
        .zip(freq_pos_b.iter().flatten())
        .for_each(|(a, b)| *a = *b);
    ret_b_n
        .iter_mut()
        .rev()
        .zip(freq_neg_b.iter().flatten())
        .for_each(|(a, b)| *a = *b);
    state.data.copy_from_slice(&ret);
}

#[allow(unused_variables)]
#[cfg(test)]
mod test {

    use lle::num_complex::Complex64;

    use crate::controller::gencprt::state::{CoupleInfo, State};

    use super::{coupling_modes, decoupling_modes};

    #[test]
    fn freq_pump_scale_check() {
        let mut state = [Complex64::ZERO; 4];
        let pump = 0.1;
        let mut fft = lle::BufferedFft::new(4);
        fft.0.fft_process(&mut state);
        state[0] += pump * state.len() as f64;
        fft.1.fft_process(&mut state);
        use assert_approx_eq::assert_approx_eq;
        assert_approx_eq!((state[0] / state.len() as f64).re, pump);
        assert_approx_eq!((state[0] / state.len() as f64).im, 0.);
    }

    #[test]
    fn coupling_decoupling() {
        let data = DATA;
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 1.5,
            period: 5.,
            frac_d1_2pi: 0.5,
        };
        let len = data.len();
        let mut state = State {
            data: data.to_vec(),
            cp: cp.clone(),
            time: 0.,
        };
        coupling_modes(&mut state);
        decoupling_modes(&mut state);
        state
            .data
            .iter()
            .zip(data.iter())
            .enumerate()
            .for_each(|(i, (a, b))| {
                println!("{i}\t {a:08}, {b:08} ");
            });
        for (a, b) in state.data.iter().zip(data.iter()).take(len / 2) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        }
    }

    #[test]
    fn fft_consistent() {
        let data = DATA;
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 1.5,
            period: 20.,
            frac_d1_2pi: 0.5,
        };
        use lle::FftSource;
        let mut state = State {
            data: data.to_vec(),
            cp: cp.clone(),
            time: 0.,
        };
        let mut fft = State::default_fft(state.fft_len());
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        let data = state.data.clone();
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        let scale = state.scale_factor();
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        for (a, b) in state.data.iter().zip(data.iter()).take(data.len() / 2) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        }
    }

    #[test]
    fn fft_linear() {
        use lle::FftSource;
        use lle::LinearOpExt;
        let data = DATA;
        let mut state = State {
            data: data.to_vec(),
            cp: CoupleInfo {
                couple_strength: Default::default(),
                center_pos: 1.5,
                period: 5.1,
                frac_d1_2pi: 0.5,
            },
            time: 1.,
        };
        let mut fft = State::default_fft(state.fft_len());
        let scale = state.scale_factor();
        let data = state.data.clone();
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        state
            .data
            .iter()
            .zip(data.iter())
            .enumerate()
            .for_each(|(i, (a, b))| {
                println!("{i}\t {a:08}, {b:08} ");
            });
        for (a, b) in state.data.iter().zip(data.iter()).take(data.len() / 2) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        }
        let step_dist = 1e-8;
        let cur_step = 0;
        let data = state.data.clone();
        state.fft_process_forward(&mut fft);
        let linear: (lle::DiffOrder, Complex64) = (2, Complex64::i() * 0.5 / 2. / 4.);
        linear.apply_freq(state.as_mut(), step_dist, cur_step);

        state.fft_process_inverse(&mut fft);
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        for (a, b) in state.data.iter().zip(data.iter()).take(data.len() / 2) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.norm(), b.norm());
        }
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
