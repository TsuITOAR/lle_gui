use lle::num_complex::Complex64;

use crate::lle_util::FpXPhaMod;

#[allow(unused)]
pub type App = crate::app::GenApp<FpLleController, FpLleSolver, crate::drawer::ViewField>;

pub type FpLleSolver = lle::LleSolver<
    f64,
    Vec<Complex64>,
    lle::LinearOpAdd<f64, (lle::DiffOrder, Complex64), (lle::DiffOrder, Complex64)>,
    Nonlin,
    Complex64,
>;

pub type FpLleController = super::LleController;

type Nonlin = lle::NonLinearOpAdd<f64, lle::SPhaMod, FpXPhaMod>;
