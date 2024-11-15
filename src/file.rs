use std::sync::Arc;

use anyhow::bail;
use rfd::FileHandle;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        path::PathBuf,
        sync::{Arc, LazyLock},
    };

    use rfd::FileHandle;
    use tokio::runtime::Runtime;

    pub static RUNTIME: LazyLock<Runtime> = LazyLock::new(default_runtime);
    fn default_runtime() -> Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("Can't initialize runtime")
    }
    pub type FutureHandler<T> = tokio::task::JoinHandle<T>;
    pub type FutureFileHandle = FutureHandler<Option<FileHandle>>;
    pub type FutureFileSaveHandle = FutureHandler<anyhow::Result<()>>;
    pub type FutureFileReadHandle = FutureHandler<Arc<Vec<u8>>>;

    pub fn spawn<F: std::future::Future<Output = T> + Send + 'static, T: Send + 'static>(
        f: F,
    ) -> FutureHandler<T> {
        RUNTIME.spawn(f)
    }

    pub fn try_poll<T>(handle: &mut Option<tokio::task::JoinHandle<T>>) -> Option<T> {
        if handle.as_ref().map(|x| x.is_finished()).unwrap_or(false) {
            let spawn = handle.take().unwrap();
            Some(RUNTIME.block_on(spawn).expect("join task"))
        } else {
            None
        }
    }

    #[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
    pub struct FileStorage {
        read: Option<PathBuf>,
        save: Option<PathBuf>,
    }

    impl<'a> From<&'a super::File> for FileStorage {
        fn from(f: &'a super::File) -> Self {
            Self {
                read: f.read.clone().map(|x| x.path().to_owned()),
                save: f.save.clone().map(|x| x.path().to_owned()),
            }
        }
    }

    impl From<FileStorage> for super::File {
        fn from(f: FileStorage) -> Self {
            Self {
                read: f.read.map(|x| FileHandle::from(x)),
                save: f.save.map(|x| FileHandle::from(x)),
                read_spawn: None,
                read_io_spawn: None,
                save_spawn: None,
                save_io_spawn: None,
                cache: None,
            }
        }
    }

    impl serde::Serialize for super::File {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            FileStorage::from(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for super::File {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            FileStorage::deserialize(deserializer).map(Self::from)
        }
    }

    pub fn show_file_name(file: Option<&FileHandle>, ui: &mut egui::Ui) {
        ui.add(egui::Label::new(
            file.map(FileHandle::file_name)
                .unwrap_or("Not set".to_string()),
        ));
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{fmt::Debug, sync::Arc};

    use tokio::sync::oneshot;

    use rfd::FileHandle;

    pub type FutureHandler<T> = oneshot::Receiver<T>;
    pub type FutureFileHandle = FutureHandler<Option<FileHandle>>;
    pub type FutureFileSaveHandle = FutureHandler<anyhow::Result<()>>;
    pub type FutureFileReadHandle = FutureHandler<Arc<Vec<u8>>>;

    pub fn spawn<F: std::future::Future<Output = T> + 'static, T: Debug + 'static>(
        f: F,
    ) -> FutureHandler<T> {
        let (tx, rx) = oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move { tx.send(f.await).unwrap() });
        rx
    }

    pub fn try_poll<T>(handle: &mut Option<FutureHandler<T>>) -> Option<T> {
        if let Some(h) = handle.as_mut() {
            match h.try_recv() {
                Ok(x) => Some(x),
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => {
                    *handle = None;
                    None
                }
            }
        } else {
            None
        }
    }

    #[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
    pub struct FileStorage {}

    impl serde::Serialize for super::File {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            FileStorage::default().serialize(_serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for super::File {
        fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(Self::default())
        }
    }

    pub fn show_file_name(file: Option<&FileHandle>, ui: &mut egui::Ui) {
        ui.add(egui::Label::new(
            file.map(FileHandle::file_name)
                .unwrap_or("Not set".to_string()),
        ));
    }
}

#[cfg(target_arch = "wasm32")]
use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
use native::*;

#[derive(Debug, Default)]
pub struct File {
    read: Option<FileHandle>,
    read_spawn: Option<FutureFileHandle>,
    read_io_spawn: Option<FutureFileReadHandle>,
    save: Option<FileHandle>,
    save_spawn: Option<FutureFileHandle>,
    save_io_spawn: Option<FutureFileSaveHandle>,
    cache: Option<Arc<Vec<u8>>>,
}

impl File {
    pub fn poll<P: for<'de> serde::Deserialize<'de>>(&mut self, p: &mut P) -> anyhow::Result<bool> {
        let mut changed = false;
        if let Some(data) = try_poll(&mut self.read_io_spawn) {
            let state = bincode::deserialize(&data)?;
            self.cache = Some(data);
            *p = state;
            changed = true;
        }

        try_poll(&mut self.save_io_spawn);

        if let Some(Some(read)) = try_poll(&mut self.read_spawn) {
            self.update_read(read)?;
        }

        if let Some(Some(save)) = try_poll(&mut self.save_spawn) {
            self.update_save(save)?;
        }

        Ok(changed)
    }

    pub(crate) fn clone_for_save(&self) -> Self {
        Self {
            read: self.read.clone(),
            save: self.save.clone(),
            cache: None,
            read_spawn: None,
            save_spawn: None,
            read_io_spawn: None,
            save_io_spawn: None,
        }
    }

    pub(crate) fn update_read(&mut self, read: FileHandle) -> anyhow::Result<()> {
        self.read = Some(read);
        self.cache = None;

        Ok(())
    }

    pub(crate) fn update_save(&mut self, save: FileHandle) -> anyhow::Result<()> {
        self.save = Some(save);
        Ok(())
    }

    pub(crate) fn save_state<P: serde::Serialize>(&mut self, state: &P) -> anyhow::Result<()> {
        if let Some(ref s) = self.save {
            let serialized_data = bincode::serialize(state)?;
            let s = s.clone();
            self.save_io_spawn = Some(spawn(async move {
                let serialized_data = serialized_data;
                s.write(&serialized_data).await?;
                Ok(())
            }));
        } else {
            bail!("Save path not set");
        }

        Ok(())
    }

    pub(crate) fn load_state(&mut self) -> anyhow::Result<()> {
        if let Some(ref data) = self.cache {
            let data = data.clone();
            self.read_io_spawn = Some(spawn(async move { data }));
            Ok(())
        } else if let Some(ref s) = self.read {
            let s = s.clone();
            self.read_io_spawn = Some(spawn(async move {
                let data = s.read().await;
                Arc::new(data)
            }));
            Ok(())
        } else {
            bail!("Read path not set");
        }
    }
}

impl File {
    pub fn show<S: serde::Serialize + for<'de> serde::Deserialize<'de>>(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        s: &mut S,
    ) -> anyhow::Result<bool> {
        ui.collapsing("File", |ui| -> anyhow::Result<()> {
            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Read").clicked() {
                    self.load_state()?;
                }
                show_file_name(self.read.as_ref(), ui);
                //ui.add(egui::TextEdit::singleline(&mut self.read).hint_text("File path to load"));
                if ui.button("Browse").clicked() {
                    let ctx = ctx.clone();
                    self.read_spawn.get_or_insert_with(|| {
                        spawn(async move {
                            let s = rfd::AsyncFileDialog::new().pick_file().await;
                            ctx.request_repaint();
                            s.map(|x| x.into())
                        })
                    });
                }

                Ok(())
            })
            .inner?;

            ui.horizontal(|ui| -> anyhow::Result<()> {
                if ui.button("Save").clicked() {
                    self.save_state(s)?;
                }
                show_file_name(self.save.as_ref(), ui);
                //ui.add(egui::TextEdit::singleline(&mut self.save).hint_text("File path to save"));
                if ui.button("Browse").clicked() {
                    let ctx = ctx.clone();
                    self.save_spawn.get_or_insert_with(|| {
                        spawn(async move {
                            let s = rfd::AsyncFileDialog::new().save_file().await;
                            ctx.request_repaint();
                            s.map(|x| x.into())
                        })
                    });
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
        let changed = self.poll(s)?;
        Ok(changed)
    }
}
