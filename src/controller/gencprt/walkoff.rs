use std::f64::consts::TAU;

use lle::{num_complex::Complex64, ConstOp, DiffOrder, Evolver, StaticLinearOp, Step};

use crate::{
    controller::{Controller, SharedState, Simulator, StoreState},
    random::RandomNoise,
    FftSource,
};

use super::{ops::PumpFreq, state::State, GenCprtController};

pub struct WalkOff<E> {
    pub cp: super::state::CoupleInfo,
    pub core: E,
    pub fft: Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>,
}

impl<
        L: lle::LinearOp<f64> + Send + Sync + 'static,
        NL: lle::NonLinearOp<f64> + Send + Sync + 'static,
        C: ConstOp<f64> + Send + Sync + 'static,
        CF: ConstOp<f64> + Send + Sync + 'static,
    > Evolver<f64> for WalkOff<lle::LleSolver<f64, State, L, NL, C, CF>>
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
        self.core.evolve();
        let step_dist = self.core.step_dist;
        let frac_d1_2pi = self.core.get_raw_state().cp.frac_d1_2pi;
        let time = self.core.get_raw_state().time;
        self.core.get_raw_state_mut().time = (time + step_dist).rem_euclid(2. / frac_d1_2pi);
        let step_dist = self.core.step_dist;
        let state = self.core.get_raw_state_mut();
        apply_walk_off(state, step_dist, &mut self.fft);
    }
}

fn apply_walk_off(
    state: &mut State,
    step_dist: f64,
    fft: &mut Option<(lle::BufferedFft<f64>, lle::BufferedFft<f64>)>,
) {
    let d1 = state.cp.frac_d1_2pi * TAU;
    let len = state.data.len();
    let fft = fft.get_or_insert_with(|| lle::BufferedFft::new(len / 2));
    let (a, b) = state.data.split_at_mut(len / 2);
    fft.0.fft_process(a);
    fft.0.fft_process(b);

    let (freq_iter_mut_pos, freq_iter_mut_neg) = state.coupling_iter_mut();
    freq_iter_mut_pos.for_each(|x| {
        use super::state::ModeMut;
        let m = x.m() as f64;
        match x {
            ModeMut::Single { amp, .. } => {
                if let Some(amp) = amp {
                    *amp *= (-Complex64::i() * m / 2. * d1 * step_dist).exp()
                }
            }
            ModeMut::Pair { amp1, amp2, .. } => {
                if let Some(amp1) = amp1 {
                    *amp1 *= (-Complex64::i() * m / 2. * d1 * step_dist).exp()
                }
                if let Some(amp2) = amp2 {
                    *amp2 *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp()
                }
            }
        };
    });

    freq_iter_mut_neg.for_each(|x| {
        use super::state::ModeMut;
        let m = x.m() as f64;
        match x {
            ModeMut::Single { amp, .. } => {
                if let Some(amp) = amp {
                    *amp *= (-Complex64::i() * m / 2. * d1 * step_dist).exp()
                }
            }
            ModeMut::Pair { amp1, amp2, .. } => {
                // amp1 is from ring 1
                if let Some(amp1) = amp1 {
                    *amp1 *= (-Complex64::i() * m / 2. * d1 * step_dist).exp()
                }
                if let Some(amp2) = amp2 {
                    *amp2 *= (-Complex64::i() * -m / 2. * d1 * step_dist).exp()
                }
            }
        };
    });

    let (a, b) = state.data.split_at_mut(len / 2);
    fft.1.fft_process(a);
    fft.1.fft_process(b);
    let scale = len as f64 / 2.;
    state.data.iter_mut().for_each(|x| *x /= scale);
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
        L: lle::LinearOp<f64> + Send + Sync + 'static,
        NL: lle::NonLinearOp<f64> + Send + Sync + 'static,
        C: ConstOp<f64> + Send + Sync + 'static,
        CF: ConstOp<f64> + Send + Sync + 'static,
    > Simulator for WalkOff<lle::LleSolver<f64, State, L, NL, C, CF>>
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

    use lle::FftSource;

    use super::*;
    use crate::controller::gencprt::state::{CoupleInfo, State};
    #[test]
    fn test_walkoff() {
        let cp = CoupleInfo {
            couple_strength: Default::default(),
            center_pos: 0.0,
            period: 10.,
            frac_d1_2pi: 0.5,
        };
        let mut state = State {
            data: TEST_DATA.to_vec(),
            cp: cp.clone(),
            time: 1.,
        };

        let step_dist = 0.55;

        let mut back = state.clone();
        let len = state.data.len();

        let mut fft = State::default_fft(state.fft_len());
        back.fft_process_forward(&mut fft);
        let scale = state.scale_factor();
        let mut fft1 = None;
        for i in 0..100 {
            apply_walk_off(&mut state, step_dist, &mut fft1);
            state.time += step_dist;

            state.fft_process_forward(&mut fft);
            // coupling_modes(&mut state);
            println!("loop {i}");
            state
                .data
                .iter()
                .zip(back.data.iter())
                .enumerate()
                .for_each(|(i, (a, b))| {
                    println!("{i}\t {a:>8}, {b:>8} ");
                });
            use assert_approx_eq::assert_approx_eq;
            use lle::num_complex::ComplexFloat;
            for (a, b) in state.data.iter().zip(back.data.iter()) {
                assert_approx_eq!(a, b);
            }
            state.fft_process_inverse(&mut fft);
            state.data.iter_mut().for_each(|x| *x /= scale);
        }
        let linear: (lle::DiffOrder, Complex64) = (2, Complex64::i() * 0.5 / 2. / 4.);
        for i in 0..100 {
            println!("loop {i}");
            use lle::LinearOpExt;
            apply_walk_off(&mut state, step_dist, &mut fft1);
            state.time += step_dist;

            state.fft_process_forward(&mut fft);
            state
                .data
                .iter()
                .zip(back.data.iter())
                .enumerate()
                .for_each(|(i, (a, b))| {
                    println!("{i}\t {:08}, {:08}", a.abs(), b.abs());
                });
            use assert_approx_eq::assert_approx_eq;
            use lle::num_complex::ComplexFloat;
            for (a, b) in state.data.iter().zip(back.data.iter()).take(len / 2) {
                assert_approx_eq!(a.abs(), b.abs());
            }
            linear.apply_freq(state.as_mut(), step_dist, 0);

            state.fft_process_inverse(&mut fft);
            state.data.iter_mut().for_each(|x| *x /= scale);
        }
    }

    use super::super::TEST_DATA;
}
