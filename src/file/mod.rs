use std::sync::Arc;

use anyhow::bail;
use rfd::FileHandle;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
use wasm::*;

use crate::{
    app::{Core, CoreStorage},
    controller::{Controller, Simulator},
    util::{try_poll, FutureHandler, Promise},
};

pub type FutureFileHandle = FutureHandler<Option<FileHandle>>;
pub type FutureFileSaveHandle = FutureHandler<anyhow::Result<()>>;
pub type FutureFileReadHandle = FutureHandler<Arc<Vec<u8>>>;

#[derive(Default)]
pub struct FileFutures {
    #[cfg(not(target_arch = "wasm32"))]
    write_path_spawn: Option<FutureFileHandle>,
    read_path_spawn: Option<FutureFileHandle>,
    read_io_spawn: Option<FutureFileReadHandle>,
    save_io_spawn: Option<FutureFileSaveHandle>,
    cache: Option<Arc<Vec<u8>>>,
}

impl std::fmt::Debug for FileFutures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn is_some<T>(x: &Option<T>) -> &'static str {
            if x.is_some() {
                "Some(..)"
            } else {
                "None"
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            f.debug_struct("FileFutures")
                .field("write_path_spawn", &is_some(&self.write_path_spawn))
                .field("read_path_spawn", &is_some(&self.read_path_spawn))
                .field("read_io_spawn", &is_some(&self.read_io_spawn))
                .field("save_io_spawn", &is_some(&self.save_io_spawn))
                .field("cache", &self.cache)
                .finish()
        }
        #[cfg(target_arch = "wasm32")]
        {
            f.debug_struct("FileFutures")
                .field("read_path_spawn", &is_some(&self.read_path_spawn))
                .field("read_io_spawn", &is_some(&self.read_io_spawn))
                .field("save_io_spawn", &is_some(&self.save_io_spawn))
                .field("cache", &self.cache)
                .finish()
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FilePaths {
    read: Option<FileHandle>,
    #[cfg(not(target_arch = "wasm32"))]
    save: Option<FileHandle>,
}

#[derive(Debug)]
pub struct FileManager {
    name: String,
    files: FilePaths,
    futures: FileFutures,
}

impl FileFutures {
    pub fn poll<P: for<'de> serde::Deserialize<'de>>(
        &mut self,
        p: &mut P,
        files: &mut FilePaths,
    ) -> anyhow::Result<bool> {
        let mut changed = false;
        if let Some(data) = try_poll(&mut self.read_io_spawn) {
            let state = ron::de::from_bytes(&data)?;
            self.cache = Some(data);
            *p = state;
            changed = true;
        }

        try_poll(&mut self.save_io_spawn);

        if let Some(Some(read)) = try_poll(&mut self.read_path_spawn) {
            files.update_read(read)?;
            self.cache = None;
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(Some(save)) = try_poll(&mut self.write_path_spawn) {
            files.update_save(save)?;
        }

        Ok(changed)
    }

    pub(crate) fn spawn_read_browser<S: Extension>(&mut self) -> anyhow::Result<()> {
        self.read_path_spawn.get_or_insert_with(|| {
            Promise::new(async move {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    rfd::AsyncFileDialog::new()
                        .add_filter("model", &[format!("{}.ron", S::extension())])
                        .add_filter("all", &["*"])
                        .pick_file()
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                {
                    rfd::AsyncFileDialog::new()
                        .add_filter("model", &[format!("{}.ron", S::extension())])
                        .pick_file()
                        .await
                }
            })
        });
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn spawn_write_browser<S: Extension>(&mut self) -> anyhow::Result<()> {
        self.write_path_spawn.get_or_insert_with(|| {
            Promise::new(async move {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    rfd::AsyncFileDialog::new()
                        .add_filter("model", &[format!("{}.ron", S::extension())])
                        .add_filter("all", &["*"])
                        .save_file()
                        .await
                }
                #[cfg(target_arch = "wasm32")]
                {
                    rfd::AsyncFileDialog::new()
                        .add_filter("model", &[format!("{}.ron", S::extension())])
                        .save_file()
                        .await
                }
            })
        });
        Ok(())
    }

    pub(crate) fn spawn_read(&mut self, file: &FileHandle) -> anyhow::Result<()> {
        if let Some(ref data) = self.cache {
            let data = data.clone();
            self.read_io_spawn = Some(Promise::new(async move { data }));
            Ok(())
        } else {
            let file = file.clone();
            self.read_io_spawn = Some(Promise::new(async move {
                let data = file.read().await;
                Arc::new(data)
            }));
            Ok(())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn spawn_write<T: serde::Serialize>(
        &mut self,
        file: &FileHandle,
        t: &T,
    ) -> anyhow::Result<()> {
        #[allow(unused)]
        let file = file.clone();
        let serialized_data = ron::ser::to_string_pretty(t, ron::ser::PrettyConfig::default())?;
        self.save_io_spawn = Some(Promise::new(async move {
            let serialized_data = serialized_data;
            file.write(serialized_data.as_bytes()).await?;
            Ok(())
        }));
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn spawn_write<T: serde::Serialize>(&mut self, t: &T) -> anyhow::Result<()> {
        let serialized_data = ron::ser::to_string_pretty(t, ron::ser::PrettyConfig::default())?;
        self.save_io_spawn = Some(Promise::new(async move {
            if let Some(file) = rfd::AsyncFileDialog::new().save_file().await {
                let serialized_data = serialized_data;
                file.write(serialized_data.as_bytes()).await?;
            } else {
                bail!("Can't save file");
            };

            Ok(())
        }));
        Ok(())
    }
}

impl FilePaths {
    pub(crate) fn update_read(&mut self, read: FileHandle) -> anyhow::Result<()> {
        self.read = Some(read);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn update_save(&mut self, save: FileHandle) -> anyhow::Result<()> {
        self.save = Some(save);
        Ok(())
    }
}

impl FileManager {
    pub(crate) fn default_state() -> Self {
        Self::new("Save state")
    }

    pub(crate) fn default_check_points() -> Self {
        Self::new("Save checkpoints")
    }

    pub fn new(n: impl ToString) -> Self {
        Self {
            name: n.to_string(),
            files: Default::default(),
            futures: Default::default(),
        }
    }

    pub(crate) fn clone_for_save(&self) -> Self {
        Self {
            name: self.name.clone(),
            files: self.files.clone(),
            futures: Default::default(),
        }
    }

    pub fn start_read(&mut self) -> anyhow::Result<()> {
        if let Some(ref read) = self.files.read {
            self.futures.spawn_read(read)?;
        } else {
            bail!("Read file not set");
        }
        Ok(())
    }
    pub fn start_write<T: serde::Serialize>(&mut self, s: &T) -> anyhow::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(ref save) = self.files.save {
            self.futures.spawn_write(save, s)?;
        } else {
            bail!("Save file not set");
        }
        #[cfg(target_arch = "wasm32")]
        self.futures.spawn_write(s)?;
        Ok(())
    }

    pub fn show_save_load<S>(&mut self, ui: &mut egui::Ui, s: &mut S) -> anyhow::Result<bool>
    where
        S: Extension + serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
        ui.collapsing(self.name.clone(), |ui| -> anyhow::Result<()> {
            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Read").clicked() {
                    self.start_read()?;
                }
                //ui.add(egui::TextEdit::singleline(&mut self.read).hint_text("File path to load"));
                let file = self.files.read.as_ref();
                if ui
                    .button(file_name(file).unwrap_or("Set read path".to_string()))
                    .clicked()
                {
                    self.futures.spawn_read_browser::<S>()?;
                }

                Ok(())
            })
            .inner?;

            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Save").clicked() {
                    self.start_write(s)?;
                }
                //ui.add(egui::TextEdit::singleline(&mut self.save).hint_text("File path to save"));
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let file = self.files.save.as_ref();
                    if ui
                        .button(file_name(file).unwrap_or("Set save path".to_string()))
                        .clicked()
                    {
                        self.futures.spawn_write_browser::<S>()?;
                    }
                }

                Ok(())
            })
            .inner?;
            if ui.button("Refresh cache").clicked() {
                self.futures.cache = None;
            }
            Ok(())
        })
        .body_returned
        .unwrap_or(Ok(()))?;
        let changed = self.futures.poll(s, &mut self.files)?;
        Ok(changed)
    }
}

pub trait Extension {
    const EXTENSION: &'static str;
    fn extension() -> String {
        Self::EXTENSION.to_string()
    }
}

impl<C, S> Extension for Core<C, S>
where
    C: Controller<S>,
    S: Simulator,
{
    const EXTENSION: &'static str = C::EXTENSION;
}

impl<C, S> Extension for CoreStorage<C, S>
where
    C: Controller<S>,
    S: Simulator,
{
    const EXTENSION: &'static str = C::EXTENSION;
}
