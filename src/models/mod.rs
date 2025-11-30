pub mod enums;
pub mod ids;

use crate::models::enums::OrderStatus;
use crate::models::ids::{ChatId, OrderId};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct OfferEditParams {
    pub quantity: Option<String>,
    pub quantity2: Option<String>,
    pub method: Option<String>,
    pub offer_type: Option<String>,
    pub server_id: Option<String>,
    pub desc_ru: Option<String>,
    pub desc_en: Option<String>,
    pub payment_msg_ru: Option<String>,
    pub payment_msg_en: Option<String>,
    pub summary_ru: Option<String>,
    pub summary_en: Option<String>,
    pub game: Option<String>,
    pub images: Option<String>,
    pub price: Option<String>,
    pub deactivate_after_sale: Option<bool>,
    pub active: Option<bool>,
    pub location: Option<String>,
    pub deleted: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct OfferFullParams {
    pub offer_id: i64,
    pub node_id: i64,
    pub quantity: Option<String>,
    pub quantity2: Option<String>,
    pub method: Option<String>,
    pub offer_type: Option<String>,
    pub server_id: Option<String>,
    pub desc_ru: Option<String>,
    pub desc_en: Option<String>,
    pub payment_msg_ru: Option<String>,
    pub payment_msg_en: Option<String>,
    pub images: Option<String>,
    pub price: Option<String>,
    pub deactivate_after_sale: bool,
    pub active: bool,
    pub location: Option<String>,
    pub custom_fields: Vec<OfferCustomField>,
}

#[derive(Debug, Clone)]
pub struct OfferCustomField {
    pub name: String,
    pub label: String,
    pub field_type: OfferFieldType,
    pub value: String,
    pub options: Vec<OfferFieldOption>,
}

#[derive(Debug, Clone)]
pub struct OfferFieldOption {
    pub value: String,
    pub label: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OfferFieldType {
    Text,
    Textarea,
    Select,
    Checkbox,
    Hidden,
    Unknown(String),
}

impl OfferEditParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_price(mut self, price: impl Into<String>) -> Self {
        self.price = Some(price.into());
        self
    }

    pub fn with_quantity(mut self, quantity: impl Into<String>) -> Self {
        self.quantity = Some(quantity.into());
        self
    }

    pub fn with_desc_ru(mut self, desc: impl Into<String>) -> Self {
        self.desc_ru = Some(desc.into());
        self
    }

    pub fn with_desc_en(mut self, desc: impl Into<String>) -> Self {
        self.desc_en = Some(desc.into());
        self
    }

    pub fn with_method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn with_server_id(mut self, server_id: impl Into<String>) -> Self {
        self.server_id = Some(server_id.into());
        self
    }

    pub fn with_deactivate_after_sale(mut self, deactivate: bool) -> Self {
        self.deactivate_after_sale = Some(deactivate);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn with_images(mut self, images: impl Into<String>) -> Self {
        self.images = Some(images.into());
        self
    }

    pub fn with_payment_msg_ru(mut self, msg: impl Into<String>) -> Self {
        self.payment_msg_ru = Some(msg.into());
        self
    }

    pub fn with_payment_msg_en(mut self, msg: impl Into<String>) -> Self {
        self.payment_msg_en = Some(msg.into());
        self
    }

    pub fn with_deleted(mut self, deleted: bool) -> Self {
        self.deleted = Some(deleted);
        self
    }

    pub fn merge(self, other: OfferEditParams) -> Self {
        Self {
            quantity: other.quantity.filter(|s| !s.is_empty()).or(self.quantity),
            quantity2: other.quantity2.filter(|s| !s.is_empty()).or(self.quantity2),
            method: other.method.filter(|s| !s.is_empty()).or(self.method),
            offer_type: other
                .offer_type
                .filter(|s| !s.is_empty())
                .or(self.offer_type),
            server_id: other.server_id.filter(|s| !s.is_empty()).or(self.server_id),
            desc_ru: other.desc_ru.filter(|s| !s.is_empty()).or(self.desc_ru),
            desc_en: other.desc_en.filter(|s| !s.is_empty()).or(self.desc_en),
            payment_msg_ru: other
                .payment_msg_ru
                .filter(|s| !s.is_empty())
                .or(self.payment_msg_ru),
            payment_msg_en: other
                .payment_msg_en
                .filter(|s| !s.is_empty())
                .or(self.payment_msg_en),
            summary_ru: other
                .summary_ru
                .filter(|s| !s.is_empty())
                .or(self.summary_ru),
            summary_en: other
                .summary_en
                .filter(|s| !s.is_empty())
                .or(self.summary_en),
            game: other.game.filter(|s| !s.is_empty()).or(self.game),
            images: other.images.filter(|s| !s.is_empty()).or(self.images),
            price: other.price.filter(|s| !s.is_empty()).or(self.price),
            deactivate_after_sale: other.deactivate_after_sale.or(self.deactivate_after_sale),
            active: other.active.or(self.active),
            location: other.location.filter(|s| !s.is_empty()).or(self.location),
            deleted: other.deleted.or(self.deleted),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Offer {
    pub id: i64,
    pub node_id: i64,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct MarketOffer {
    pub id: i64,
    pub node_id: i64,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub seller_id: i64,
    pub seller_name: String,
    pub seller_online: bool,
    pub seller_rating: Option<f64>,
    pub seller_reviews: u32,
    pub is_promo: bool,
}

#[derive(Debug, Clone)]
pub struct ChatShortcut {
    pub id: i64,
    pub name: String,
    pub last_message_text: Option<String>,
    pub node_msg_id: i64,
    pub user_msg_id: i64,
    pub unread: bool,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: i64,
    pub chat_id: ChatId,
    pub chat_name: Option<String>,
    pub text: Option<String>,
    pub interlocutor_id: Option<i64>,
    pub author_id: i64,
}

#[derive(Debug, Clone)]
pub struct OrderShortcut {
    pub id: OrderId,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub buyer_username: String,
    pub buyer_id: i64,
    pub chat_id: ChatId,
    pub status: OrderStatus,
    pub date_text: String,
    pub subcategory: Subcategory,
    pub amount: i32,
}

#[derive(Debug, Clone)]
pub struct Review {
    pub stars: Option<i32>,
    pub text: Option<String>,
    pub reply: Option<String>,
    pub anonymous: bool,
    pub html: String,
    pub hidden: bool,
    pub order_id: Option<OrderId>,
    pub author: Option<String>,
    pub author_id: Option<i64>,
    pub by_bot: bool,
    pub reply_by_bot: bool,
}

#[derive(Debug, Clone)]
pub struct Subcategory {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CategorySubcategory {
    pub id: i64,
    pub name: String,
    pub offer_count: u32,
    pub subcategory_type: CategorySubcategoryType,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategorySubcategoryType {
    Lots,
    Chips,
}

#[derive(Debug, Clone)]
pub struct CategoryFilter {
    pub id: String,
    pub name: String,
    pub filter_type: CategoryFilterType,
    pub options: Vec<CategoryFilterOption>,
}

#[derive(Debug, Clone)]
pub struct CategoryFilterOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryFilterType {
    Select,
    RadioBox,
    Range,
    Checkbox,
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: OrderId,
    pub status: OrderStatus,
    pub lot_params: Vec<(String, String)>,
    pub buyer_params: HashMap<String, String>,
    pub short_description: Option<String>,
    pub full_description: Option<String>,
    pub subcategory: Option<Subcategory>,
    pub amount: i32,
    pub sum: f64,
    pub currency: String,
    pub buyer_id: i64,
    pub buyer_username: String,
    pub seller_id: i64,
    pub seller_username: String,
    pub chat_id: ChatId,
    pub html: String,
    pub review: Option<Review>,
    pub order_secrets: Vec<String>,
}
