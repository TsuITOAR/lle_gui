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

macro_rules! assert_is_zero {
    ($x:expr) => {
        #[cfg(debug_assertions)]
        {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!($x.re, 0.);
            assert_approx_eq!($x.im, 0.);
        }
    };
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
        let period = self.cp.period;
        let center = self.cp.center;
        let split = (len / 2).div_ceil(2);
        let (s2_p, s2_n) = s2.split_at(split);

        let s2_p = s2_p.iter().enumerate().flat_map(|(i, x)| {
            let freq = i as i32;
            if singularity_point(freq, center, period) {
                [Some(Complex64::zero()), Some(*x)].into_iter().flatten()
            } else {
                [Some(*x), None].into_iter().flatten()
            }
        });
        let s2_n = s2_n.iter().rev().enumerate().flat_map(|(i, x)| {
            let freq = -(i as i32) - 1;
            if singularity_point(freq, center, period) {
                [Some(Complex64::zero()), Some(*x)].into_iter().flatten()
            } else {
                [Some(*x), None].into_iter().flatten()
            }
        });
        let s2_n = s2_n.collect::<Vec<_>>();

        let s2 = s2_p
            .take(split)
            .chain(s2_n[0..split].iter().rev().copied())
            .collect::<Vec<_>>();
        let mut new = vec![Complex64::zero(); len];
        coupling_modes(s1, &s2, &mut new, &self.cp);
        self.data.copy_from_slice(&new);
    }

    fn fft_process_inverse(&mut self, fft: &mut Self::FftProcessor) {
        let len = self.data.len();

        let mut new_a = vec![Complex64::zero(); len / 2];
        let mut new_b = vec![Complex64::zero(); len / 2];
        decoupling_modes(&self.data, &mut new_a, &mut new_b, &self.cp);

        let split = (len / 2).div_ceil(2);
        let period = self.cp.period;
        let center = self.cp.center;

        let mut real_data = vec![Complex64::zero(); len];

        let (real_a, real_b) = real_data.split_at_mut(len / 2);
        real_a.copy_from_slice(new_a.as_slice());

        let (real_b_p, real_b_n) = real_b.split_at_mut(split);
        let mut i = 0;
        let mut skipped = false;

        let (new_b_p, new_b_n) = new_b.split_at(split);
        for p in new_b_p.iter() {
            let freq = i as i32;
            if !skipped && singularity_point(freq, center, period) {
                assert_is_zero!(*p);
                skipped = true;
                continue;
            } else {
                skipped = false;
                real_b_p[i] = *p;
                i += 1;
            }
        }
        let mut i = 0;
        let mut skipped = false;
        for n in new_b_n.iter().rev() {
            let freq = -(i as i32) - 1;
            if !skipped && singularity_point(freq, center, period) {
                assert_is_zero!(*n);
                skipped = true;
                continue;
            } else {
                skipped = false;
                real_b_n[split - 1 - i] = *n;
                i += 1;
            }
        }

        self.data.copy_from_slice(&real_data);
        let (s1, s2) = self.data.split_at_mut(len / 2);
        fft.fft.1.fft_process(s1);
        fft.fft.1.fft_process(s2);
    }
}

fn coupling_modes(
    a: &[Complex64],
    b: &[Complex64],
    dst: &mut [Complex64],
    cp: &super::state::CoupleInfo,
) {
    let len = a.len();
    assert_eq!(len, b.len());
    assert!(len % 2 == 0);
    assert_eq!(dst.len(), len * 2);
    let (new_p, new_n) = dst.split_at_mut(len);
    let (a_p, a_n) = a.split_at(len / 2);
    let (b_p, b_n) = b.split_at(len / 2);
    a_p.iter()
        .zip(b_p)
        .zip(new_p.array_chunks_mut::<2>())
        .enumerate()
        .for_each(|(i, ((a, b), dst))| {
            let freq = i as i32;
            let (frac1, frac2) = cp.fraction_at(freq);
            dst[0] = *a * frac1.0 + b * frac1.1;
            dst[1] = *a * frac2.0 + b * frac2.1;
        });
    a_n.iter()
        .rev()
        .zip(b_n.iter().rev())
        .zip(new_n.array_chunks_mut::<2>().rev())
        .enumerate()
        .for_each(|(i, ((a, b), dst))| {
            let freq = -(i as i32) - 1;
            let (frac1, frac2) = cp.fraction_at(freq);
            dst[0] = *a * frac1.0 + b * frac1.1;
            dst[1] = *a * frac2.0 + b * frac2.1;
        });
}

fn decoupling_modes(
    src: &[Complex64],
    a: &mut [Complex64],
    b: &mut [Complex64],
    cp: &super::state::CoupleInfo,
) {
    let len = a.len();
    assert!(len == b.len());
    assert!(len % 2 == 0);
    assert!(src.len() == len * 2);
    let split = len.div_ceil(2);
    let (a_p, a_n) = a.split_at_mut(split);
    let (b_p, b_n) = b.split_at_mut(split);

    let (src_p, src_n) = src.split_at(len);

    src_p
        .array_chunks::<2>()
        .zip(a_p.iter_mut().zip(b_p.iter_mut()))
        .enumerate()
        .for_each(|(i, (chunk, (a, b)))| {
            let freq = i as i32;
            let (frac1, frac2) = cp.fraction_at(freq);
            *a = chunk[0] * frac1.0 + chunk[1] * frac2.0;
            *b = chunk[0] * frac1.1 + chunk[1] * frac2.1;
        });
    src_n
        .array_chunks::<2>()
        .rev()
        .zip(a_n.iter_mut().rev().zip(b_n.iter_mut().rev()))
        .enumerate()
        .for_each(|(i, (chunk, (a, b)))| {
            let freq = -(i as i32) - 1;
            let (frac1, frac2) = cp.fraction_at(freq);
            *a = chunk[0] * frac1.0 + chunk[1] * frac2.0;
            *b = chunk[0] * frac1.1 + chunk[1] * frac2.1;
        });
}
#[allow(unused_variables)]
#[cfg(test)]
mod test {

    const DATA: [Complex64; 32] = [
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 0.),
        Complex64::new(1., 0.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 0.),
        Complex64::new(1., 0.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 0.),
        Complex64::new(3., 0.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
        Complex64::new(1., 0.),
        Complex64::new(2., 0.),
        Complex64::new(4., 0.),
        Complex64::new(1., 0.),
        Complex64::new(1., 1.),
        Complex64::new(2., 3.),
        Complex64::new(1., 4.),
        Complex64::new(1., 4.),
    ];

    use lle::num_complex::Complex64;
    use num_traits::Zero;

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
            g: 0.5,
            mu: 0.5,
            center: 1.5,
            period: 5.,
        };
        let data_sample = data;
        let (a, b) = data.split_at(data.len() / 2);
        let mut new = vec![Complex64::zero(); data.len()];
        coupling_modes(a, b, &mut new, &cp);
        let mut buf = vec![Complex64::zero(); data.len()];
        let (a, b) = buf.split_at_mut(data.len() / 2);
        decoupling_modes(&new, a, b, &cp);

        for (a, b) in data_sample.iter().zip(buf.iter()) {
            use assert_approx_eq::assert_approx_eq;
            assert_approx_eq!(a.re, b.re);
            assert_approx_eq!(a.im, b.im);
        }
    }

    #[test]
    fn fft_consistent() {
        let data = DATA;
        let cp = CoupleInfo {
            g: 0.5,
            mu: 0.5,
            center: 1.5,
            period: 20.,
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
                g: 0.5,
                mu: 0.5,
                center: 1.5,
                period: 5.1,
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
}
