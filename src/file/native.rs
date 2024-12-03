use std::path::PathBuf;

use rfd::FileHandle;

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
