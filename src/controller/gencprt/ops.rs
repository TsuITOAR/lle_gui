use lle::{freq_at, num_complex::Complex64, Step};
use num_traits::Zero;

use super::{singularity_point, state::State};

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
        coupling_modes(s1, s2, &self.cp);
    }

    fn fft_process_inverse(&mut self, fft: &mut Self::FftProcessor) {
        let len = self.data.len();
        let (f_p, f_n) = self.data.split_at_mut(len / 2);
        decoupling_modes(f_p, f_n, &self.cp);
        fft.fft.1.fft_process(f_p);
        fft.fft.1.fft_process(f_n);
    }
}

fn coupling_modes(s1: &mut [Complex64], s2: &mut [Complex64], cp: &super::state::CoupleInfo) {
    let s1t = &*s1;
    let s2t = &*s2;
    let mut i = 0;
    let mut m_f = 0;
    let len = s1t.len();
    let mut new_p = Vec::with_capacity(len);
    let mut new_n = Vec::with_capacity(len);
    while i < len / 2 {
        let j = i - m_f;
        let freq = lle::freq_at(len, i);
        debug_assert!(freq >= 0);
        let (a, b) = (s1t[i], s2t[j]);
        if singularity_point(freq, cp.center_pos, cp.period) {
            new_p.push(a);
            m_f += 1;
        } else {
            let (frac1, frac2) = cp.fraction_at(freq);
            let a1 = a * frac1.0 + b * frac1.1;
            let b1 = a * frac2.0 + b * frac2.1;
            new_p.push(a1);
            new_p.push(b1);
        }
        i += 1;
    }
    let mut i = s1t.len() - 1;
    let mut m_b = 0;
    let len = s1t.len();
    while i >= len / 2 {
        let j = i + m_b;
        let freq = lle::freq_at(len, i);
        debug_assert!(freq.is_negative());
        let (a, b) = (s1t[i], s2t[j]);
        if singularity_point(freq, cp.center_pos, cp.period) {
            new_n.push(a);
            m_b += 1;
        } else {
            let (frac1, frac2) = cp.fraction_at(freq);
            let a1 = a * frac1.0 + b * frac1.1;
            let b1 = a * frac2.0 + b * frac2.1;
            new_n.push(b1);
            new_n.push(a1);
        }
        i -= 1;
    }
    new_p.extend_from_slice(&s2[(len / 2 - m_f)..len / 2]);
    new_n.extend(s2[(len / 2)..(len / 2 + m_b)].iter().rev().copied());
    debug_assert!(new_p.len() == len);
    debug_assert!(new_n.len() == len);
    s1.copy_from_slice(&new_p);
    s2.iter_mut()
        .rev()
        .zip(new_n)
        .for_each(|(a, b)| *a = b);
}

fn decoupling_modes(f_p: &mut [Complex64], f_n: &mut [Complex64], cp: &super::state::CoupleInfo) {
    let len = f_p.len();
    let tp = f_p.to_vec();
    let tn = f_n.to_vec();
    let mut tp = tp.into_iter();
    let mut tn = tn.into_iter().rev();
    let mut i = 0;
    let mut m_f = 0;
    while i < len / 2 {
        let j = i - m_f;
        let freq = lle::freq_at(len, i);
        debug_assert!(freq >= 0);
        let (a, b) = (&mut f_p[i], &mut f_n[j]);
        if singularity_point(freq, cp.center_pos, cp.period) {
            *a = tp.next().unwrap();
            m_f += 1;
        } else {
            let (frac1, frac2) = cp.fraction_at(freq);
            let (a1, b1) = (tp.next().unwrap(), tp.next().unwrap());
            *a = a1 * frac1.0 + b1 * frac2.0;
            *b = a1 * frac1.1 + b1 * frac2.1;
        }
        i += 1;
    }
    let mut i = len - 1;
    let mut m_b = 0;
    while i >= len / 2 {
        let j = i + m_b;
        let freq = lle::freq_at(len, i);
        debug_assert!(freq.is_negative());
        let (a, b) = (&mut f_p[i], &mut f_n[j]);
        if singularity_point(freq, cp.center_pos, cp.period) {
            *a = tn.next().unwrap();
            m_b += 1;
        } else {
            let (frac1, frac2) = cp.fraction_at(freq);
            let (b1, a1) = (tn.next().unwrap(), tn.next().unwrap());
            *a = a1 * frac1.0 + b1 * frac2.0;
            *b = a1 * frac1.1 + b1 * frac2.1;
        }
        i -= 1;
    }
    f_n[(len / 2 - m_f)..len / 2]
        .iter_mut()
        .zip(tp.by_ref())
        .for_each(|x| *x.0 = x.1);
    f_n[(len / 2)..(len / 2 + m_b)]
        .iter_mut()
        .rev()
        .zip(tn.by_ref())
        .for_each(|x| *x.0 = x.1);
    debug_assert!(tp.next().is_none());
    debug_assert!(tn.next().is_none());
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
        let mut data = DATA;
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 1.5,
            period: 5.,
            frac_d1_2pi: 0.5,
        };
        let data_sample = data;
        let len = data.len();
        let (a, b) = data.split_at_mut(len / 2);
        coupling_modes(a, b, &cp);
        decoupling_modes(a, b, &cp);

        for (a, b) in data_sample.iter().zip(data.iter()) {
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
        };
        let mut fft = State::default_fft(state.fft_len());
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        let data = state.data.clone();
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        let scale = state.scale_factor();
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        for (a, b) in state.data.iter().zip(data.iter()) {
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
        };
        let mut fft = State::default_fft(state.fft_len());
        let scale = state.scale_factor();
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        let data = state.data.clone();
        state.fft_process_forward(&mut fft);
        state.fft_process_inverse(&mut fft);
        state.as_mut().iter_mut().for_each(|x| *x /= scale);
        for (a, b) in state.data.iter().zip(data.iter()) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        }
        let step_dist = 1e-4;
        let cur_step = 0;
        state.fft_process_forward(&mut fft);
        let linear: (lle::DiffOrder, Complex64) = (2, Complex64::i() * 0.5 / 2. / 4.);
        let backup = state.data.clone();
        linear.apply_freq(state.as_mut(), step_dist, cur_step);

        state.fft_process_inverse(&mut fft);
        /* state.as_mut().iter_mut().for_each(|x| *x /= scale);
        for (a, b) in state.data.iter().zip(data.iter()) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        } */
    }
    const DATA: [Complex64; 32] = [
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 0.),
        Complex64::new(1., 0.),
        Complex64::new(3., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 2.),
        Complex64::new(1., 4.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(6., 4.),
        Complex64::new(1., 0.),
        Complex64::new(1., -8.),
        Complex64::new(2., -5.),
        Complex64::new(4., 2.),
        Complex64::new(3., 4.),
        Complex64::new(2., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
        Complex64::new(1., -3.),
        Complex64::new(2., 0.),
        Complex64::new(4., 7.),
        Complex64::new(1., 0.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
    ];
}
