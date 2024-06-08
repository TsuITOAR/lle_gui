use lle::CoupleOp;

use super::*;

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoupleLleController {
    a: LleController,
    pos: Property,
    g: Property,
}

impl Default for CoupleLleController {
    fn default() -> Self {
        Self {
            a: LleController::default(),
            pos: Property::new_int(0, "pos"),
            g: Property::new_float(100., "g"),
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
        let properties = &self.a.properties;
        let step_dist = properties["step dist"].value.f64().unwrap();
        let pump = properties["pump"].value.f64().unwrap();
        let linear = properties["linear"].value.f64().unwrap();
        let alpha = properties["alpha"].value.f64().unwrap();
        let pos = self.pos.value.i32().unwrap();
        let g = self.g.value.f64().unwrap();
        let mut init = vec![zero(); dim];
        default_add_random(init.iter_mut());
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
    fn construct_with_seed(&self, dim: usize, _seed: u32) -> CLleSolver {
        self.construct_engine(dim)
    }

    fn show_in_control_panel(&mut self, ui: &mut egui::Ui) {
        for p in self.a.properties.values_mut() {
            p.show_in_control_panel(ui)
        }
        self.g.show_in_control_panel(ui);
        self.pos.show_in_control_panel(ui);
    }

    fn show_in_start_window(&mut self, dim: &mut usize, ui: &mut egui::Ui) {
        crate::config::config(
            dim,
            self.a
                .properties
                .values_mut()
                .chain([&mut self.g, &mut self.pos]),
            ui,
        )
    }

    fn sync_paras(&mut self, engine: &mut CLleSolver) {
        crate::synchronize_properties(&self.a.properties, &mut engine.component1);
        crate::synchronize_properties_no_pump(&self.a.properties, &mut engine.component2);
        engine.couple.couple.strength = self.g.value.f64().unwrap();
        engine.couple.couple.mode = self.pos.value.i32().unwrap() as _;
    }

    fn steps(&self) -> u32 {
        self.a.properties["steps"].value.u32().unwrap()
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
