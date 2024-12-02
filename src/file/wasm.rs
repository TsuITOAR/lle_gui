use rfd::FileHandle;
use std::fmt::Debug;

pub fn try_poll<T>(handle: &mut Option<super::FutureHandler<T>>) -> Option<T> {
    let h = handle.take()?;
    match h.try_take() {
        Ok(x) => Some(x),
        Err(e) => {
            *handle = Some(e);
            None
        }
    }
}

impl<T: 'static> super::Promise<T> {
    pub fn new(f: impl std::future::Future<Output = T> + 'static) -> Self {
        Self(poll_promise::Promise::spawn_local(f))
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
