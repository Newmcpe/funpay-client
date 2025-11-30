use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait StateStorage: Send + Sync {
    async fn load(&self) -> anyhow::Result<HashMap<i64, i64>>;
    async fn save(&self, data: &HashMap<i64, i64>) -> anyhow::Result<()>;
}

pub mod json;
pub mod memory;
