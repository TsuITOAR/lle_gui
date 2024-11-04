use lle::CoupleOp;

use super::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoupleLleController {
    a: LleController,
    pos: Property<i32>,
    g: Property<f64>,
}

impl Default for CoupleLleController {
    fn default() -> Self {
        Self {
            a: LleController::default(),
            pos: Property::new(0, "pos"),
            g: Property::new(100., "g").range((0., 100.)),
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

        let step_dist = self.a.step_dist.get_value();
        let pump = self.a.pump.get_value();
        let linear = self.a.linear.get_value();
        let alpha = self.a.alpha.get_value();
        let pos = self.pos.get_value();
        let g = self.g.get_value();

        let mut init = vec![zero(); dim];
        default_add_random(init.as_mut_slice());
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

    // todo: use seed
    fn construct_with_seed(&self, dim: usize, seed: u64) -> CLleSolver {
        use lle::LinearOp;

        let step_dist = self.a.step_dist.get_value();
        let pump = self.a.pump.get_value();
        let linear = self.a.linear.get_value();
        let alpha = self.a.alpha.get_value();
        let pos = self.pos.get_value();
        let g = self.g.get_value();

        let mut init = vec![zero(); dim];
        default_add_random_with_seed(init.as_mut_slice(), seed);
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
            LleSolver<SPhaMod>>>::show_in_control_panel(&mut self.a,ui);

        self.g.show_in_control_panel(ui);
        self.pos.show_in_control_panel(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(dim, &mut self.a, ui)
    }

    fn sync_paras(&mut self, engine: &mut CLleSolver) {
        crate::synchronize_properties(&self.a, &mut engine.component1);
        crate::synchronize_properties_no_pump(&self.a, &mut engine.component2);
        engine.couple.couple.strength = self.g.get_value();
        engine.couple.couple.mode = self.pos.get_value();
    }

    fn steps(&self) -> u32 {
        self.a.steps.get_value()
    }
}

impl Simulator for CLleSolver {
    type State = [Complex64];
    fn run(&mut self, steps: u32) {
        use lle::Evolver;
        self.evolve_n(steps)
    }
    fn states(&self) -> &Self::State {
        use lle::Evolver;
        self.state()
    }
}
