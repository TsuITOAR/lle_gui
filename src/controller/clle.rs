use lle::{CoupleOp, DiffOrder, NoneOp};

use super::*;

#[allow(unused)]
pub type App = crate::app::GenApp<CoupleLleController, CLleSolver, [crate::drawer::ViewField; 2]>;

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    ui_traits::ControllerStartWindow,
    ui_traits::ControllerUI,
)]
pub struct CoupleLleController {
    pub(crate) basic: LleController,
    pub(crate) couple: Property<f64>,
    pub(crate) pos: Property<i32>,
}

impl Default for CoupleLleController {
    fn default() -> Self {
        Self {
            basic: LleController::default(),
            pos: Property::new_no_slider(0, "pos"),
            couple: Property::new(0., "g").range((0., 100.)),
        }
    }
}

pub type Couple = lle::CoupleOpWithLinear<lle::ModeSplit<f64>, lle::XPhaMod>;

pub type CLleSolver = lle::CoupledLleSolver<
    f64,
    Vec<Complex64>,
    Vec<Complex64>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    SPhaMod,
    SPhaMod,
    Complex64,
    NoneOp<f64>,
    NoneOp<f64>,
    NoneOp<f64>,
    Couple,
>;

impl Controller<CLleSolver> for CoupleLleController {
    const EXTENSION: &'static str = "clle";
    type Dispersion = (DiffOrder, Complex64);
    fn dispersion(&self) -> Self::Dispersion {
        (2, Complex64::i() * self.basic.linear.get_value() / 2.)
    }
    fn construct_engine(&self, dim: usize) -> CLleSolver {
        use lle::LinearOp;

        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let linear = self.basic.linear.get_value();
        let alpha = self.basic.alpha.get_value();
        let pos = self.pos.get_value();
        let g = self.couple.get_value();

        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        CLleSolver::builder()
            .component1(
                LleSolver::builder()
                    .state(init.to_vec())
                    .step_dist(step_dist)
                    .linear(
                        (0, -(Complex64::i() * alpha + 1.))
                            .add_linear_op((2, Complex64::i() * linear / 2.)),
                    )
                    .nonlin(SPhaMod)
                    .constant(Complex64::from(pump))
                    .constant_freq(NoneOp::default())
                    .build(),
            )
            .component2(
                lle::LleSolver::new(init, step_dist)
                    .linear(
                        (0, -(Complex64::i() * alpha + 1.))
                            .add_linear_op((2, Complex64::i() * linear / 2.)),
                    )
                    .nonlin(SPhaMod),
            )
            .couple(
                lle::ModeSplit {
                    mode: pos as _,
                    strength: g,
                }
                .with_linear(lle::XPhaMod),
            )
            .build()
    }

    fn sync_paras(&mut self, engine: &mut CLleSolver) {
        crate::util::synchronize_properties(&self.basic, &mut engine.component1);
        crate::util::synchronize_properties_no_pump(&self.basic, &mut engine.component2);
        engine.couple.couple.strength = self.couple.get_value();
        engine.couple.couple.mode = self.pos.get_value();
    }

    fn steps(&self) -> u32 {
        self.basic.steps.get_value()
    }
}

impl<'a> SharedState<'a> for CLleSolver {
    type SharedState = [&'a [Complex64]; 2];

    fn states(&'a self) -> Self::SharedState {
        use lle::Evolver;
        [self.component1.state(), self.component2.state()]
    }
    fn set_state(&mut self, state: Self::SharedState) {
        self.component1.set_state(state[0]);
        self.component2.set_state(state[1]);
    }
}

impl StoreState for CLleSolver {
    type OwnedState = [Vec<Complex64>; 2];
    fn get_owned_state(&self) -> Self::OwnedState {
        [
            self.component1.get_owned_state(),
            self.component2.get_owned_state(),
        ]
    }
    fn set_owned_state(&mut self, state: Self::OwnedState) {
        let [a, b] = state;
        self.component1.set_owned_state(a);
        self.component2.set_owned_state(b);
    }
    fn default_state(dim: usize) -> Self::OwnedState {
        [vec![Complex64::zero(); dim], vec![Complex64::zero(); dim]]
    }
}

impl Simulator for CLleSolver {
    fn add_rand(&mut self, r: &mut RandomNoise) {
        self.component1.add_rand(r);
        self.component2.add_rand(r);
    }
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps)
    }
    fn cur_step(&self) -> u32 {
        <Self as lle::Evolver<f64>>::cur_step(self)
    }
}
