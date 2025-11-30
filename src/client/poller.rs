use crate::client::FunpayGateway;
use crate::error::FunPayError;
use crate::events::Event;
use crate::models::ids::{ChatId, OrderId};
use crate::models::{ChatShortcut, Message, OrderShortcut};
use crate::parsing::{parse_message_html, parse_orders_list};
use crate::storage::StateStorage;
use log::debug;
use scraper::{Html, Selector};
use serde_json::{json, to_string, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::time::sleep;

pub struct FunPayPoller {
    pub gateway: Arc<dyn FunpayGateway>,
    pub golden_key: String,
    pub user_agent: String,
    pub id: i64,
    pub username: Option<String>,
    pub csrf_token: String,
    pub phpsessid: Option<String>,
    pub events_tx: Sender<Event>,
    pub storage: Arc<dyn StateStorage>,
    pub polling_interval: Duration,
    pub error_retry_delay: Duration,

    // State
    pub last_msg_event_tag: String,
    pub last_order_event_tag: String,
    pub last_messages: HashMap<i64, (i64, i64, Option<String>)>,
    pub last_messages_ids: HashMap<i64, i64>,
    pub saved_orders: HashMap<OrderId, OrderShortcut>,
}

impl FunPayPoller {
    pub async fn start(mut self) -> Result<(), FunPayError> {
        self.load_last_messages_ids().await;
        debug!(
            target: "funpay_client",
            "Starting polling loop for {}",
            self.username.clone().unwrap_or_default()
        );

        let mut first = true;
        loop {
            let orders = json!({
                "type": "orders_counters",
                "id": self.id,
                "tag": self.last_order_event_tag,
                "data": false
            });
            let chats = json!({
                "type": "chat_bookmarks",
                "id": self.id,
                "tag": self.last_msg_event_tag,
                "data": false
            });
            let objects_json = to_string(&json!([orders, chats])).unwrap();

            let updates = match self.post_runner(objects_json).await {
                Ok(updates) => updates,
                Err(e) => {
                    log::error!(target: "funpay_client", "HTTP request failed: {e}. Retrying in {:?}...", self.error_retry_delay);
                    sleep(self.error_retry_delay).await;
                    continue;
                }
            };

            let (evs, changed_chats) = self.parse_events_from_updates(&updates, first);
            for ev in evs {
                let _ = self.events_tx.send(ev);
            }

            let mut persist_required = false;
            if !changed_chats.is_empty() {
                match self.fetch_chats_histories(&changed_chats).await {
                    Ok(mut histories) => {
                        for (cid, mut msgs) in histories.drain() {
                            if let Some(last_id) = self.last_messages_ids.get(&cid).copied() {
                                msgs.retain(|m| m.id > last_id);
                            }
                            if let Some(max_id) = msgs.iter().map(|m| m.id).max() {
                                let prev = self.last_messages_ids.insert(cid, max_id);
                                if prev.map_or(true, |value| value != max_id) {
                                    persist_required = true;
                                }
                            }
                            if !first {
                                for m in msgs {
                                    let _ = self.events_tx.send(Event::NewMessage { message: m });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!(target: "funpay_client", "Failed to fetch chat histories: {e}");
                    }
                }
            }

            if persist_required {
                self.persist_last_messages_ids().await;
            }

            match self.fetch_sales_list().await {
                Ok(list) => {
                    let mut new_map: HashMap<OrderId, OrderShortcut> = HashMap::new();
                    for o in list.into_iter() {
                        new_map.insert(o.id.clone(), o);
                    }
                    if self.saved_orders.is_empty() {
                        for order in new_map.values() {
                            let _ = self.events_tx.send(Event::InitialOrder {
                                order: order.clone(),
                            });
                        }
                    } else {
                        for (id, order) in new_map.iter() {
                            if let Some(prev) = self.saved_orders.get(id) {
                                if prev.status != order.status {
                                    let _ = self.events_tx.send(Event::OrderStatusChanged {
                                        order: order.clone(),
                                    });
                                }
                            } else {
                                let _ = self.events_tx.send(Event::NewOrder {
                                    order: order.clone(),
                                });
                                if order.status == crate::models::enums::OrderStatus::Closed {
                                    let _ = self.events_tx.send(Event::OrderStatusChanged {
                                        order: order.clone(),
                                    });
                                }
                            }
                        }
                    }
                    self.saved_orders = new_map;
                }
                Err(e) => {
                    log::error!(target: "funpay_client", "Failed to fetch sales list: {e}");
                }
            }

            first = false;
            sleep(self.polling_interval).await;
        }
    }

    async fn post_runner(&self, objects_json: String) -> Result<Value, FunPayError> {
        self.gateway
            .post_runner(
                &self.golden_key,
                &self.user_agent,
                &self.csrf_token,
                self.phpsessid.as_deref(),
                &objects_json,
                None,
            )
            .await
    }

    fn parse_events_from_updates(
        &mut self,
        updates: &Value,
        first: bool,
    ) -> (Vec<Event>, Vec<(i64, Option<String>)>) {
        let mut events = Vec::new();
        let mut changed_chats: Vec<ChatShortcut> = Vec::new();
        let objects = updates
            .get("objects")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        for obj in objects {
            let typ = obj.get("type").and_then(|x| x.as_str()).unwrap_or("");
            if typ == "chat_bookmarks" {
                if let Some(tag) = obj.get("tag").and_then(|x| x.as_str()) {
                    self.last_msg_event_tag = tag.to_string();
                }
                let html = obj
                    .get("data")
                    .and_then(|x| x.get("html"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if html.is_empty() {
                    continue;
                }
                let chats = self.parse_chat_bookmarks(html);
                if first {
                    for ch in chats {
                        events.push(Event::InitialChat { chat: ch.clone() });
                        if ch.node_msg_id > 0 {
                            changed_chats.push(ch);
                        }
                    }
                } else {
                    if !chats.is_empty() {
                        events.push(Event::ChatsListChanged);
                    }
                    for ch in chats {
                        let prev = self
                            .last_messages
                            .get(&ch.id)
                            .cloned()
                            .unwrap_or((-1, -1, None));
                        if ch.node_msg_id > prev.0 {
                            events.push(Event::LastChatMessageChanged { chat: ch.clone() });
                            changed_chats.push(ch.clone());
                        }
                        self.last_messages.insert(
                            ch.id,
                            (ch.node_msg_id, ch.user_msg_id, ch.last_message_text.clone()),
                        );
                    }
                }
            } else if typ == "orders_counters" {
                if let Some(tag) = obj.get("tag").and_then(|x| x.as_str()) {
                    self.last_order_event_tag = tag.to_string();
                }
                let purchases = obj
                    .get("data")
                    .and_then(|x| x.get("buyer"))
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0) as i32;
                let sales = obj
                    .get("data")
                    .and_then(|x| x.get("seller"))
                    .and_then(|x| x.as_i64())
                    .unwrap_or(0) as i32;
                events.push(Event::OrdersListChanged { purchases, sales });
            }
        }
        let chats_data: Vec<(i64, Option<String>)> = changed_chats
            .into_iter()
            .map(|c| (c.id, Some(c.name)))
            .collect();
        (events, chats_data)
    }

    fn parse_chat_bookmarks(&mut self, html: &str) -> Vec<ChatShortcut> {
        let doc = Html::parse_fragment(html);
        let sel_chat = Selector::parse("a.contact-item").unwrap();
        let sel_msg = Selector::parse("div.contact-item-message").unwrap();
        let sel_name = Selector::parse("div.media-user-name").unwrap();
        let mut out = Vec::new();
        for el in doc.select(&sel_chat) {
            let id_attr = el.value().attr("data-id").unwrap_or("0");
            let id = id_attr.parse::<i64>().unwrap_or(0);
            let node_msg_id = el
                .value()
                .attr("data-node-msg")
                .unwrap_or("0")
                .parse::<i64>()
                .unwrap_or(0);
            let user_msg_id = el
                .value()
                .attr("data-user-msg")
                .unwrap_or("0")
                .parse::<i64>()
                .unwrap_or(0);
            let unread = el.value().classes().any(|c| c == "unread");
            let last_message_text = el
                .select(&sel_msg)
                .next()
                .map(|n| n.text().collect::<String>());
            let name = el
                .select(&sel_name)
                .next()
                .map(|n| n.text().collect::<String>())
                .unwrap_or_default();
            out.push(ChatShortcut {
                id,
                name,
                last_message_text,
                node_msg_id,
                user_msg_id,
                unread,
            });
        }
        out
    }

    async fn fetch_sales_list(&self) -> Result<Vec<OrderShortcut>, FunPayError> {
        let body = self
            .gateway
            .get_orders_trade(&self.golden_key, &self.user_agent)
            .await?;
        parse_orders_list(&body, self.id)
    }

    async fn fetch_chats_histories(
        &self,
        chats_data: &[(i64, Option<String>)],
    ) -> Result<HashMap<i64, Vec<Message>>, FunPayError> {
        let mut objects = Vec::with_capacity(chats_data.len());
        for (chat_id, _name) in chats_data.iter() {
            objects.push(json!({
                "type": "chat_node",
                "id": chat_id,
                "tag": "00000000",
                "data": {"node": chat_id, "last_message": -1, "content": ""}
            }));
        }
        let objects_json = to_string(&objects).unwrap();
        let res = self.post_runner(objects_json).await?;
        let mut out: HashMap<i64, Vec<Message>> = HashMap::new();
        let objects = res
            .get("objects")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        for obj in objects {
            if obj.get("type").and_then(|x| x.as_str()) != Some("chat_node") {
                continue;
            }
            let id = obj.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
            let data = obj.get("data");
            if data.is_none() {
                out.insert(id, Vec::new());
                continue;
            }
            let data = data.unwrap();
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
                    chat_id: ChatId::from(format!("{id}")),
                    chat_name: chats_data
                        .iter()
                        .find(|(cid, _)| *cid == id)
                        .and_then(|(_, n)| n.clone()),
                    text,
                    interlocutor_id: None,
                    author_id,
                });
            }
            out.insert(id, list);
        }
        Ok(out)
    }

    async fn load_last_messages_ids(&mut self) {
        match self.storage.load().await {
            Ok(stored) => {
                self.last_messages_ids = stored;
            }
            Err(e) => {
                log::error!(target: "funpay_client", "Failed to load last messages store: {e}");
            }
        }
    }

    async fn persist_last_messages_ids(&self) {
        if let Err(e) = self.storage.save(&self.last_messages_ids).await {
            log::error!(target: "funpay_client", "Failed to persist last messages ids: {e}");
        }
    }
}
