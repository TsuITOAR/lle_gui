use std::cell::RefCell;

use num_traits::Zero;

use super::*;

pub struct SelfPumpOp {
    pub(crate) now: RefCell<usize>,
    pub(crate) delay: usize,
    pub(crate) loop_dispersion: f64,
    pub(crate) loop_loss: f64,
    pub(crate) cache: RefCell<Vec<Complex64>>,
    fft: RefCell<Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>>,
}

impl Clone for SelfPumpOp {
    fn clone(&self) -> Self {
        SelfPumpOp {
            now: RefCell::new(*self.now.borrow()),
            delay: self.delay,
            loop_dispersion: self.loop_dispersion,
            loop_loss: self.loop_loss,
            cache: RefCell::new(self.cache.borrow().clone()),
            fft: RefCell::new(None),
        }
    }
}

impl std::fmt::Debug for SelfPumpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelfPumpOp")
            .field("now", &self.now)
            .field("delay", &self.delay)
            .field("loop_dispersion", &self.loop_dispersion)
            .field("loop_loss", &self.loop_loss)
            .finish()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelfPumpOpStorage {
    pub(crate) now: usize,
    pub(crate) delay: usize,
    pub(crate) loop_dispersion: f64,
    pub(crate) loop_loss: f64,
}

impl serde::Serialize for SelfPumpOp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SelfPumpOpStorage {
            now: *self.now.borrow(),
            delay: self.delay,
            loop_dispersion: self.loop_dispersion,
            loop_loss: self.loop_loss,
        }
        .serialize(serializer)
    }
}

impl<'a> serde::Deserialize<'a> for SelfPumpOp {
    fn deserialize<D: serde::Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        let SelfPumpOpStorage {
            now,
            delay,
            loop_dispersion,
            loop_loss,
        } = SelfPumpOpStorage::deserialize(deserializer)?;
        Ok(SelfPumpOp {
            now: RefCell::new(now),
            delay,
            loop_dispersion,
            loop_loss,
            cache: RefCell::new(Vec::new()),
            fft: RefCell::new(None),
        })
    }
}

impl Default for SelfPumpOp {
    fn default() -> Self {
        Self {
            now: RefCell::new(0),
            delay: 0,
            loop_dispersion: 0.,
            cache: RefCell::new(Vec::new()),
            loop_loss: 1.,
            fft: RefCell::new(None),
        }
    }
}

impl lle::ConstOp<f64> for SelfPumpOp {
    fn get_value(
        &self,
        _cur_step: Step,
        pos: usize,
        state: &[lle::num_complex::Complex<f64>],
    ) -> lle::num_complex::Complex<f64> {
        let Self {
            now,
            delay,
            loop_dispersion,
            loop_loss,
            cache,
            fft,
        } = self;
        let len = state.len();
        let mut cache = cache.borrow_mut();
        let mut state = state.to_vec();
        if loop_dispersion.is_zero() {
            lle::apply_linear(
                &mut state,
                &(2, -Complex64::i() * loop_dispersion / 2.),
                fft.borrow_mut()
                    .get_or_insert_with(|| lle::BufferedFft::new(len)),
                1.,
                0,
            );
        }

        if cache.len() < len * self.delay {
            cache.resize(len * (self.delay + 1), Complex64::zero());
        }
        let mut now = now.borrow_mut();
        cache[(*now * len)..(*now * len + len)].copy_from_slice(&state);
        let ret = cache[*now * len + pos];
        if pos == len - 1 {
            *now += 1;
            if *delay != 0 {
                *now %= delay;
            } else {
                *now = 0;
            }
        }
        ret * loop_loss
    }
}
