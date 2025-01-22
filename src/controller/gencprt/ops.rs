use lle::{freq_at, num_complex::Complex64, Step};
use num_traits::Zero;

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
        let mut new = vec![Complex64::zero(); len];
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
        let mut new = vec![Complex64::zero(); len];
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
    use lle::num_complex::Complex64;

    use crate::controller::gencprt::state::{CoupleInfo, State};

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
    fn fft_consistent() {
        let data = vec![
            Complex64::new(1., 0.),
            Complex64::new(2., 0.),
            Complex64::new(4., 0.),
            Complex64::new(1., 0.),
            Complex64::new(1., 1.),
            Complex64::new(2., 3.),
            Complex64::new(1., 4.),
            Complex64::new(1., 4.),
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
