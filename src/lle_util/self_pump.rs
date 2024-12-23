use egui::mutex::RwLock;
use num_traits::Zero;

use super::*;

pub struct SelfPumpOp {
    pub(crate) now: RwLock<usize>,
    pub(crate) delay: usize,
    pub(crate) d1_mismatch: f64,
    pub(crate) loop_dispersion: f64,
    pub(crate) loop_loss: f64,
    pub(crate) window: usize,
    pub(crate) cache: RwLock<Vec<Complex64>>,
    pub(crate) fft: RwLock<Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>>,
}

impl Clone for SelfPumpOp {
    fn clone(&self) -> Self {
        SelfPumpOp {
            now: RwLock::new(*self.now.read()),
            delay: self.delay,
            d1_mismatch: self.d1_mismatch,
            loop_dispersion: self.loop_dispersion,
            loop_loss: self.loop_loss,
            window: self.window,
            cache: RwLock::new(self.cache.read().clone()),
            fft: RwLock::new(None),
        }
    }
}

impl std::fmt::Debug for SelfPumpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelfPumpOp")
            .field("now", &self.now.read())
            .field("delay", &self.delay)
            .field("d1_mismatch", &self.d1_mismatch)
            .field("loop_dispersion", &self.loop_dispersion)
            .field("loop_loss", &self.loop_loss)
            .field("window", &self.window)
            .finish()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfPumpOpStorage {
    pub(crate) now: usize,
    pub(crate) delay: usize,
    pub(crate) d1_mismatch: f64,
    pub(crate) loop_dispersion: f64,
    pub(crate) loop_loss: f64,
    pub(crate) window: usize,
}

impl serde::Serialize for SelfPumpOp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SelfPumpOpStorage {
            now: *self.now.read(),
            delay: self.delay,
            d1_mismatch: self.d1_mismatch,
            loop_dispersion: self.loop_dispersion,
            loop_loss: self.loop_loss,
            window: self.window,
        }
        .serialize(serializer)
    }
}

impl<'a> serde::Deserialize<'a> for SelfPumpOp {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let SelfPumpOpStorage {
            now,
            delay,
            d1_mismatch,
            loop_dispersion,
            loop_loss,
            window,
        } = SelfPumpOpStorage::deserialize(deserializer)?;
        Ok(SelfPumpOp {
            now: RwLock::new(now),
            delay,
            d1_mismatch,
            loop_dispersion,
            loop_loss,
            window,
            cache: RwLock::new(Vec::new()),
            fft: RwLock::new(None),
        })
    }
}

impl Default for SelfPumpOp {
    fn default() -> Self {
        Self {
            now: RwLock::new(0),
            delay: 0,
            d1_mismatch: 0.,
            loop_dispersion: 0.,
            window: 0,
            cache: RwLock::new(Vec::new()),
            loop_loss: 0.01,
            fft: RwLock::new(None),
        }
    }
}

impl SelfPumpOp {
    pub(crate) fn update_state(&self, state: &[Complex64]) {
        let Self {
            now,
            delay,
            d1_mismatch,
            loop_dispersion,
            loop_loss: _,
            window,
            cache,
            fft,
        } = self;
        let len = state.len();
        let mut now = now.write();
        if *now > *delay {
            *now = 0;
        }
        let mut cache = cache.write();
        if cache.len() < len * (delay + 1) {
            cache.resize(len * (delay + 1), Complex64::zero());
        }
        {
            let now = *now;
            let mut state = state.to_vec();
            if !d1_mismatch.is_zero() || !loop_dispersion.is_zero() || *window != 0 {
                let mut fft = fft.write();
                let fft = fft.get_or_insert_with(|| lle::BufferedFft::new(len));

                fft.0.fft_process(&mut state);
                use lle::LinearOp;
                if !d1_mismatch.is_zero() || !loop_dispersion.is_zero() {
                    lle::apply_linear_freq(
                        &mut state,
                        &(1, -Complex64::i() * *d1_mismatch / 2.)
                            .add_linear_op((2, -Complex64::i() * *loop_dispersion / 2.)),
                        1.,
                        0,
                    );
                }

                if *window != 0 {
                    let window: i32 = *window as i32;
                    let max_f = window / 2;
                    let min_f = max_f + 1 - window;
                    state.iter_mut().enumerate().for_each(|(i, x)| {
                        let f = lle::freq_at(len, i);
                        if f < min_f || f > max_f {
                            *x = Complex64::zero();
                        }
                    });
                }

                fft.1.fft_process(&mut state);

                state.iter_mut().for_each(|x| *x /= len as f64);
            }
            cache[(now * len)..(now * len + len)].copy_from_slice(&state);
        }
        *now += 1;
        if *delay != 0 {
            *now %= delay;
        } else {
            *now = 0;
        }
    }

    pub(crate) fn cur_value(&self, len: usize, pos: usize) -> Complex64 {
        let now = *self.now.read();
        self.cache.read()[now * len + pos]
    }

    pub(crate) fn cur_array(&self, len: usize) -> Vec<Complex64> {
        let now = *self.now.read();
        self.cache.read()[now * len..now * len + len].to_vec()
    }
}

impl lle::ConstOp<f64> for SelfPumpOp {
    fn get_value(&self, _cur_step: Step, pos: usize, state: &[Complex64]) -> Complex64 {
        if pos == 0 {
            self.update_state(state);
        }
        self.cur_value(state.len(), pos) * self.loop_loss
    }
    fn get_value_array(&self, _cur_step: Step, state: &[Complex64]) -> Vec<Complex64> {
        self.update_state(state);
        let mut ret = self.cur_array(state.len());
        ret.iter_mut().for_each(|x| *x *= self.loop_loss);
        ret
    }

    fn fill_value_array(&self, cur_step: Step, state: &[Complex64], dst: &mut [Complex64]) {
        let r = self.get_value_array(cur_step, state);
        dst.copy_from_slice(&r);
    }
}
