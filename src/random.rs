use std::f64::consts::PI;

use lle::num_complex::Complex64;
use static_assertions::assert_impl_all;

#[derive(Debug, Clone)]
enum RandCore {
    Seed { seed: u64, rng: rand::rngs::StdRng },
    //Thread(rand::rngs::ThreadRng),
}

assert_impl_all!(RandCore: Send, Sync);

impl RandCore {
    fn new(seed: Option<u64>) -> Self {
        use rand::{Rng, SeedableRng};
        let seed = seed.unwrap_or(rand::thread_rng().gen());

        Self::Seed {
            seed,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }
}

impl Default for RandCore {
    fn default() -> Self {
        Self::new(None)
        //RandCore::Thread(rand::thread_rng())
    }
}

impl serde::Serialize for RandCore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let storage = RandCoreStorage::from(self.clone());
        storage.serialize(serializer)
    }
}

impl<'a> serde::Deserialize<'a> for RandCore {
    fn deserialize<D>(deserializer: D) -> Result<RandCore, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let storage = RandCoreStorage::deserialize(deserializer)?;
        Ok(storage.into())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RandCoreStorage {
    seed: Option<u64>,
}

impl From<RandCore> for RandCoreStorage {
    fn from(core: RandCore) -> Self {
        match core {
            RandCore::Seed { seed, .. } => Self { seed: Some(seed) },
            //RandCore::Thread(_) => Self { seed: None },
        }
    }
}

impl From<RandCoreStorage> for RandCore {
    fn from(storage: RandCoreStorage) -> Self {
        Self::new(storage.seed)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RandomNoise {
    core: RandCore,
    std_dev: f64,
}

impl Default for RandomNoise {
    fn default() -> Self {
        Self {
            core: RandCore::default(),
            std_dev: 1E-5,
        }
    }
}

impl RandomNoise {
    ///
    /// noise_amp = sqrt(1/2/dt)/norm_E
    /// norm_E = sqrt(kappa/2/g)
    /// dt = calc_step*tr/norm_t
    /// norm_t = 2/kappa
    ///
    ///
    pub fn new(std_dev: f64, seed: Option<u64>) -> Self {
        Self {
            core: RandCore::new(seed),
            std_dev,
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui, add: &mut bool) {
        ui.collapsing("Noise", |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(add, "Noise");
                if *add {
                    ui.add(
                        egui::Slider::new(&mut self.std_dev, 1E-10..=1E-4)
                            .logarithmic(true)
                            .clamping(egui::SliderClamping::Never)
                            .text("Amplitude")
                            .custom_formatter(|x, _r| format!("{:E}", x)),
                    );
                }
            })
        });
    }

    pub fn add_random(&mut self, state: &mut [Complex64]) {
        let dist = rand_distr::Normal::new(0., self.std_dev * (state.len() as f64).sqrt()).unwrap();
        match &mut self.core {
            RandCore::Seed { seed: _, rng } => {
                add_random_with_dist(state, rng, &dist);
            } /* RandCore::Thread(ref mut t) => {
                  add_random_with_dist(state, t, &dist);
              } */
        }
    }
}

fn add_random_with_dist<D: rand::distributions::Distribution<f64>>(
    state: &mut [Complex64],
    rng: &mut impl rand::Rng,
    dist: &D,
) {
    state
        .iter_mut()
        .for_each(|x| *x += (Complex64::i() * rng.gen::<f64>() * 2. * PI).exp() * dist.sample(rng));
}
