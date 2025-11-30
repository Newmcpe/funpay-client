use scraper::{Html, Selector};

pub fn parse_message_html(html: &str) -> (Option<String>, Option<String>) {
    let html_owned = html.replace("<br>", "\n");
    let doc = Html::parse_fragment(&html_owned);

    let sel_text = Selector::parse("div.chat-msg-text").unwrap();
    if let Some(n) = doc.select(&sel_text).next() {
        let t = n.text().collect::<String>();
        return (Some(t), None);
    }

    let sel_alert = Selector::parse("div[role=alert]").unwrap();
    if let Some(n) = doc.select(&sel_alert).next() {
        let t = n.text().collect::<String>();
        return (Some(t), None);
    }

    let sel_img = Selector::parse("a.chat-img-link").unwrap();
    if let Some(n) = doc.select(&sel_img).next() {
        let href = n.value().attr("href").map(|s| s.to_string());
        return (None, href);
    }

    (None, None)
}
