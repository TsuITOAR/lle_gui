use super::*;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Copy, Default)]
pub struct FpXPhaMod;

impl lle::NonLinearOp<f64> for FpXPhaMod {
    fn get_value(
        &mut self,
        _step: Step,
        state: &[lle::num_complex::Complex<f64>],
        dst: &mut [lle::num_complex::Complex<f64>],
    ) {
        let sum = state.iter().map(|x| x.norm_sqr()).sum::<f64>();
        let mean = sum / (state.len() as f64);
        dst.fill(Complex64::i() * 2. * mean);
    }
}
