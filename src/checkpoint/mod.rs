use crate::{
    app::{Core, CoreStorage},
    controller::{Controller, Simulator},
    file::Extension,
};
use egui::Widget;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct CheckPoints<S> {
    checkpoints: Vec<CheckPoint<S>>,
    current: Option<usize>,
}

impl<C> Extension for CheckPoints<C>
where
    C: Extension,
{
    const EXTENSION: &'static str = "cp";
    fn extension() -> String {
        format!("{}.{}", C::EXTENSION, Self::EXTENSION)
    }
}

impl<S> CheckPoints<S> {
    pub fn add<T: Restorable<Store = S>>(&mut self, t: &mut T) {
        self.checkpoints.push(t.checkpoint());
    }

    pub fn restore<T: Restorable<Store = S>>(&mut self, t: &mut T, index: usize) {
        t.restore_by_ref(&self.checkpoints[index]);
    }

    pub fn delete(&mut self, index: usize) {
        self.checkpoints.remove(index);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckPoint<S> {
    pub name: Option<String>,
    pub state: S,
}

pub trait Restorable {
    type Store;
    fn checkpoint(&self) -> CheckPoint<Self::Store>;
    fn restore_by_ref(&mut self, checkpoint: &CheckPoint<Self::Store>);
}

impl<P, S> Restorable for Core<P, S>
where
    P: Clone + Controller<S>,
    S: Simulator,
    S::OwnedState: Clone,
{
    type Store = CoreStorage<P, S>; // Ensure CoreStorage implements Clone
    fn checkpoint(&self) -> CheckPoint<Self::Store> {
        CheckPoint {
            name: None,
            state: CoreStorage::from(self),
        }
    }

    /* fn from_checkpoint(checkpoint: CheckPoint<Self::Store>) -> Self {
        Self::from(checkpoint.state)
    }

    fn restore(&mut self, checkpoint: CheckPoint<Self::Store>) {
        *self = Core::from(checkpoint.state);
    } */

    fn restore_by_ref(&mut self, checkpoint: &CheckPoint<Self::Store>) {
        *self = Core::from(checkpoint.state.clone());
    }
}

impl<S> CheckPoints<S> {
    pub fn show<T: Restorable<Store = S>>(&mut self, ui: &mut egui::Ui, dst: &mut T) -> bool {
        let mut changed: bool = false;
        if ui.button("Add checkpoint").clicked() {
            self.add(dst);
        }
        ui.horizontal(|ui| {
            for (i, checkpoint) in self.checkpoints.iter_mut().enumerate() {
                let res = ui.add(egui::Button::selectable(
                    self.current == Some(i),
                    format!("{}.{}", i, checkpoint.name.get_or_insert_default()),
                ));
                if res.double_clicked() {
                    self.current = Some(i);
                    dst.restore_by_ref(checkpoint);
                    changed = true;
                } else if res.clicked() {
                    self.current = Some(i);
                }
            }
        });
        //self.current.map(|i| &self.checkpoints[i]);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.current.is_some(), egui::Button::new("load"))
                .clicked()
                && let Some(current) = self.current
            {
                self.restore(dst, current);
                changed = true;
            }
            if ui
                .add_enabled(self.current.is_some(), egui::Button::new("delete"))
                .clicked()
                && let Some(current) = self.current
            {
                self.delete(current);
                self.current = None;
            }
        });

        if let Some(current) = self.current {
            ui.collapsing("Checkpoint editor", |ui| {
                let edit_target = &mut self.checkpoints[current];
                egui::TextEdit::singleline(edit_target.name.get_or_insert_default())
                    .hint_text("add tag to this checkpoint")
                    .ui(ui);
            });
        }
        changed
    }
}
