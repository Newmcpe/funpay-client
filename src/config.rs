use std::path::PathBuf;
use std::time::Duration;

pub struct FunPayConfig {
    pub base_url: String,
    pub user_agent: String,
    pub retry_base_ms: u32,
    pub max_retries: u32,
    pub redirect_limit: usize,
    pub polling_interval: Duration,
    pub error_retry_delay: Duration,
    pub event_channel_capacity: usize,
    pub state_storage_path: Option<PathBuf>,
}

impl Default for FunPayConfig {
    fn default() -> Self {
        Self {
            base_url: "https://funpay.com".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36".to_string(),
            retry_base_ms: 20,
            max_retries: 3,
            redirect_limit: 10,
            polling_interval: Duration::from_millis(1500),
            error_retry_delay: Duration::from_secs(5),
            event_channel_capacity: 512,
            state_storage_path: None,
        }
    }
}

impl FunPayConfig {
    pub fn builder() -> FunPayConfigBuilder {
        FunPayConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct FunPayConfigBuilder {
    config: FunPayConfig,
}

impl FunPayConfigBuilder {
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.config.base_url = url.into();
        self
    }

    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.config.user_agent = ua.into();
        self
    }

    pub fn retry_policy(mut self, base_ms: u32, max_retries: u32) -> Self {
        self.config.retry_base_ms = base_ms;
        self.config.max_retries = max_retries;
        self
    }

    pub fn redirect_limit(mut self, limit: usize) -> Self {
        self.config.redirect_limit = limit;
        self
    }

    pub fn polling_interval(mut self, interval: Duration) -> Self {
        self.config.polling_interval = interval;
        self
    }

    pub fn error_retry_delay(mut self, delay: Duration) -> Self {
        self.config.error_retry_delay = delay;
        self
    }

    pub fn event_channel_capacity(mut self, capacity: usize) -> Self {
        self.config.event_channel_capacity = capacity;
        self
    }

    pub fn state_storage_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.state_storage_path = Some(path.into());
        self
    }

    pub fn build(self) -> FunPayConfig {
        self.config
    }
}
