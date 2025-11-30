use crate::error::FunPayError;
use crate::models::enums::OrderStatus;
use crate::models::ids::{ChatId, OrderId};
use crate::models::{Order, OrderShortcut, Review, Subcategory};
use crate::parsing::locales;
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;

pub fn parse_orders_list(html: &str, my_id: i64) -> Result<Vec<OrderShortcut>, FunPayError> {
    let doc = Html::parse_document(html);

    let sel_user = Selector::parse("div.user-link-name").unwrap();
    if doc.select(&sel_user).next().is_none() {
        return Err(FunPayError::Unauthorized);
    }

    let sel_item = Selector::parse("a.tc-item").unwrap();
    let sel_order = Selector::parse("div.tc-order").unwrap();
    let sel_desc = Selector::parse("div.order-desc").unwrap();
    let sel_price = Selector::parse("div.tc-price").unwrap();
    let sel_buyer = Selector::parse("div.media-user-name span").unwrap();
    let sel_subcat = Selector::parse("div.text-muted").unwrap();
    let sel_subcat_link = Selector::parse("div.text-muted a").unwrap();
    let sel_date = Selector::parse("div.tc-date-time").unwrap();
    let sel_div = Selector::parse("div").unwrap();

    let re_subcat =
        Regex::new(r"/(?:chips|lots|market|goods|game|category|subcategory)/(\d+)/?").unwrap();
    let re_amount = Regex::new(r"(?i)(\d+)\s*(шт|pcs|pieces|ед)\.?").unwrap();

    let mut out = Vec::new();

    for a in doc.select(&sel_item) {
        let class_list: Vec<String> = a.value().classes().map(|s| s.to_string()).collect();
        let status = if class_list.iter().any(|c| c == "warning") {
            OrderStatus::Refunded
        } else if class_list.iter().any(|c| c == "info") {
            OrderStatus::Paid
        } else {
            OrderStatus::Closed
        };

        let Some(order_div) = a.select(&sel_order).next() else {
            continue;
        };

        let mut id_text = order_div.text().collect::<String>();
        id_text = id_text.trim().to_string();
        let id = id_text.strip_prefix('#').unwrap_or(&id_text).to_string();

        let description = a
            .select(&sel_desc)
            .next()
            .and_then(|d| {
                d.select(&sel_div)
                    .next()
                    .map(|n| n.text().collect::<String>())
            })
            .unwrap_or_default()
            .trim()
            .to_string();

        let price_text_raw = a
            .select(&sel_price)
            .next()
            .map(|n| n.text().collect::<String>())
            .unwrap_or_default();
        let price_text = price_text_raw.replace('\u{00A0}', " ").trim().to_string();

        let (price_val, currency) = if let Some((p, cur)) = price_text.rsplit_once(' ') {
            let pv = p.replace(' ', "");
            let parsed = pv.parse::<f64>().unwrap_or(0.0);
            (parsed, cur.to_string())
        } else {
            (0.0, String::new())
        };

        let buyer_span = a.select(&sel_buyer).next();
        let buyer_username = buyer_span
            .as_ref()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let buyer_id = buyer_span
            .and_then(|n| n.value().attr("data-href"))
            .and_then(|v| v.split("/users/").nth(1))
            .and_then(|tail| tail.trim_end_matches('/').parse::<i64>().ok())
            .unwrap_or(0);

        let (id1, id2) = (my_id.min(buyer_id), my_id.max(buyer_id));
        let chat_id = ChatId::from(format!("users-{id1}-{id2}"));

        let subcategory_name = a
            .select(&sel_subcat)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let subcategory_id = a
            .select(&sel_subcat_link)
            .next()
            .and_then(|lnk| lnk.value().attr("href"))
            .and_then(|href| {
                re_subcat
                    .captures(href)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<i64>().ok())
            });

        let subcategory = Subcategory {
            id: subcategory_id,
            name: subcategory_name,
        };

        let date_text = a
            .select(&sel_date)
            .next()
            .map(|n| n.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let amount = re_amount
            .captures(&description)
            .and_then(|caps| {
                caps.get(1)
                    .and_then(|m| m.as_str().replace(' ', "").parse::<i32>().ok())
            })
            .unwrap_or(1);

        out.push(OrderShortcut {
            id: OrderId::from(id),
            description,
            price: price_val,
            currency,
            buyer_username,
            buyer_id,
            chat_id,
            status,
            date_text,
            subcategory,
            amount,
        });
    }

    Ok(out)
}

pub fn parse_order_secrets(doc: &Html) -> Vec<String> {
    let sel_param = Selector::parse("div.param-item").unwrap();
    let sel_h5 = Selector::parse("h5").unwrap();
    let sel_secret = Selector::parse("span.secret-placeholder").unwrap();

    let mut order_secrets = Vec::new();
    for p in doc.select(&sel_param) {
        let Some(header) = p.select(&sel_h5).next() else {
            continue;
        };
        let h_text = header.text().collect::<String>();
        let h = h_text.trim();
        if locales::matches_any(h, locales::PAID_PRODUCT) {
            for s in p.select(&sel_secret) {
                let t = s.text().collect::<String>().trim().to_string();
                if !t.is_empty() {
                    order_secrets.push(t);
                }
            }
        }
    }
    order_secrets
}

pub fn parse_order_page(html: &str, order_id: &str) -> Result<Order, FunPayError> {
    let doc = Html::parse_document(html);
    let sel_user = Selector::parse("div.user-link-name").unwrap();
    if doc.select(&sel_user).next().is_none() {
        return Err(FunPayError::Unauthorized);
    }

    let re_category = Regex::new(r"/(?:chips|lots)/(\d+)/?").unwrap();
    let re_users = Regex::new(r"/users/(\d+)/").unwrap();
    let re_chat = Regex::new(r"/chat/(\d+)/").unwrap();

    let status = {
        let sel_warn = Selector::parse("span.text-warning").unwrap();
        let sel_succ = Selector::parse("span.text-success").unwrap();
        let refunded = doc
            .select(&sel_warn)
            .next()
            .map(|n| n.text().collect::<String>())
            .map(|t| locales::matches_any(t.trim(), locales::REFUND))
            .unwrap_or(false);
        if refunded {
            OrderStatus::Refunded
        } else {
            let closed = doc
                .select(&sel_succ)
                .next()
                .map(|n| n.text().collect::<String>())
                .map(|t| locales::matches_any(t.trim(), locales::CLOSED))
                .unwrap_or(false);
            if closed {
                OrderStatus::Closed
            } else {
                OrderStatus::Paid
            }
        }
    };

    let sel_param = Selector::parse("div.param-item").unwrap();
    let sel_h5 = Selector::parse("h5").unwrap();
    let sel_div = Selector::parse("div").unwrap();

    let mut short_description: Option<String> = None;
    let mut full_description: Option<String> = None;
    let mut lot_params: Vec<(String, String)> = Vec::new();
    let buyer_params: HashMap<String, String> = HashMap::new();
    let mut amount: Option<i32> = None;
    let mut subcategory: Option<Subcategory> = None;

    for p in doc.select(&sel_param) {
        let Some(header) = p.select(&sel_h5).next() else {
            continue;
        };
        let h_text = header.text().collect::<String>();
        let h = h_text.trim();
        if locales::matches_any(h, locales::SHORT_DESCRIPTION) {
            if let Some(content) = p.select(&sel_div).next() {
                short_description = Some(content.text().collect::<String>().trim().to_string());
            }
        } else if locales::matches_any(h, locales::FULL_DESCRIPTION) {
            if let Some(content) = p.select(&sel_div).next() {
                full_description = Some(content.text().collect::<String>().trim().to_string());
            }
        } else if locales::matches_any(h, locales::CATEGORY) {
            let sel_a = Selector::parse("a").unwrap();
            if let Some(a) = p.select(&sel_a).next() {
                let href = a.value().attr("href").unwrap_or("");
                let name = a.text().collect::<String>().trim().to_string();
                let id = re_category
                    .captures(href)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<i64>().ok());
                if let Some(sid) = id {
                    subcategory = Some(Subcategory {
                        id: Some(sid),
                        name,
                    });
                }
            }
        } else if locales::matches_any(h, locales::AMOUNT) {
            let content = p.select(&sel_div).next();
            if let Some(c) = content {
                let a_txt = c.text().collect::<String>().trim().to_string();
                if let Ok(a) = a_txt.parse::<i32>() {
                    amount = Some(a);
                }
            }
        } else if !locales::matches_any(h, locales::PAID_PRODUCT) {
            let content_div = p.select(&sel_div).next();
            if let Some(content) = content_div {
                let content_text = content.text().collect::<String>().trim().to_string();
                if !content_text.is_empty() {
                    lot_params.push((h.to_string(), content_text));
                }
            }
        }
    }

    let order_secrets = parse_order_secrets(&doc);

    let sel_order_buyer = Selector::parse(".order-buyer").unwrap();
    let sel_order_sum = Selector::parse(".order-sum").unwrap();

    let buyer_info = doc.select(&sel_order_buyer).next();
    let sum_info = doc.select(&sel_order_sum).next();

    let (buyer_id, buyer_username) = buyer_info
        .and_then(|buyer| buyer.select(&Selector::parse("a").unwrap()).next())
        .map(|link| {
            let username = link.text().collect::<String>().trim().to_string();
            let id = link
                .value()
                .attr("href")
                .and_then(|href| re_users.captures(href))
                .and_then(|captures| captures.get(1))
                .and_then(|id_str| id_str.as_str().parse::<i64>().ok());
            (id, Some(username))
        })
        .unwrap_or((None, None));

    let (sum_val, currency) = if let Some(sum) = sum_info {
        let sum_text = sum.text().collect::<String>();
        let re = Regex::new(r"([\d.,]+)\s*([A-Za-zА-Яа-я₽$€£¥₴]+)").unwrap();
        if let Some(captures) = re.captures(&sum_text) {
            let amount_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let curr_str = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let amount_parsed = amount_str.replace(',', ".").parse::<f64>().ok();
            (
                amount_parsed,
                if curr_str.is_empty() {
                    None
                } else {
                    Some(curr_str.to_string())
                },
            )
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let chat_id = {
        let sel_chat = Selector::parse("a[href*='/chat/']").unwrap();
        doc.select(&sel_chat).next().and_then(|a| {
            a.value().attr("href").and_then(|href| {
                re_chat.captures(href).and_then(|captures| {
                    captures
                        .get(1)
                        .map(|id| ChatId::from(id.as_str().to_string()))
                })
            })
        })
    };

    let review = {
        let sel_review = Selector::parse(".review-item").unwrap();
        doc.select(&sel_review).next().map(|r| {
            let rating_sel = Selector::parse(".rating-mini .fas.fa-star").unwrap();
            let stars = Some(r.select(&rating_sel).count() as i32);
            let text_sel = Selector::parse(".review-text").unwrap();
            let text = r
                .select(&text_sel)
                .next()
                .map(|t| t.text().collect::<String>().trim().to_string());
            Review {
                stars,
                text,
                reply: None,
                anonymous: false,
                html: r.text().collect::<String>(),
                hidden: false,
                order_id: Some(OrderId::from(order_id.to_string())),
                author: None,
                author_id: None,
                by_bot: false,
                reply_by_bot: false,
            }
        })
    };

    Ok(Order {
        id: OrderId::from(order_id.to_string()),
        status,
        lot_params,
        buyer_params,
        short_description,
        full_description,
        subcategory,
        amount: amount.unwrap_or(0),
        sum: sum_val.unwrap_or(0.0),
        currency: currency.unwrap_or_else(|| String::from("RUB")),
        buyer_id: buyer_id.unwrap_or(0),
        buyer_username: buyer_username.unwrap_or_default(),
        seller_id: 0,
        seller_username: String::new(),
        chat_id: chat_id.unwrap_or_else(|| ChatId::from(String::from("0"))),
        html: html.to_string(),
        review,
        order_secrets,
    })
}
