use lle::CoupleOp;

use super::*;

#[allow(unused)]
pub type App = crate::GenApp<CoupleLleController, CLleSolver, [crate::drawer::ViewField; 2]>;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoupleLleController {
    basic: LleController,
    pos: Property<i32>,
    g: Property<f64>,
}

impl Default for CoupleLleController {
    fn default() -> Self {
        Self {
            basic: LleController::default(),
            pos: Property::new_no_slider(0, "pos"),
            g: Property::new(0., "g").range((0., 100.)),
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
    Couple,
>;

impl Controller<CLleSolver> for CoupleLleController {
    fn construct_engine(&self, dim: usize) -> CLleSolver {
        use lle::LinearOp;

        let step_dist = self.basic.step_dist.get_value();
        let pump = self.basic.pump.get_value();
        let linear = self.basic.linear.get_value();
        let alpha = self.basic.alpha.get_value();
        let pos = self.pos.get_value();
        let g = self.g.get_value();

        let init = vec![zero(); dim];
        //r.add_random(init.as_mut_slice());
        CLleSolver::builder()
            .component1(
                LleSolver::builder()
                    .state(init.to_vec())
                    .step_dist(step_dist)
                    .linear(
                        (0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)),
                    )
                    .nonlin(SPhaMod::default())
                    .constant(Complex64::from(pump))
                    .build(),
            )
            .component2(
                LleSolver::builder()
                    .state(init.to_vec())
                    .step_dist(step_dist)
                    .linear(
                        (0, -(Complex64::i() * alpha + 1.)).add((2, -Complex64::i() * linear / 2.)),
                    )
                    .nonlin(SPhaMod::default())
                    .build(),
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

    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        <crate::controller::LleController as crate::controller::Controller<
            LleSolver<SPhaMod>>>::show_in_control_panel(&mut self.basic,ui);

        self.g.show_in_control_panel(ui);
        self.pos.show_in_control_panel(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(dim, &mut self.basic, ui)
    }

    fn sync_paras(&mut self, engine: &mut CLleSolver) {
        crate::synchronize_properties(&self.basic, &mut engine.component1);
        crate::synchronize_properties_no_pump(&self.basic, &mut engine.component2);
        engine.couple.couple.strength = self.g.get_value();
        engine.couple.couple.mode = self.pos.get_value();
    }

    fn steps(&self) -> u32 {
        self.basic.steps.get_value()
    }
}

impl<'a> Simulator<'a> for CLleSolver {
    type State = [&'a [Complex64]; 2];
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps)
    }
    fn states(&'a self) -> Self::State {
        use lle::Evolver;
        [self.component1.state(), self.component2.state()]
    }
    fn set_state(&mut self, state: Self::State) {
        self.component1.set_state(state[0]);
        self.component2.set_state(state[1]);
    }
    fn add_rand(&mut self, r: &mut RandomNoise) {
        self.component1.add_rand(r);
        self.component2.add_rand(r);
    }
}
