use crate::client::http::ReqwestGateway;
use crate::client::poller::FunPayPoller;
use crate::client::FunpayGateway;
use crate::config::FunPayConfig;
use crate::error::FunPayError;
use crate::events::Event;
use crate::models::enums::SubcategoryType;
use crate::models::ids::ChatId;
use crate::models::{
    CategoryFilter, CategorySubcategory, MarketOffer, Message, Offer, OfferEditParams, OfferFullParams, OfferSaveRequest, Order, OrderShortcut, Subcategory
};
use crate::parsing::{
    parse_category_filters, parse_category_subcategories, parse_market_offers, parse_message_html,
    parse_my_offers, parse_offer_edit_params, parse_offer_full_params, parse_order_page,
    parse_order_secrets, parse_orders_list,
};
use crate::storage::json::JsonFileStorage;
use crate::storage::memory::InMemoryStorage;
use crate::storage::StateStorage;
use crate::utils::{extract_phpsessid, random_tag};
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::{json, to_string, Value};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::{self, Sender};

#[derive(Debug, Clone)]
struct AppData {
    user_id: i64,
    csrf_token: String,
}

pub struct FunPayAccount {
    gateway: Arc<dyn FunpayGateway>,
    pub golden_key: String,
    user_agent: String,
    pub id: Option<i64>,
    pub username: Option<String>,
    pub csrf_token: Option<String>,
    phpsessid: Option<String>,
    locale: Option<String>,
    pub events_tx: Sender<Event>,
    sorted_subcategories: HashMap<SubcategoryType, HashMap<i64, Subcategory>>,
    storage: Arc<dyn StateStorage>,
    polling_interval: Duration,
    error_retry_delay: Duration,
}

impl fmt::Debug for FunPayAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunPayAccount")
            .field("golden_key", &"[redacted]")
            .field("user_agent", &self.user_agent)
            .field("id", &self.id)
            .field("username", &self.username)
            .finish()
    }
}

#[derive(Clone)]
pub struct FunPaySender {
    gateway: Arc<dyn FunpayGateway>,
    golden_key: String,
    user_agent: String,
    csrf_token: String,
    phpsessid: Option<String>,
    seller_id: i64,
}

impl FunPaySender {
    pub fn seller_id(&self) -> i64 {
        self.seller_id
    }
}

impl FunPayAccount {
    pub fn new(golden_key: String) -> Self {
        Self::with_config(golden_key, FunPayConfig::default())
    }

    pub fn with_config(golden_key: String, config: FunPayConfig) -> Self {
        let gateway: Arc<dyn FunpayGateway> = Arc::new(ReqwestGateway::with_config(&config));
        Self::with_gateway_and_config(gateway, golden_key, config)
    }

    pub fn with_proxy(golden_key: String, proxy_url: &str) -> Self {
        Self::with_proxy_and_config(golden_key, proxy_url, FunPayConfig::default())
    }

    pub fn with_proxy_and_config(
        golden_key: String,
        proxy_url: &str,
        config: FunPayConfig,
    ) -> Self {
        let gateway: Arc<dyn FunpayGateway> =
            Arc::new(ReqwestGateway::with_proxy_and_config(proxy_url, &config));
        Self::with_gateway_and_config(gateway, golden_key, config)
    }

    pub fn with_gateway(gateway: Arc<dyn FunpayGateway>, golden_key: String) -> Self {
        Self::with_gateway_and_config(gateway, golden_key, FunPayConfig::default())
    }

    pub fn with_gateway_and_config(
        gateway: Arc<dyn FunpayGateway>,
        golden_key: String,
        config: FunPayConfig,
    ) -> Self {
        let (tx, _rx) = broadcast::channel(config.event_channel_capacity);
        let storage: Arc<dyn StateStorage> = match config.state_storage_path {
            Some(ref path) => Arc::new(JsonFileStorage::new(path.clone())),
            None => Arc::new(InMemoryStorage::new()),
        };
        Self {
            gateway,
            golden_key,
            user_agent: config.user_agent.clone(),
            id: None,
            username: None,
            csrf_token: None,
            phpsessid: None,
            locale: None,
            events_tx: tx,
            sorted_subcategories: HashMap::new(),
            storage,
            polling_interval: config.polling_interval,
            error_retry_delay: config.error_retry_delay,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events_tx.subscribe()
    }

    pub async fn init(&mut self) -> Result<(), FunPayError> {
        self.get().await
    }

    pub fn create_sender(&self) -> Result<FunPaySender, FunPayError> {
        let csrf = self
            .csrf_token
            .as_ref()
            .ok_or(FunPayError::AccountNotInitiated)?
            .to_string();
        let seller_id = self.id.ok_or(FunPayError::AccountNotInitiated)?;
        Ok(FunPaySender {
            gateway: self.gateway.clone(),
            golden_key: self.golden_key.clone(),
            user_agent: self.user_agent.clone(),
            csrf_token: csrf,
            phpsessid: self.phpsessid.clone(),
            seller_id,
        })
    }

    async fn get(&mut self) -> Result<(), FunPayError> {
        let (body, set_cookies) = self
            .gateway
            .get_home(&self.golden_key, &self.user_agent)
            .await?;
        if let Some(sess) = extract_phpsessid(&set_cookies) {
            self.phpsessid = Some(sess);
        }
        let html = Html::parse_document(&body);
        let sel_body = Selector::parse("body").unwrap();
        let mut app_data: Option<AppData> = None;
        if let Some(b) = html.select(&sel_body).next() {
            if let Some(attr) = b.value().attr("data-app-data") {
                let v: Value =
                    serde_json::from_str(attr).map_err(|e| FunPayError::Parse(e.to_string()))?;
                let user_id = v
                    .get("userId")
                    .and_then(|x| x.as_i64())
                    .ok_or_else(|| FunPayError::Parse(String::from("missing userId")))?;
                let csrf = v
                    .get("csrf-token")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| FunPayError::Parse(String::from("missing csrf-token")))?;
                if let Some(loc) = v.get("locale").and_then(|x| x.as_str()) {
                    self.locale = Some(loc.to_string());
                }
                app_data = Some(AppData {
                    user_id,
                    csrf_token: csrf.to_string(),
                });
            }
        }
        let sel_uname = Selector::parse("div.user-link-name").unwrap();
        let username = html
            .select(&sel_uname)
            .next()
            .map(|n| n.text().collect::<String>());
        if username.is_none() {
            return Err(FunPayError::Unauthorized);
        }
        let app = app_data.ok_or_else(|| FunPayError::Parse(String::from("missing app data")))?;
        self.id = Some(app.user_id);
        self.csrf_token = Some(app.csrf_token);
        self.username = username;
        self.setup_subcategories(&body);
        Ok(())
    }

    fn setup_subcategories(&mut self, html: &str) {
        let doc = Html::parse_document(html);
        let sel_lists = Selector::parse("div.promo-game-list").unwrap();
        let mut lists: Vec<_> = doc.select(&sel_lists).collect();
        if lists.is_empty() {
            return;
        }
        let container = if lists.len() > 1 {
            lists.remove(1)
        } else {
            lists.remove(0)
        };
        let sel_item = Selector::parse("div.promo-game-item").unwrap();
        let sel_ul = Selector::parse("ul.list-inline").unwrap();
        let sel_a = Selector::parse("a").unwrap();
        let re_id = Regex::new(r"/(?:chips|lots)/(\d+)/?").unwrap();
        for game in container.select(&sel_item) {
            for ul in game.select(&sel_ul) {
                for li in ul.children() {
                    if let Some(el) = li.value().as_element() {
                        if el.name() != "li" {
                            continue;
                        }
                    } else {
                        continue;
                    }
                    if let Some(a) = li
                        .first_child()
                        .and_then(|n| n.value().as_element())
                        .and_then(|_| ul.select(&sel_a).next())
                    {
                        let name = a.text().collect::<String>().trim().to_string();
                        if name.is_empty() {
                            continue;
                        }
                        let href = a.value().attr("href").unwrap_or("");
                        let typ = if href.contains("chips/") {
                            SubcategoryType::Currency
                        } else {
                            SubcategoryType::Common
                        };
                        let id = re_id
                            .captures(href)
                            .and_then(|c| c.get(1))
                            .and_then(|m| m.as_str().parse::<i64>().ok());
                        if let Some(sid) = id {
                            let sub = Subcategory {
                                id: Some(sid),
                                name: name.clone(),
                            };
                            let entry = self.sorted_subcategories.entry(typ).or_default();
                            entry.insert(sid, sub);
                        }
                    }
                }
            }
        }
    }

    pub async fn start_polling_loop(&mut self) -> Result<(), FunPayError> {
        let poller = FunPayPoller {
            gateway: self.gateway.clone(),
            golden_key: self.golden_key.clone(),
            user_agent: self.user_agent.clone(),
            id: self.id.ok_or(FunPayError::AccountNotInitiated)?,
            username: self.username.clone(),
            csrf_token: self
                .csrf_token
                .clone()
                .ok_or(FunPayError::AccountNotInitiated)?,
            phpsessid: self.phpsessid.clone(),
            events_tx: self.events_tx.clone(),
            storage: self.storage.clone(),
            polling_interval: self.polling_interval,
            error_retry_delay: self.error_retry_delay,
            last_msg_event_tag: random_tag(),
            last_order_event_tag: random_tag(),
            last_messages: HashMap::new(),
            last_messages_ids: HashMap::new(),
            saved_orders: HashMap::new(),
        };
        poller.start().await
    }
}

impl FunPaySender {
    pub async fn send_chat_message(&self, chat_id: &str, content: &str) -> Result<(), FunPayError> {
        let mut csrf_to_use = self.csrf_token.clone();
        let mut phpsess_to_use = self.phpsessid.clone();
        if phpsess_to_use.is_none() {
            let (body, set_cookies) = self
                .gateway
                .get_chat_page(&self.golden_key, &self.user_agent, chat_id)
                .await?;
            let html = Html::parse_document(&body);
            let sel_body = Selector::parse("body").unwrap();
            if let Some(b) = html.select(&sel_body).next() {
                if let Some(attr) = b.value().attr("data-app-data") {
                    if let Ok(v) = serde_json::from_str::<Value>(attr) {
                        if let Some(csrf) = v.get("csrf-token").and_then(|x| x.as_str()) {
                            csrf_to_use = csrf.to_string();
                        }
                    }
                }
            }
            if let Some(sess) = extract_phpsessid(&set_cookies) {
                phpsess_to_use = Some(sess);
            }
        }
        let objects_json = to_string(&vec![serde_json::json!({
            "type": "chat_node",
            "id": chat_id,
            "tag": "00000000",
            "data": {"node": chat_id, "last_message": -1, "content": ""}
        })])
        .unwrap();
        let request_json = json!({
            "action": "chat_message",
            "data": {"node": chat_id, "last_message": -1, "content": content}
        })
        .to_string();
        self.gateway
            .post_runner(
                &self.golden_key,
                &self.user_agent,
                &csrf_to_use,
                phpsess_to_use.as_deref(),
                &objects_json,
                Some(&request_json),
            )
            .await
            .map(|_| ())
    }

    pub async fn get_chat_messages(&self, chat_id: &str) -> Result<Vec<Message>, FunPayError> {
        let objects_json = to_string(&vec![serde_json::json!({
            "type": "chat_node",
            "id": chat_id,
            "tag": "00000000",
            "data": {"node": chat_id, "last_message": -1, "content": ""}
        })])
        .unwrap();

        let res = self
            .gateway
            .post_runner(
                &self.golden_key,
                &self.user_agent,
                &self.csrf_token,
                self.phpsessid.as_deref(),
                &objects_json,
                None,
            )
            .await?;

        let objects = res
            .get("objects")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();

        for obj in objects {
            if obj.get("type").and_then(|x| x.as_str()) != Some("chat_node") {
                continue;
            }

            let data = match obj.get("data") {
                Some(d) => d,
                None => continue,
            };

            let messages = data
                .get("messages")
                .and_then(|x| x.as_array())
                .cloned()
                .unwrap_or_default();

            let mut list = Vec::new();
            for m in messages {
                let mid = m.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
                let author_id = m.get("author").and_then(|x| x.as_i64()).unwrap_or(0);
                let html = m.get("html").and_then(|x| x.as_str()).unwrap_or("");
                let (text, _image) = parse_message_html(html);
                list.push(Message {
                    id: mid,
                    chat_id: ChatId::from(chat_id.to_string()),
                    chat_name: None,
                    text,
                    interlocutor_id: None,
                    author_id,
                });
            }
            return Ok(list);
        }

        Ok(Vec::new())
    }

    pub fn get_chat_id_for_user(&self, user_id: i64) -> String {
        let my_id = self.seller_id;
        let (id1, id2) = (my_id.min(user_id), my_id.max(user_id));
        format!("users-{id1}-{id2}")
    }

    pub async fn get_order_secrets(&self, order_id: &str) -> Result<Vec<String>, FunPayError> {
        let body = self
            .gateway
            .get_order_page(&self.golden_key, &self.user_agent, order_id)
            .await?;
        let doc = Html::parse_document(&body);
        Ok(parse_order_secrets(&doc))
    }

    pub async fn get_order(&self, order_id: &str) -> Result<Order, FunPayError> {
        let body = self
            .gateway
            .get_order_page(&self.golden_key, &self.user_agent, order_id)
            .await?;
        parse_order_page(&body, order_id)
    }

    pub async fn edit_offer(
        &self,
        offer_id: i64,
        node_id: i64,
        params: OfferEditParams,
    ) -> Result<Value, FunPayError> {
        let html = self
            .gateway
            .get_offer_edit_page(&self.golden_key, &self.user_agent, node_id, offer_id)
            .await?;

        let current = parse_offer_edit_params(&html);
        log::debug!(
            target: "funpay_client",
            "Parsed offer {} current params: quantity={:?}, method={:?}, price={:?}",
            offer_id,
            current.quantity,
            current.method,
            current.price
        );
        let merged = current.merge(params);
        log::debug!(
            target: "funpay_client",
            "Merged offer {} params: quantity={:?}, method={:?}, price={:?}",
            offer_id,
            merged.quantity,
            merged.method,
            merged.price
        );

        self.gateway
            .post_offer_save(OfferSaveRequest {
                golden_key: &self.golden_key,
                user_agent: &self.user_agent,
                phpsessid: self.phpsessid.as_deref(),
                csrf: &self.csrf_token,
                offer_id,
                node_id,
                params: &merged,
            })
            .await
    }

    pub async fn get_offer_params(
        &self,
        offer_id: i64,
        node_id: i64,
    ) -> Result<OfferFullParams, FunPayError> {
        let html = self
            .gateway
            .get_offer_edit_page(&self.golden_key, &self.user_agent, node_id, offer_id)
            .await?;
        Ok(parse_offer_full_params(&html, offer_id, node_id))
    }

    pub async fn get_my_offers(&self, node_id: i64) -> Result<Vec<Offer>, FunPayError> {
        let html = self
            .gateway
            .get_lots_trade_page(&self.golden_key, &self.user_agent, node_id)
            .await?;
        Ok(parse_my_offers(&html, node_id))
    }

    pub async fn get_market_offers(&self, node_id: i64) -> Result<Vec<MarketOffer>, FunPayError> {
        let html = self
            .gateway
            .get_lots_page(&self.golden_key, &self.user_agent, node_id)
            .await?;
        Ok(parse_market_offers(&html, node_id))
    }

    pub async fn get_orders(&self) -> Result<Vec<OrderShortcut>, FunPayError> {
        let body = self
            .gateway
            .get_orders_trade(&self.golden_key, &self.user_agent)
            .await?;
        parse_orders_list(&body, self.seller_id)
    }

    pub async fn get_category_subcategories(
        &self,
        node_id: i64,
    ) -> Result<Vec<CategorySubcategory>, FunPayError> {
        let html = self
            .gateway
            .get_lots_page(&self.golden_key, &self.user_agent, node_id)
            .await?;
        Ok(parse_category_subcategories(&html))
    }

    pub async fn get_category_filters(
        &self,
        node_id: i64,
    ) -> Result<Vec<CategoryFilter>, FunPayError> {
        let html = self
            .gateway
            .get_lots_page(&self.golden_key, &self.user_agent, node_id)
            .await?;
        Ok(parse_category_filters(&html))
    }
}
