use lle::{freq_at, num_complex::Complex64, Step};
use num_traits::Zero;

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

#[cfg(test)]
mod test {
    use lle::num_complex::Complex64;

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
}
