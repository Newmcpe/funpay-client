use crate::storage::StateStorage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

pub struct JsonFileStorage {
    path: PathBuf,
}

impl JsonFileStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl StateStorage for JsonFileStorage {
    async fn load(&self) -> anyhow::Result<HashMap<i64, i64>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let content = fs::read_to_string(&self.path).await?;
        let data = serde_json::from_str(&content)?;
        Ok(data)
    }

    async fn save(&self, data: &HashMap<i64, i64>) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).await?;
            }
        }
        let serialized = serde_json::to_string(data)?;
        fs::write(&self.path, serialized).await?;
        Ok(())
    }
}
