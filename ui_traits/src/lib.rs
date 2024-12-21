pub trait ControllerStartWindow {
    fn show_start_window(&mut self, ui: &mut egui::Ui);
}

pub trait ControllerUI {
    fn show_controller(&mut self, ui: &mut egui::Ui);
}

pub use ui_traits_proc::*;
