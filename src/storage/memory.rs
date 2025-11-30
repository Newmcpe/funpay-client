use crate::storage::StateStorage;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct InMemoryStorage {
    data: RwLock<HashMap<i64, i64>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStorage for InMemoryStorage {
    async fn load(&self) -> anyhow::Result<HashMap<i64, i64>> {
        Ok(self.data.read().unwrap().clone())
    }

    async fn save(&self, data: &HashMap<i64, i64>) -> anyhow::Result<()> {
        *self.data.write().unwrap() = data.clone();
        Ok(())
    }
}
