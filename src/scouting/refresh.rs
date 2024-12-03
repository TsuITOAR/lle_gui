#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Refresh {
    count: u32,
    limit: u32,
}

impl Default for Refresh {
    fn default() -> Self {
        Self {
            count: 0,
            limit: 100,
        }
    }
}

impl Refresh {
    pub fn tick(&mut self) -> bool {
        self.count += 1;

        if self.limit == 0 {
            false
        } else if self.count >= self.limit {
            self.count = 0;
            true
        } else {
            false
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Auto refresh", |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.limit));
                ui.label(format!("Refresh after {}/{}", self.count, self.limit));
            })
        });
    }
}
