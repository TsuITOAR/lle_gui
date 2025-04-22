use std::f64::consts::TAU;

use lle::{num_complex::Complex64, ConstOp, DiffOrder, Evolver, StaticLinearOp, Step};

use crate::{
    controller::{Controller, SharedState, Simulator, StoreState},
    random::RandomNoise,
    FftSource,
};

use super::{ops::PumpFreq, singularity_point, GenCprtController};

pub struct WalkOff<E> {
    pub cp: super::state::CoupleInfo,
    pub core: E,
    pub fft: Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>,
}

impl<
        S: FftSource + for<'a> serde::Deserialize<'a> + serde::Serialize,
        L: lle::LinearOp<f64> + Send + Sync + 'static,
        NL: lle::NonLinearOp<f64> + Send + Sync + 'static,
        C: ConstOp<f64> + Send + Sync + 'static,
        CF: ConstOp<f64> + Send + Sync + 'static,
    > Evolver<f64> for WalkOff<lle::LleSolver<f64, S, L, NL, C, CF>>
{
    fn state(&self) -> &[lle::num_complex::Complex<f64>] {
        self.core.state()
    }
    fn state_mut(&mut self) -> &mut [lle::num_complex::Complex<f64>] {
        self.core.state_mut()
    }
    fn cur_step(&self) -> Step {
        Evolver::cur_step(&self.core)
    }
    fn evolve(&mut self) {
        let step_dist = self.core.step_dist;
        apply_walk_off(self.core.state_mut(), step_dist, &mut self.fft, &self.cp);
        self.core.evolve();
    }
}

fn apply_walk_off(
    state: &mut [Complex64],
    step_dist: f64,
    fft: &mut Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>,
    cp: &super::state::CoupleInfo,
) {
    let len = state.len();
    let fft = fft.get_or_insert_with(|| lle::BufferedFft::new(len / 2));

    let (f_a, f_b) = state.split_at_mut(len / 2);
    fft.0.fft_process(f_a);
    fft.0.fft_process(f_b);
    let d1 = cp.frac_d1_2pi * TAU;
    //let step_dist = 1. / d1;
    let len = f_a.len();
    let (f_a_p, f_a_n) = f_a.split_at_mut(len / 2);
    let (f_b_p, f_b_n) = f_b.split_at_mut(len / 2);
    let mut f_a_p = f_a_p.iter_mut();
    let mut f_a_n = f_a_n.iter_mut().rev();
    let mut f_b_p = f_b_p.iter_mut();
    let mut f_b_n = f_b_n.iter_mut().rev();
    for freq in (0..(len / 2)).map(|x| lle::freq_at(len, x)) {
        debug_assert!(freq >= 0);
        let m = cp.m_original(freq * 2) as f64;
        if singularity_point(freq * 2, cp.center_pos, cp.period) {
            if let Some(f) = f_a_p.next() {
                *f *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp();
            }
        } else {
            if let Some(f) = f_a_p.next() {
                *f *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp();
            }
            if let Some(f) = f_b_p.next() {
                *f *= (-Complex64::i() * m / 2. * d1 * step_dist).exp();
            }
        }
    }
    for freq in ((len / 2)..len).rev().map(|x| lle::freq_at(len, x)) {
        debug_assert!(freq < 0);
        let m = cp.m_original(freq * 2) as f64;
        if singularity_point(freq * 2, cp.center_pos, cp.period) {
            if let Some(f) = f_a_n.next() {
                *f *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp();
            }
        } else {
            if let Some(f) = f_a_n.next() {
                *f *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp();
            }
            if let Some(f) = f_b_n.next() {
                *f *= (-Complex64::i() * m / 2. * d1 * step_dist).exp();
            }
        }
    }
    fft.1.fft_process(f_a);
    fft.1.fft_process(f_b);
    let scale = state.len() as f64 / 2.;
    state.iter_mut().for_each(|x| *x /= scale);
}

impl<NL: Default + lle::NonLinearOp<f64>>
    Controller<WalkOff<super::LleSolver<NL, lle::NoneOp<f64>, PumpFreq>>> for GenCprtController
{
    const EXTENSION: &'static str = "gencprt";
    type Dispersion = lle::LinearOpAdd<f64, (DiffOrder, Complex64), super::CprtDispersionFrac>;
    fn dispersion(&self) -> Self::Dispersion {
        use lle::LinearOp;
        (2, Complex64::i() * self.disper.linear.get_value() / 2. / 4.)
            .add_linear_op(self.disper.get_cprt_dispersion())
    }
    fn construct_engine(
        &self,
        dim: usize,
    ) -> WalkOff<super::LleSolver<NL, lle::NoneOp<f64>, PumpFreq>> {
        let step_dist = self.step_dist.get_value();
        let pump = self.pump.get_pump();
        let state = super::state::State::new(
            dim,
            self.disper.get_coup_info(),
            (self.steps.get_value() as f64 * step_dist)
                .rem_euclid(self.disper.frac_d1_2pi.get_value() * TAU),
        );
        //r.add_random(init.as_mut_slice());
        let core = super::LleSolver::builder()
            .state(state)
            .step_dist(step_dist)
            .linear(self.get_dispersion().cached_linear_op(dim))
            .nonlin(NL::default())
            .constant(lle::NoneOp::default())
            .constant_freq(pump)
            .build();
        WalkOff {
            cp: self.disper.get_coup_info(),
            core,
            fft: None,
        }
    }

    fn steps(&self) -> u32 {
        self.steps.get_value()
    }
    fn sync_paras(
        &mut self,
        engine: &mut WalkOff<super::LleSolver<NL, lle::NoneOp<f64>, PumpFreq>>,
    ) {
        use lle::Evolver;
        engine.cp = self.disper.get_coup_info();
        let engine = &mut engine.core;
        engine.get_raw_state_mut().cp = self.disper.get_coup_info();
        engine.constant_freq = self.pump.get_pump();
        engine.step_dist = self.step_dist.get_value();
        engine.linear = self
            .get_dispersion()
            .cached_linear_op(engine.state().as_ref().len());
    }
}
impl<
        'a,
        S: FftSource,
        L: lle::LinearOp<f64>,
        NL: lle::NonLinearOp<f64>,
        C: ConstOp<f64>,
        CF: ConstOp<f64>,
    > SharedState<'a> for WalkOff<lle::LleSolver<f64, S, L, NL, C, CF>>
{
    type SharedState = &'a S;
    fn states(&'a self) -> Self::SharedState {
        self.core.get_raw_state()
    }
    fn set_state(&mut self, state: &S) {
        *self.core.get_raw_state_mut() = state.clone();
    }
}

impl<
        S: FftSource + for<'a> serde::Deserialize<'a> + serde::Serialize,
        L: lle::LinearOp<f64>,
        NL: lle::NonLinearOp<f64>,
        C: ConstOp<f64>,
        CF: ConstOp<f64>,
    > StoreState for WalkOff<lle::LleSolver<f64, S, L, NL, C, CF>>
{
    type OwnedState = S;
    fn get_owned_state(&self) -> Self::OwnedState {
        self.core.get_raw_state().clone()
    }
    fn set_owned_state(&mut self, state: Self::OwnedState) {
        if self.core.state().len() != state.as_ref().len() {
            crate::notify::TOASTS.lock().warning(format!(
                "Skipping restore state for mismatched length between simulator({}) and storage({})",
                self.core.state().len(),
                state.as_ref().len()
            ));
            return;
        }
        *self.core.get_raw_state_mut() = state;
    }
    fn default_state(dim: usize) -> Self::OwnedState {
        S::default_with_len(dim)
    }
}

impl<
        S: FftSource + for<'a> serde::Deserialize<'a> + serde::Serialize,
        L: lle::LinearOp<f64> + Send + Sync + 'static,
        NL: lle::NonLinearOp<f64> + Send + Sync + 'static,
        C: ConstOp<f64> + Send + Sync + 'static,
        CF: ConstOp<f64> + Send + Sync + 'static,
    > Simulator for WalkOff<lle::LleSolver<f64, S, L, NL, C, CF>>
where
    S::FftProcessor: Send + Sync,
{
    fn run(&mut self, steps: u32) {
        self.evolve_n(steps as _);
    }
    fn add_rand(&mut self, r: &mut RandomNoise) {
        self.core.add_rand(r);
    }
    fn cur_step(&self) -> u32 {
        Simulator::cur_step(&self.core)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::controller::gencprt::{
        ops::coupling_modes,
        state::{CoupleInfo, State},
    };
    #[test]
    fn test_walkoff() {
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 1.5,
            period: 2.1,
            frac_d1_2pi: 0.5,
        };
        let mut state = State {
            data: DATA.to_vec(),
            cp: cp.clone(),
            time: 1.,
        };

        let step_dist = 1e-2;

        let len = state.data.len();

        let mut back = state.data.clone();
        let (back_p, back_n) = back.split_at_mut(len / 2);
        coupling_modes(back_p, back_n, &cp, state.time);

        let mut fft = None;
        apply_walk_off(&mut state.data, step_dist, &mut fft, &state.cp);

        let (data_p, data_n) = state.data.split_at_mut(len / 2);
        coupling_modes(data_p, data_n, &cp, state.time + step_dist);

        for (i, (a, b)) in back.iter().zip(state.data.iter()).enumerate() {
            assert!((a - b).norm_sqr() < 1e-5, "i: {}, a: {}, b: {}", i, a, b);
        }
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
