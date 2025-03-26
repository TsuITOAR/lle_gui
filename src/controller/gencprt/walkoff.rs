use lle::{freq_at, num_complex::Complex64, ConstOp, DiffOrder, Evolver, StaticLinearOp, Step};

use crate::{
    controller::{Controller, SharedState, Simulator, StoreState},
    random::RandomNoise,
    FftSource,
};

use super::{ops::PumpFreq, GenCprtController};

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
        self.core.evolve();
        /* let data = self.core.state_mut();
        let len = data.len();
        let fft = self
            .fft
            .get_or_insert_with(|| lle::BufferedFft::new(len / 2));
        let (f_a, f_b) = data.split_at_mut(len / 2);
        fft.0.fft_process(f_a);
        fft.0.fft_process(f_b);
        apply_walk_off(f_a, f_b, &self.cp, step_dist);
        fft.1.fft_process(f_a);
        fft.1.fft_process(f_b);
        let scale = data.len() as f64 / 2.;
        data.iter_mut().for_each(|x| *x /= scale); */
    }
}

fn apply_walk_off(
    f_a: &mut [Complex64],
    f_b: &mut [Complex64],
    cp: &super::state::CoupleInfo,
    step_dist: f64,
) {
    let d1 = cp.frac_d1_2pi * 2. * std::f64::consts::PI;
    let len = f_a.len();
    for (i, (f_a, f_b)) in f_a.iter_mut().zip(f_b.iter_mut()).enumerate() {
        let freq = freq_at(len, i);
        let m = cp.m(freq as _) as f64;
        *f_a *= (Complex64::i() * -m / 2. * d1 * step_dist).exp();
        *f_b *= (Complex64::i() * m / 2. * d1 * step_dist).exp();
    }
}

impl<NL: Default + lle::NonLinearOp<f64>>
    Controller<WalkOff<super::LleSolver<NL, lle::NoneOp<f64>, PumpFreq>>> for GenCprtController
{
    const EXTENSION: &'static str = "gencprt";
    type Dispersion =
        lle::LinearOpAdd<f64, (DiffOrder, Complex64), crate::controller::cprt2::CprtDispersion2>;
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
        let state = super::state::State::new(dim, self.disper.get_coup_info());
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
