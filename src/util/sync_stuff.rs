use lle::{num_complex::Complex64, LinearOp, NonLinearOp, NoneOp};

pub fn synchronize_properties<NL: NonLinearOp<f64>>(
    props: &crate::controller::LleController,
    engine: &mut crate::controller::LleSolver<NL, Complex64, NoneOp<f64>>,
) {
    puffin_egui::puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add_linear_op((2, Complex64::i() * props.linear.get_value() / 2.));
    engine.constant = Complex64::from(props.pump.get_value());
    engine.step_dist = props.step_dist.get_value();
}

pub fn synchronize_properties_no_pump<NL: NonLinearOp<f64>>(
    props: &crate::controller::LleController,
    engine: &mut crate::controller::LleSolver<NL, NoneOp<f64>, NoneOp<f64>>,
) {
    puffin_egui::puffin::profile_function!();
    engine.linear = (0, -(Complex64::i() * props.alpha.get_value() + 1.))
        .add_linear_op((2, -Complex64::i() * props.linear.get_value() / 2.));
    engine.step_dist = props.step_dist.get_value();
}
