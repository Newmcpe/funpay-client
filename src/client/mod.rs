use crate::error::FunPayError;
use crate::models::OfferEditParams;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait FunpayGateway: Send + Sync {
    async fn get_home(
        &self,
        golden_key: &str,
        user_agent: &str,
    ) -> Result<(String, Vec<String>), FunPayError>;
    async fn get_chat_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        chat_id: &str,
    ) -> Result<(String, Vec<String>), FunPayError>;
    async fn get_orders_trade(
        &self,
        golden_key: &str,
        user_agent: &str,
    ) -> Result<String, FunPayError>;
    async fn get_order_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        order_id: &str,
    ) -> Result<String, FunPayError>;
    async fn post_runner(
        &self,
        golden_key: &str,
        user_agent: &str,
        csrf: &str,
        phpsessid: Option<&str>,
        objects_json: &str,
        request_json: Option<&str>,
    ) -> Result<Value, FunPayError>;
    async fn post_offer_save(
        &self,
        golden_key: &str,
        user_agent: &str,
        phpsessid: Option<&str>,
        csrf: &str,
        offer_id: i64,
        node_id: i64,
        params: &OfferEditParams,
    ) -> Result<Value, FunPayError>;
    async fn get_offer_edit_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
        offer_id: i64,
    ) -> Result<String, FunPayError>;
    async fn get_lots_trade_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
    ) -> Result<String, FunPayError>;
    async fn get_lots_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
    ) -> Result<String, FunPayError>;
}

pub mod account;
pub mod http;
pub mod poller;
pub mod urls;
