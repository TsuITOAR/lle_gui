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
        if handle.as_ref()?.is_finished() {
            let handle = handle.take()?;
            match RUNTIME.block_on(handle) {
                Ok(x) => Some(x),
                Err(e) => {
                    crate::TOASTS
                        .lock()
                        .error(format!("Error in future: {}", e));
                    None
                }
            }
        } else {
            None
        }
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct FileStorage {
        pub(crate) name: String,
        pub(crate) read: Option<PathBuf>,
        pub(crate) save: Option<PathBuf>,
    }

    impl From<FileStorage> for super::FileManager {
        fn from(f: FileStorage) -> Self {
            Self {
                name: f.name,
                files: super::FilePaths {
                    read: f.read.map(FileHandle::from),
                    save: f.save.map(FileHandle::from),
                },
                futures: Default::default(),
            }
        }
    }

    impl From<&super::FileManager> for FileStorage {
        fn from(f: &super::FileManager) -> Self {
            Self {
                name: f.name.clone(),
                read: f.files.read.as_ref().map(|x| x.path().to_path_buf()),
                save: f.files.save.as_ref().map(|x| x.path().to_path_buf()),
            }
        }
    }

    impl serde::Serialize for super::FileManager {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            FileStorage::from(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for super::FileManager {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            FileStorage::deserialize(deserializer).map(Self::from)
        }
    }

    pub fn file_name(file: Option<&FileHandle>) -> Option<String> {
        file.map(FileHandle::file_name)
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
    pub struct FileStorage {
        name: String,
    }

    impl From<FileStorage> for super::FileManager {
        fn from(f: FileStorage) -> Self {
            Self {
                name: f.name,
                files: Default::default(),
                futures: Default::default(),
            }
        }
    }

    impl From<&super::FileManager> for FileStorage {
        fn from(f: &super::FileManager) -> Self {
            Self {
                name: f.name.clone(),
            }
        }
    }

    impl serde::Serialize for super::FileManager {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            FileStorage::from(self).serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for super::FileManager {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            FileStorage::deserialize(deserializer).map(Self::from)
        }
    }

    pub fn file_name(file: Option<&FileHandle>) -> Option<String> {
        file.map(FileHandle::file_name)
    }
}

#[cfg(target_arch = "wasm32")]
use wasm::*;

#[cfg(not(target_arch = "wasm32"))]
use native::*;

use crate::{
    controller::{Controller, Simulator},
    core::{Core, CoreStorage},
};

#[derive(Debug, Default)]
pub struct FileFutures {
    write_spawn: Option<FutureFileHandle>,
    read_spawn: Option<FutureFileHandle>,
    read_io_spawn: Option<FutureFileReadHandle>,
    save_io_spawn: Option<FutureFileSaveHandle>,
    cache: Option<Arc<Vec<u8>>>,
}

#[derive(Debug, Default, Clone)]
pub struct FilePaths {
    read: Option<FileHandle>,
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
            let state = bincode::deserialize(&data)?;
            self.cache = Some(data);
            *p = state;
            changed = true;
        }

        try_poll(&mut self.save_io_spawn);

        if let Some(Some(read)) = try_poll(&mut self.read_spawn) {
            files.update_read(read)?;
            self.cache = None;
        }

        if let Some(Some(save)) = try_poll(&mut self.write_spawn) {
            files.update_save(save)?;
        }

        Ok(changed)
    }

    pub(crate) fn spawn_read_browser<S: Extension>(&mut self) -> anyhow::Result<()> {
        self.read_spawn.get_or_insert_with(|| {
            spawn(async move {
                rfd::AsyncFileDialog::new()
                    .add_filter("model", &[S::extension()])
                    .add_filter("all", &["*"])
                    .pick_file()
                    .await
            })
        });
        Ok(())
    }

    pub(crate) fn spawn_write_browser<S: Extension>(&mut self) -> anyhow::Result<()> {
        self.write_spawn.get_or_insert_with(|| {
            spawn(async move {
                rfd::AsyncFileDialog::new()
                    .add_filter("model", &[S::extension()])
                    .add_filter("all", &["*"])
                    .save_file()
                    .await
            })
        });
        Ok(())
    }

    pub(crate) fn spawn_read(&mut self, file: &FileHandle) -> anyhow::Result<()> {
        if let Some(ref data) = self.cache {
            let data = data.clone();
            self.read_io_spawn = Some(spawn(async move { data }));
            Ok(())
        } else {
            let file = file.clone();
            self.read_io_spawn = Some(spawn(async move {
                let data = file.read().await;
                Arc::new(data)
            }));
            Ok(())
        }
    }

    pub(crate) fn spawn_write<T: serde::Serialize>(
        &mut self,
        file: &FileHandle,
        t: &T,
    ) -> anyhow::Result<()> {
        let file = file.clone();
        let serialized_data = bincode::serialize(t)?;
        self.save_io_spawn = Some(spawn(async move {
            let serialized_data = serialized_data;
            file.write(&serialized_data).await?;
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
        if let Some(ref save) = self.files.save {
            self.futures.spawn_write(save, s)?;
        } else {
            bail!("Save file not set");
        }
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
                let file = self.files.save.as_ref();
                if ui
                    .button(file_name(file).unwrap_or("Set save path".to_string()))
                    .clicked()
                {
                    self.futures.spawn_write_browser::<S>()?;
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
