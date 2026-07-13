use crate::store::StoreError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{self, Duration};

#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub backup_dir: PathBuf,
    pub backup_interval_minutes: u64,
    pub max_backups: usize,
    pub compress: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_dir: PathBuf::from("backups"),
            backup_interval_minutes: 60,
            max_backups: 24,
            compress: true,
        }
    }
}

#[derive(Debug)]
pub struct BackupManager {
    config: BackupConfig,
    running: bool,
}

impl BackupManager {
    pub fn new(config: BackupConfig) -> Self {
        fs::create_dir_all(&config.backup_dir).ok();
        Self { config, running: false }
    }

    pub async fn start_backup_loop<F>(&mut self, backup_fn: F)
    where
        F: Fn() -> Result<PathBuf, StoreError> + Send + Sync + 'static,
    {
        if self.running {
            return;
        }
        self.running = true;

        let interval = Duration::from_secs(self.config.backup_interval_minutes * 60);
        let mut interval_stream = time::interval(interval);

        if self.running {
            loop {
                interval_stream.tick().await;
                if !self.running {
                    break;
                }
                if let Err(e) = self.perform_backup(&backup_fn).await {
                    tracing::error!("backup failed: {}", e);
                }
            }
        }
    }

    pub async fn perform_backup<F>(&self, backup_fn: F) -> Result<PathBuf, StoreError>
    where
        F: Fn() -> Result<PathBuf, StoreError> + Send + Sync,
    {
        let backup_path = backup_fn()?;
        self.enforce_retention()?;
        Ok(backup_path)
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    fn enforce_retention(&self) -> Result<(), StoreError> {
        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.config.backup_dir)
            .map_err(|e| StoreError::Storage(format!("read backup dir: {}", e)))?
        {
            let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {}", e)))?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("backup-") && name.ends_with(".db") {
                    backups.push(path);
                }
            }
        }
        backups.sort_by(|a, b| {
            let a_modified =
                fs::metadata(a).ok().and_then(|m| m.modified().ok()).unwrap_or(UNIX_EPOCH);
            let b_modified =
                fs::metadata(b).ok().and_then(|m| m.modified().ok()).unwrap_or(UNIX_EPOCH);
            a_modified.cmp(&b_modified)
        });

        while backups.len() > self.config.max_backups {
            let oldest = backups.remove(0);
            fs::remove_file(&oldest)
                .map_err(|e| StoreError::Storage(format!("remove old backup: {}", e)))?;
        }
        Ok(())
    }

    pub fn backup_path(&self) -> PathBuf {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        self.config.backup_dir.join(format!("backup-{}.db", timestamp))
    }

    pub fn list_backups(&self) -> Result<Vec<BackupInfo>, StoreError> {
        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.config.backup_dir)
            .map_err(|e| StoreError::Storage(format!("read backup dir: {}", e)))?
        {
            let entry = entry.map_err(|e| StoreError::Storage(format!("dir entry: {}", e)))?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("backup-") && name.ends_with(".db") {
                    let metadata = fs::metadata(&path)
                        .map_err(|e| StoreError::Storage(format!("get metadata: {}", e)))?;
                    backups.push(BackupInfo {
                        path,
                        size_bytes: metadata.len(),
                        created_at: metadata.modified().map_err(|e| {
                            StoreError::Storage(format!("get modified time: {}", e))
                        })?,
                    });
                }
            }
        }
        backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(backups)
    }

    pub fn restore_from_backup(
        &self,
        backup_path: &Path,
        target_path: &Path,
    ) -> Result<(), StoreError> {
        fs::copy(backup_path, target_path)
            .map_err(|e| StoreError::Storage(format!("restore backup: {}", e)))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created_at: SystemTime,
}

impl BackupInfo {
    pub fn created_at_ns(&self) -> u64 {
        self.created_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos() as u64
    }
}
