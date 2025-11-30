use crate::models::{
    MarketOffer, Offer, OfferCustomField, OfferEditParams, OfferFieldOption, OfferFieldType,
    OfferFullParams,
};
use crate::parsing::{
    extract_checkbox_value, extract_field_value, extract_input_value, extract_textarea_value,
};
use regex::Regex;
use scraper::{Html, Selector};

pub fn parse_my_offers(html: &str, node_id: i64) -> Vec<Offer> {
    let doc = Html::parse_document(html);
    let sel_item = Selector::parse("a.tc-item[data-offer]").unwrap();
    let sel_desc = Selector::parse("div.tc-desc-text").unwrap();
    let sel_price = Selector::parse("div.tc-price").unwrap();
    let sel_unit = Selector::parse("span.unit").unwrap();

    let mut offers = Vec::new();

    for item in doc.select(&sel_item) {
        let offer_id = item
            .value()
            .attr("data-offer")
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        if offer_id == 0 {
            continue;
        }

        let description = item
            .select(&sel_desc)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let price_el = item.select(&sel_price).next();
        let price = price_el
            .and_then(|el| el.value().attr("data-s"))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let currency = price_el
            .and_then(|el| el.select(&sel_unit).next())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "₽".to_string());

        let active = !item.value().classes().any(|c| c == "warning");

        offers.push(Offer {
            id: offer_id,
            node_id,
            description,
            price,
            currency,
            active,
        });
    }

    offers
}

pub fn parse_market_offers(html: &str, node_id: i64) -> Vec<MarketOffer> {
    let doc = Html::parse_document(html);
    let sel_item = Selector::parse("a.tc-item").unwrap();
    let sel_desc = Selector::parse("div.tc-desc-text").unwrap();
    let sel_price = Selector::parse("div.tc-price").unwrap();
    let sel_unit = Selector::parse("span.unit").unwrap();
    let sel_seller = Selector::parse("span.pseudo-a[data-href]").unwrap();
    let sel_reviews = Selector::parse("div.media-user-reviews").unwrap();
    let sel_rating_count = Selector::parse("span.rating-mini-count").unwrap();
    let sel_rating_stars = Selector::parse("div.rating-stars").unwrap();

    let re_offer_id = Regex::new(r"[?&]id=(\d+)").unwrap();
    let re_user_id = Regex::new(r"/users/(\d+)/?").unwrap();
    let re_reviews_text = Regex::new(r"(\d+)").unwrap();
    let re_rating = Regex::new(r"rating-(\d+(?:\.\d+)?)").unwrap();

    let mut offers = Vec::new();

    for item in doc.select(&sel_item) {
        let href = item.value().attr("href").unwrap_or("");
        let offer_id = re_offer_id
            .captures(href)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);

        if offer_id == 0 {
            continue;
        }

        let description = item
            .select(&sel_desc)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let price_el = item.select(&sel_price).next();
        let price = price_el
            .and_then(|el| el.value().attr("data-s"))
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let currency = price_el
            .and_then(|el| el.select(&sel_unit).next())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "₽".to_string());

        let seller_el = item.select(&sel_seller).next();
        let seller_name = seller_el
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let seller_id = seller_el
            .and_then(|el| el.value().attr("data-href"))
            .and_then(|href| {
                re_user_id
                    .captures(href)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<i64>().ok())
            })
            .unwrap_or(0);

        let seller_online = item.value().attr("data-online") == Some("1");
        let is_promo = item.value().classes().any(|c| c == "offer-promo");

        let seller_reviews = item
            .select(&sel_reviews)
            .next()
            .and_then(|reviews_el| {
                if let Some(count_el) = reviews_el.select(&sel_rating_count).next() {
                    count_el
                        .text()
                        .collect::<String>()
                        .trim()
                        .parse::<u32>()
                        .ok()
                } else {
                    let text = reviews_el.text().collect::<String>();
                    re_reviews_text
                        .captures(&text)
                        .and_then(|c| c.get(1))
                        .and_then(|m| m.as_str().parse::<u32>().ok())
                }
            })
            .unwrap_or(0);

        let seller_rating = item.select(&sel_reviews).next().and_then(|reviews_el| {
            reviews_el
                .select(&sel_rating_stars)
                .next()
                .and_then(|rating_el| {
                    rating_el.value().classes().find_map(|class| {
                        re_rating
                            .captures(class)
                            .and_then(|c| c.get(1))
                            .and_then(|m| m.as_str().parse::<f64>().ok())
                    })
                })
        });

        offers.push(MarketOffer {
            id: offer_id,
            node_id,
            description,
            price,
            currency,
            seller_id,
            seller_name,
            seller_online,
            seller_rating,
            seller_reviews,
            is_promo,
        });
    }

    offers
}

pub fn parse_offer_edit_params(html: &str) -> OfferEditParams {
    let doc = Html::parse_document(html);

    OfferEditParams {
        quantity: Some(extract_field_value(&doc, "fields[quantity]")),
        quantity2: Some(extract_field_value(&doc, "fields[quantity2]")),
        method: Some(extract_field_value(&doc, "fields[method]")),
        offer_type: Some(extract_field_value(&doc, "fields[type]")),
        server_id: Some(extract_field_value(&doc, "server_id")),
        desc_ru: Some(extract_textarea_value(&doc, "fields[desc][ru]")),
        desc_en: Some(extract_textarea_value(&doc, "fields[desc][en]")),
        payment_msg_ru: Some(extract_textarea_value(&doc, "fields[payment_msg][ru]")),
        payment_msg_en: Some(extract_textarea_value(&doc, "fields[payment_msg][en]")),
        summary_ru: Some(extract_input_value(&doc, "fields[summary][ru]")),
        summary_en: Some(extract_input_value(&doc, "fields[summary][en]")),
        game: Some(extract_field_value(&doc, "fields[game]")),
        images: Some(extract_input_value(&doc, "fields[images]")),
        price: Some(extract_input_value(&doc, "price")),
        deactivate_after_sale: Some(extract_checkbox_value(&doc, "deactivate_after_sale")),
        active: Some(extract_checkbox_value(&doc, "active")),
        location: Some(extract_input_value(&doc, "location")),
        deleted: None,
    }
}

pub fn parse_offer_full_params(html: &str, offer_id: i64, node_id: i64) -> OfferFullParams {
    let doc = Html::parse_document(html);
    let mut custom_fields = Vec::new();
    let sel_form_group = Selector::parse("div.form-group").unwrap();
    let sel_label = Selector::parse("label").unwrap();
    let sel_input = Selector::parse("input").unwrap();
    let sel_textarea = Selector::parse("textarea").unwrap();
    let sel_select = Selector::parse("select").unwrap();
    let sel_option = Selector::parse("option").unwrap();
    let re_field_name = Regex::new(r"fields\[([^\]]+)\]").unwrap();

    for group in doc.select(&sel_form_group) {
        let label_text = group
            .select(&sel_label)
            .next()
            .map(|l| l.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if let Some(input) = group.select(&sel_input).next() {
            let name = input.value().attr("name").unwrap_or("");
            if !name.starts_with("fields[")
                || name.contains("[desc]")
                || name.contains("[payment_msg]")
                || name.contains("[images]")
            {
                continue;
            }

            let input_type = input.value().attr("type").unwrap_or("text");
            let value = input.value().attr("value").unwrap_or("").to_string();

            let _field_name = re_field_name
                .captures(name)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| name.to_string());

            let field_type = match input_type {
                "checkbox" => OfferFieldType::Checkbox,
                "hidden" => OfferFieldType::Hidden,
                _ => OfferFieldType::Text,
            };

            let actual_value = if field_type == OfferFieldType::Checkbox {
                if input.value().attr("checked").is_some() {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            } else {
                value
            };

            custom_fields.push(OfferCustomField {
                name: name.to_string(),
                label: label_text.clone(),
                field_type,
                value: actual_value,
                options: vec![],
            });
        } else if let Some(textarea) = group.select(&sel_textarea).next() {
            let name = textarea.value().attr("name").unwrap_or("");
            if !name.starts_with("fields[")
                || name.contains("[desc]")
                || name.contains("[payment_msg]")
            {
                continue;
            }

            let value = textarea.text().collect::<String>();

            custom_fields.push(OfferCustomField {
                name: name.to_string(),
                label: label_text.clone(),
                field_type: OfferFieldType::Textarea,
                value,
                options: vec![],
            });
        } else if let Some(select) = group.select(&sel_select).next() {
            let name = select.value().attr("name").unwrap_or("");
            if !name.starts_with("fields[") {
                continue;
            }

            let mut options = Vec::new();
            let mut selected_value = String::new();

            for opt in select.select(&sel_option) {
                let opt_value = opt.value().attr("value").unwrap_or("").to_string();
                let opt_label = opt.text().collect::<String>().trim().to_string();
                let is_selected = opt.value().attr("selected").is_some();

                if is_selected {
                    selected_value = opt_value.clone();
                }

                options.push(OfferFieldOption {
                    value: opt_value,
                    label: opt_label,
                    selected: is_selected,
                });
            }

            custom_fields.push(OfferCustomField {
                name: name.to_string(),
                label: label_text.clone(),
                field_type: OfferFieldType::Select,
                value: selected_value,
                options,
            });
        }
    }

    OfferFullParams {
        offer_id,
        node_id,
        quantity: Some(extract_field_value(&doc, "fields[quantity]")).filter(|s| !s.is_empty()),
        quantity2: Some(extract_field_value(&doc, "fields[quantity2]")).filter(|s| !s.is_empty()),
        method: Some(extract_field_value(&doc, "fields[method]")).filter(|s| !s.is_empty()),
        offer_type: Some(extract_field_value(&doc, "fields[type]")).filter(|s| !s.is_empty()),
        server_id: Some(extract_field_value(&doc, "server_id")).filter(|s| !s.is_empty()),
        desc_ru: Some(extract_textarea_value(&doc, "fields[desc][ru]")).filter(|s| !s.is_empty()),
        desc_en: Some(extract_textarea_value(&doc, "fields[desc][en]")).filter(|s| !s.is_empty()),
        payment_msg_ru: Some(extract_textarea_value(&doc, "fields[payment_msg][ru]"))
            .filter(|s| !s.is_empty()),
        payment_msg_en: Some(extract_textarea_value(&doc, "fields[payment_msg][en]"))
            .filter(|s| !s.is_empty()),
        images: Some(extract_input_value(&doc, "fields[images]")).filter(|s| !s.is_empty()),
        price: Some(extract_input_value(&doc, "price")).filter(|s| !s.is_empty()),
        deactivate_after_sale: extract_checkbox_value(&doc, "deactivate_after_sale"),
        active: extract_checkbox_value(&doc, "active"),
        location: Some(extract_input_value(&doc, "location")).filter(|s| !s.is_empty()),
        custom_fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_HTML: &str = r#"
<a href="https://funpay.com/lots/offer?id=58789647" class="tc-item offer-promo offer-promoted" data-online="1" data-user="4029757" data-f-quantity="13 звёзд" data-f-method="подарком">
<div class="tc-desc">
<div class="tc-desc-text">13 звёзд, Подарком</div>
</div>
<div class="tc-user">
<div class="media media-user online style-circle">
<div class="media-left">
<div class="avatar-photo pseudo-a" tabindex="0" data-href="https://funpay.com/users/4029757/" style="background-image: url(https://sfunpay.com/s/avatar/cp/wy/cpwyafyka0rqnbjemxc4.jpg);"></div>
</div>
<div class="media-body">
<div class="media-user-name">
<span class="pseudo-a" tabindex="0" data-href="https://funpay.com/users/4029757/">Ded777Veka</span>
</div>
<div class="media-user-reviews">
<div class="rating-stars rating-5"><i class="fas"></i><i class="fas"></i><i class="fas"></i><i class="fas"></i><i class="fas"></i></div><span class="rating-mini-count">220</span>
</div>
<div class="media-user-info">на сайте 4 года</div>
</div>
</div>
</div><div class="tc-price" data-s="20.89613">
<div>20.90 <span class="unit">₽</span></div>
<div class="sc-offer-icons"><div class="promo-offer-icon lb-promo-offer-hightlight" style="margin-left: 4px;"></div> <div class="promo-offer-icon"></div></div></div>
</a><a href="https://funpay.com/lots/offer?id=58821247" class="tc-item" data-online="1" data-f-quantity="13 звёзд" data-f-method="подарком">
<div class="tc-desc">
<div class="tc-desc-text">13 звёзд, Подарком</div>
</div>
<div class="tc-user">
<div class="media media-user online style-circle">
<div class="media-left">
<div class="avatar-photo pseudo-a" tabindex="0" data-href="https://funpay.com/users/17151546/" style="background-image: url(https://sfunpay.com/s/avatar/19/b9/19b9tuf6mqnwt0fn71xj.jpg);"></div>
</div>
<div class="media-body">
<div class="media-user-name">
<span class="pseudo-a" tabindex="0" data-href="https://funpay.com/users/17151546/">Ksannyaa</span>
</div>
<div class="media-user-reviews">35 отзывов</div>
<div class="media-user-info">на сайте месяц</div>
</div>
</div>
</div><div class="tc-price" data-s="19.038697">
<div>19.04 <span class="unit">₽</span></div>
</div>
</a>
<a href="https://funpay.com/lots/offer?id=51391953" class="tc-item offer-promo" data-online="1" data-user="16023197" data-f-quantity="13 звёзд" data-f-method="подарком">
<div class="tc-desc">
<div class="tc-desc-text">13 звёзд, Подарком</div>
</div>
<div class="tc-user">
<div class="media media-user online style-circle">
<div class="media-left">
<div class="avatar-photo pseudo-a" tabindex="0" data-href="https://funpay.com/users/16023197/" style="background-image: url(https://sfunpay.com/s/avatar/7y/we/7ywe4fo04xk4vx9wo89v.jpg);"></div>
</div>
<div class="media-body">
<div class="media-user-name">
<span class="pseudo-a" tabindex="0" data-href="https://funpay.com/users/16023197/">starsTGgreat</span>
</div>
<div class="media-user-reviews">
<div class="rating-stars rating-5"><i class="fas"></i><i class="fas"></i><i class="fas"></i><i class="fas"></i><i class="fas"></i></div><span class="rating-mini-count">5876</span>
</div>
<div class="media-user-info">на сайте 4 месяца</div>
</div>
</div>
</div><div class="tc-price" data-s="19.154786">
<div>19.15 <span class="unit">₽</span></div>
<div class="sc-offer-icons"><div class="promo-offer-icon lb-promo-offer-hightlight" style="margin-left: 4px;"></div></div></div>
</a>
"#;

    #[test]
    fn test_parse_market_offers() {
        let offers = parse_market_offers(TEST_HTML, 2418);

        assert_eq!(offers.len(), 3);

        let first = &offers[0];
        assert_eq!(first.id, 58789647);
        assert_eq!(first.node_id, 2418);
        assert_eq!(first.description, "13 звёзд, Подарком");
        assert!((first.price - 20.89613).abs() < 0.0001);
        assert_eq!(first.currency, "₽");
        assert_eq!(first.seller_id, 4029757);
        assert_eq!(first.seller_name, "Ded777Veka");
        assert!(first.seller_online);
        assert_eq!(first.seller_reviews, 220);
        assert!(first.is_promo);

        let second = &offers[1];
        assert_eq!(second.id, 58821247);
        assert_eq!(second.description, "13 звёзд, Подарком");
        assert!((second.price - 19.038697).abs() < 0.0001);
        assert_eq!(second.currency, "₽");
        assert_eq!(second.seller_id, 17151546);
        assert_eq!(second.seller_name, "Ksannyaa");
        assert!(second.seller_online);
        assert_eq!(second.seller_reviews, 35);
        assert!(!second.is_promo);

        let third = &offers[2];
        assert_eq!(third.id, 51391953);
        assert_eq!(third.description, "13 звёзд, Подарком");
        assert!((third.price - 19.154786).abs() < 0.0001);
        assert_eq!(third.currency, "₽");
        assert_eq!(third.seller_id, 16023197);
        assert_eq!(third.seller_name, "starsTGgreat");
        assert!(third.seller_online);
        assert_eq!(third.seller_reviews, 5876);
        assert!(third.is_promo);
    }

    #[test]
    fn test_parse_market_offers_empty_page() {
        let html = r#"<div class="tc"></div>"#;
        let offers = parse_market_offers(html, 2418);
        assert!(offers.is_empty());
    }

    #[test]
    fn test_parse_market_offers_skips_invalid() {
        let html = r#"
            <a href="https://funpay.com/lots/offer" class="tc-item">
                <div class="tc-desc-text">No ID offer</div>
            </a>
            <a href="https://funpay.com/lots/offer?id=123" class="tc-item" data-online="1">
                <div class="tc-desc-text">Valid offer</div>
                <div class="tc-price" data-s="100.0">
                    <span class="unit">₽</span>
                </div>
                <span class="pseudo-a" data-href="https://funpay.com/users/999/">Seller</span>
            </a>
        "#;

        let offers = parse_market_offers(html, 1234);

        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].id, 123);
        assert_eq!(offers[0].node_id, 1234);
    }
}
