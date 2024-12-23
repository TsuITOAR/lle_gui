use super::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InterleaveSelfPumpOp {
    pub(crate) channel1: SelfPumpOp,
    pub(crate) channel2: SelfPumpOp,
    pub(crate) mix: f64,
}

impl Default for InterleaveSelfPumpOp {
    fn default() -> Self {
        Self {
            channel1: SelfPumpOp::default(),
            channel2: SelfPumpOp::default(),
            mix: 1.,
        }
    }
}

impl InterleaveSelfPumpOp {
    fn update_state(&self, state: &[Complex64]) {
        self.channel1.update_state(state);
        self.channel2.update_state(state);
    }

    fn cur_value(&self, len: usize, pos: usize) -> Complex64 {
        let channel1 = self.channel1.cur_value(len, pos);
        let channel2 = self.channel2.cur_value(len, pos);
        channel1 * self.mix + channel2 * (1. - self.mix)
    }

    fn cur_array(&self, len: usize) -> Vec<Complex64> {
        let mut channnel1 = self.channel1.cur_array(len);
        let channnel2 = self.channel2.cur_array(len);
        channnel1
            .iter_mut()
            .zip(channnel2.iter())
            .for_each(|(x, y)| *x = *x * self.mix + *y * (1. - self.mix));
        channnel1
    }
}

impl lle::ConstOp<f64> for InterleaveSelfPumpOp {
    fn get_value(&self, _cur_step: Step, pos: usize, state: &[Complex64]) -> Complex64 {
        if pos == 0 {
            self.update_state(state);
        }
        self.cur_value(state.len(), pos)
    }
    fn get_value_array(&self, _cur_step: Step, state: &[Complex64]) -> Vec<Complex64> {
        self.update_state(state);
        self.cur_array(state.len())
    }

    fn fill_value_array(&self, cur_step: Step, state: &[Complex64], dst: &mut [Complex64]) {
        let r = self.get_value_array(cur_step, state);
        dst.copy_from_slice(&r);
    }
}
