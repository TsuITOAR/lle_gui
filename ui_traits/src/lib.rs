pub trait ControllerStartWindow {
    fn show_start_window(&mut self, ui: &mut egui::Ui);
}

pub trait ControllerUI {
    fn show_controller(&mut self, ui: &mut egui::Ui);
}

pub use ui_traits_proc::*;

pub trait DisplayStr {
    fn desc(&self) -> &str;
}

impl<T: DisplayStr + enum_iterator::Sequence + Eq> ControllerUI for T {
    fn show_controller(&mut self, ui: &mut egui::Ui) {
        enum_iterator::all::<T>().for_each(|s| {
            if ui.selectable_label(self == &s, s.desc()).clicked() {
                *self = s;
            }
        })
    }
}
