//! File store configuration.

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileStoreConfig {
    pub data_dir: PathBuf,
    pub max_file_size_bytes: u64,
    pub max_files: usize,
    pub index_file: PathBuf,
}

impl Default for FileStoreConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("events"),
            max_file_size_bytes: 50 * 1024 * 1024,
            max_files: 20,
            index_file: PathBuf::from("events/event_index.json"),
        }
    }
}
