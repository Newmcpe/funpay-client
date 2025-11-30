use crate::client::urls::UrlBuilder;
use crate::client::FunpayGateway;
use crate::config::FunPayConfig;
use crate::error::FunPayError;
use crate::models::OfferEditParams;
use async_trait::async_trait;
use reqwest::{header, redirect::Policy, Client, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReqwestGateway {
    pub client: ClientWithMiddleware,
    pub urls: UrlBuilder,
}

impl ReqwestGateway {
    pub fn new() -> Self {
        Self::with_config(&FunPayConfig::default())
    }

    pub fn with_config(config: &FunPayConfig) -> Self {
        let retry_policy = ExponentialBackoff::builder()
            .base(config.retry_base_ms)
            .build_with_max_retries(config.max_retries);

        let client = ClientBuilder::new(
            Client::builder()
                .redirect(Policy::limited(config.redirect_limit))
                .build()
                .unwrap(),
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        Self {
            client,
            urls: UrlBuilder::new(&config.base_url),
        }
    }

    pub fn with_proxy(proxy_url: &str) -> Self {
        Self::with_proxy_and_config(proxy_url, &FunPayConfig::default())
    }

    pub fn with_proxy_and_config(proxy_url: &str, config: &FunPayConfig) -> Self {
        fn normalize_proxy_url(raw: &str) -> String {
            if raw.contains('@') {
                return raw.to_string();
            }
            if let Some((scheme, rest)) = raw.split_once("://") {
                let parts: Vec<&str> = rest.split(':').collect();
                if parts.len() == 4 {
                    let host = parts[0];
                    let port = parts[1];
                    let user = parts[2];
                    let pass = parts[3];
                    return format!("{scheme}://{user}:{pass}@{host}:{port}");
                }
            }
            raw.to_string()
        }

        let retry_policy = ExponentialBackoff::builder()
            .base(config.retry_base_ms)
            .build_with_max_retries(config.max_retries);

        let normalized = normalize_proxy_url(proxy_url);
        let client = ClientBuilder::new(
            Client::builder()
                .redirect(Policy::limited(config.redirect_limit))
                .proxy(reqwest::Proxy::all(&normalized).expect("invalid proxy url"))
                .build()
                .expect("failed to build reqwest client with proxy"),
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        Self {
            client,
            urls: UrlBuilder::new(&config.base_url),
        }
    }

    fn add_common_headers(
        &self,
        builder: reqwest_middleware::RequestBuilder,
        golden_key: &str,
        user_agent: &str,
        phpsessid: Option<&str>,
    ) -> reqwest_middleware::RequestBuilder {
        let cookie = if let Some(sess) = phpsessid {
            format!("golden_key={golden_key}; cookie_prefs=1; PHPSESSID={sess}")
        } else {
            format!("golden_key={golden_key}; cookie_prefs=1")
        };

        builder
            .header(header::COOKIE, cookie)
            .header(header::USER_AGENT, user_agent)
    }

    async fn execute(
        &self,
        builder: reqwest_middleware::RequestBuilder,
    ) -> Result<reqwest::Response, FunPayError> {
        let resp = builder.send().await?;
        if resp.status() == StatusCode::FORBIDDEN {
            return Err(FunPayError::Unauthorized);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let url = resp.url().to_string();
            let body = resp.text().await.unwrap_or_default();
            return Err(FunPayError::RequestFailed { status, body, url });
        }
        Ok(resp)
    }
}

impl Default for ReqwestGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FunpayGateway for ReqwestGateway {
    async fn get_home(
        &self,
        golden_key: &str,
        user_agent: &str,
    ) -> Result<(String, Vec<String>), FunPayError> {
        let url = self.urls.home();
        let req = self.client.get(&url);
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;

        let set_cookies: Vec<String> = resp
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let body = resp.text().await?;
        Ok((body, set_cookies))
    }

    async fn get_chat_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        chat_id: &str,
    ) -> Result<(String, Vec<String>), FunPayError> {
        let chat_url = self.urls.chat_page(chat_id);
        let req = self.client.get(&chat_url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;

        let set_cookies: Vec<String> = resp
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
            .collect();
        let body = resp.text().await.unwrap_or_default();
        Ok((body, set_cookies))
    }

    async fn get_orders_trade(
        &self,
        golden_key: &str,
        user_agent: &str,
    ) -> Result<String, FunPayError> {
        let url = self.urls.orders_trade();
        let req = self.client.get(&url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;
        let body = resp.text().await?;
        Ok(body)
    }

    async fn get_order_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        order_id: &str,
    ) -> Result<String, FunPayError> {
        let url = self.urls.order_page(order_id);
        let req = self.client.get(&url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;
        let body = resp.text().await?;
        Ok(body)
    }

    async fn post_runner(
        &self,
        golden_key: &str,
        user_agent: &str,
        csrf: &str,
        phpsessid: Option<&str>,
        objects_json: &str,
        request_json: Option<&str>,
    ) -> Result<Value, FunPayError> {
        let url = self.urls.runner();
        let payload = match request_json {
            Some(req) => format!(
                "objects={}&request={}&csrf_token={}",
                urlencoding::encode(objects_json),
                urlencoding::encode(req),
                urlencoding::encode(csrf)
            ),
            None => format!(
                "objects={}&request=false&csrf_token={}",
                urlencoding::encode(objects_json),
                urlencoding::encode(csrf)
            ),
        };

        let req = self
            .client
            .post(&url)
            .header(
                header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("x-requested-with", "XMLHttpRequest")
            .header(header::ACCEPT, "*/*")
            .header(header::ORIGIN, self.urls.base_url())
            .header(header::REFERER, format!("{}/chat/", self.urls.base_url()))
            .body(payload);

        let req = self.add_common_headers(req, golden_key, user_agent, phpsessid);
        let resp = self.execute(req).await?;
        let v: Value = resp.json().await?;
        Ok(v)
    }

    async fn post_offer_save(
        &self,
        golden_key: &str,
        user_agent: &str,
        phpsessid: Option<&str>,
        csrf: &str,
        offer_id: i64,
        node_id: i64,
        params: &OfferEditParams,
    ) -> Result<Value, FunPayError> {
        let url = self.urls.offer_save();
        let form_created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let field = |key: &str, val: Option<&str>| {
            format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(val.unwrap_or(""))
            )
        };

        let mut form_parts = vec![
            format!("csrf_token={}", urlencoding::encode(csrf)),
            format!("form_created_at={form_created_at}"),
            format!("offer_id={offer_id}"),
            format!("node_id={node_id}"),
            field("location", params.location.as_deref()),
            format!(
                "deleted={}",
                if params.deleted.unwrap_or(false) {
                    "1"
                } else {
                    ""
                }
            ),
            field("fields[quantity]", params.quantity.as_deref()),
            field("fields[quantity2]", params.quantity2.as_deref()),
            field("fields[method]", params.method.as_deref()),
            field("fields[type]", params.offer_type.as_deref()),
            field("server_id", params.server_id.as_deref()),
            field("fields[desc][ru]", params.desc_ru.as_deref()),
            field("fields[desc][en]", params.desc_en.as_deref()),
            field("fields[payment_msg][ru]", params.payment_msg_ru.as_deref()),
            field("fields[payment_msg][en]", params.payment_msg_en.as_deref()),
            field("fields[summary][ru]", params.summary_ru.as_deref()),
            field("fields[summary][en]", params.summary_en.as_deref()),
            field("fields[game]", params.game.as_deref()),
            field("fields[images]", params.images.as_deref()),
            field("price", params.price.as_deref()),
        ];

        if params.deactivate_after_sale.unwrap_or(false) {
            form_parts.push(field("deactivate_after_sale[]", None));
            form_parts.push(field("deactivate_after_sale[]", Some("on")));
        } else {
            form_parts.push(field("deactivate_after_sale", None));
        }

        if params.active.unwrap_or(true) {
            form_parts.push(field("active", Some("on")));
        } else {
            form_parts.push(field("active", None));
        }

        let payload = form_parts.join("&");
        let referer = self.urls.offer_edit(node_id, offer_id);

        log::debug!(
            target: "funpay_client",
            "POST {} | offer_id={} node_id={} price={:?}\nPayload: {}",
            url,
            offer_id,
            node_id,
            params.price,
            payload
        );

        let req = self
            .client
            .post(&url)
            .header(
                header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("x-requested-with", "XMLHttpRequest")
            .header(
                header::ACCEPT,
                "application/json, text/javascript, */*; q=0.01",
            )
            .header(header::ORIGIN, self.urls.base_url())
            .header(header::REFERER, referer)
            .body(payload);

        let req = self.add_common_headers(req, golden_key, user_agent, phpsessid);
        let resp = self.execute(req).await?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        log::info!(
            target: "funpay_client",
            "Response from offerSave: status={} body={}",
            status, // accessing status after check is fine since it's Copy, but I should've saved it or execute return logic...
            // Wait, execute returns Response.
            body_text
        );
        // execute already checks status.

        let v: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
        Ok(v)
    }

    async fn get_offer_edit_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
        offer_id: i64,
    ) -> Result<String, FunPayError> {
        let url = self.urls.offer_edit(node_id, offer_id);
        let req = self.client.get(&url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;
        let body = resp.text().await?;
        Ok(body)
    }

    async fn get_lots_trade_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
    ) -> Result<String, FunPayError> {
        let url = self.urls.lots_trade(node_id);
        let req = self.client.get(&url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;
        let body = resp.text().await?;
        Ok(body)
    }

    async fn get_lots_page(
        &self,
        golden_key: &str,
        user_agent: &str,
        node_id: i64,
    ) -> Result<String, FunPayError> {
        let url = self.urls.lots_page(node_id);
        let req = self.client.get(&url).header(header::ACCEPT, "*/*");
        let req = self.add_common_headers(req, golden_key, user_agent, None);
        let resp = self.execute(req).await?;
        let body = resp.text().await?;
        Ok(body)
    }
}
