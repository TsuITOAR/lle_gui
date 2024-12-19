use super::*;

#[derive(Debug, Clone, serde::Serialize, serde:: Deserialize)]
pub struct PulsePumpOp {
    pub(crate) peak: f64,
    pub(crate) width: f64,
}

impl Default for PulsePumpOp {
    fn default() -> Self {
        Self {
            peak: 10.,
            width: 64.,
        }
    }
}

impl lle::ConstOp<f64> for PulsePumpOp {
    fn get_value(&self, _cur_step: Step, pos: usize, state: &[Complex64]) -> Complex64 {
        let len = state.len();
        let t = pos as f64 - len as f64 / 2.;
        let t = t / self.width;
        (t.cosh().recip() * self.peak).into()
    }
}

impl lle::StaticConstOp<f64> for PulsePumpOp {}
