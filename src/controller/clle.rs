use lle::CoupleOp;

use super::*;

#[allow(unused)]
pub type App = crate::GenApp<CoupleLleController, CLleSolver, [crate::drawer::ViewField; 2]>;

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
    fn construct_engine(&self, dim: usize, r: &mut RandomNoise) -> CLleSolver {
        use lle::LinearOp;

        let step_dist = self.a.step_dist.get_value();
        let pump = self.a.pump.get_value();
        let linear = self.a.linear.get_value();
        let alpha = self.a.alpha.get_value();
        let pos = self.pos.get_value();
        let g = self.g.get_value();

        let mut init = vec![zero(); dim];
        r.add_random(init.as_mut_slice());
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
    fn add_rand(&mut self, r: &mut RandomNoise) {
        self.component1.add_rand(r);
        self.component2.add_rand(r);
    }
}
