use std::path::{Path, PathBuf};

use anyhow::bail;

#[derive(Debug, PartialEq, PartialOrd, Default, serde::Deserialize, serde::Serialize)]
pub struct File {
    #[serde(default)]
    read: String,
    #[serde(default)]
    save: String,
    #[serde(skip)]
    cache: Option<Vec<u8>>,
}

impl File {
    pub(crate) fn clone_for_save(&self) -> Self {
        Self {
            read: self.read.clone(),
            save: self.save.clone(),
            cache: None,
        }
    }
    pub(crate) fn update_load(&mut self, read: PathBuf) -> anyhow::Result<()> {
        let load = read
            .to_str()
            .ok_or_else(|| anyhow::Error::msg("Invalid path"))?;
        if load != self.read.as_str() {
            self.read = load.to_string();
            self.cache = None;
        }
        Ok(())
    }

    pub(crate) fn update_save(&mut self, save: PathBuf) -> anyhow::Result<()> {
        let save = save
            .to_str()
            .ok_or_else(|| anyhow::Error::msg("Invalid path"))?;

        self.save = save.to_string();

        Ok(())
    }

    pub(crate) fn save_state<P: serde::Serialize>(&self, state: &P) -> anyhow::Result<()> {
        if self.save.is_empty() {
            bail!("Empty save path");
        }
        let path = Path::new(self.save.as_str());
        let serialized_data = bincode::serialize(state)?;
        std::fs::write(path, serialized_data)?;
        Ok(())
    }

    pub(crate) fn load_state<P: for<'de> serde::Deserialize<'de>>(&mut self) -> anyhow::Result<P> {
        if self.read.is_empty() {
            bail!("Empty load path");
        }
        let path = Path::new(self.read.as_str());
        if self.cache.is_none() {
            let data = std::fs::read(path)?;
            self.cache = Some(data);
        }
        let cache = &self.cache.as_ref().unwrap();
        let state = bincode::deserialize(cache)?;
        Ok(state)
    }
}

impl File {
    pub fn show<S: serde::Serialize + for<'de> serde::Deserialize<'de>>(
        &mut self,
        ui: &mut egui::Ui,
        s: &mut S,
    ) -> anyhow::Result<bool> {
        let mut changed = false;
        ui.collapsing("File", |ui| -> anyhow::Result<()> {
            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Read").clicked() {
                    let state = self.load_state::<S>()?;
                    *s = state;
                    changed = true;
                }
                ui.add(egui::TextEdit::singleline(&mut self.read).hint_text("File path to load"));
                if ui.button("Browse").clicked() {
                    if let Some(file) = rfd::FileDialog::new().pick_file() {
                        self.update_load(file)?;
                    }
                }

                Ok(())
            })
            .inner?;

            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Save").clicked() {
                    self.save_state(s)?;
                }
                ui.add(egui::TextEdit::singleline(&mut self.save).hint_text("File path to save"));
                if ui.button("Browse").clicked() {
                    if let Some(file) = rfd::FileDialog::new().save_file() {
                        self.update_save(file)?;
                    }
                }

                Ok(())
            })
            .inner?;
            if ui.button("Refresh cache").clicked() {
                self.cache = None;
            }
            Ok(())
        })
        .body_returned
        .unwrap_or(Ok(()))?;
        Ok(changed)
    }
}
