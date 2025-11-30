pub const DEFAULT_BASE_URL: &str = "https://funpay.com";

pub struct UrlBuilder {
    base_url: String,
}

impl Default for UrlBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

impl UrlBuilder {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn home(&self) -> String {
        format!("{}/", self.base_url)
    }

    pub fn runner(&self) -> String {
        format!("{}/runner/", self.base_url)
    }

    pub fn orders_trade(&self) -> String {
        format!("{}/orders/trade", self.base_url)
    }

    pub fn order_page(&self, order_id: &str) -> String {
        format!("{}/orders/{order_id}/", self.base_url)
    }

    pub fn chat_page(&self, chat_id: &str) -> String {
        format!("{}/chat/?node={chat_id}", self.base_url)
    }

    pub fn offer_edit(&self, node_id: i64, offer_id: i64) -> String {
        format!(
            "{}/lots/offerEdit?node={node_id}&offer={offer_id}",
            self.base_url
        )
    }

    pub fn offer_save(&self) -> String {
        format!("{}/lots/offerSave", self.base_url)
    }

    pub fn lots_trade(&self, node_id: i64) -> String {
        format!("{}/lots/{node_id}/trade", self.base_url)
    }

    pub fn lots_page(&self, node_id: i64) -> String {
        format!("{}/lots/{node_id}/", self.base_url)
    }
}
