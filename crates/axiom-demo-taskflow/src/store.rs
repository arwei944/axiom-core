use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTask {
    pub id: String,
    pub title: String,
    pub priority: u8,
    pub plan: String,
    pub body: String,
}

#[derive(Clone, Default)]
pub struct InMemoryTaskStore {
    inner: Arc<Mutex<HashMap<String, StoredTask>>>,
}

impl InMemoryTaskStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save(&self, task: StoredTask) -> Result<(), String> {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| "store lock poisoned".to_string())?;
        g.insert(task.id.clone(), task);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<StoredTask> {
        self.inner.lock().ok()?.get(id).cloned()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }
}
