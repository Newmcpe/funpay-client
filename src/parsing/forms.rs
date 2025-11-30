use scraper::{Html, Selector};

pub fn extract_input_value(doc: &Html, name: &str) -> String {
    let selector = Selector::parse(&format!("input[name=\"{}\"]", name))
        .unwrap_or_else(|_| Selector::parse("input").unwrap());
    doc.select(&selector)
        .next()
        .and_then(|el| el.value().attr("value"))
        .unwrap_or("")
        .to_string()
}

pub fn extract_textarea_value(doc: &Html, name: &str) -> String {
    let selector = Selector::parse(&format!("textarea[name=\"{}\"]", name))
        .unwrap_or_else(|_| Selector::parse("textarea").unwrap());
    doc.select(&selector)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_default()
}

pub fn extract_checkbox_value(doc: &Html, name: &str) -> bool {
    let selector = Selector::parse(&format!("input[name=\"{}\"][type=\"checkbox\"]", name))
        .unwrap_or_else(|_| Selector::parse("input").unwrap());
    doc.select(&selector)
        .next()
        .map(|el| el.value().attr("checked").is_some())
        .unwrap_or(false)
}

pub fn extract_select_value(doc: &Html, name: &str) -> String {
    let selector = Selector::parse(&format!("select[name=\"{}\"] option[selected]", name))
        .unwrap_or_else(|_| Selector::parse("select").unwrap());
    doc.select(&selector)
        .next()
        .and_then(|el| el.value().attr("value"))
        .unwrap_or("")
        .to_string()
}

pub fn extract_field_value(doc: &Html, name: &str) -> String {
    let input_val = extract_input_value(doc, name);
    if !input_val.is_empty() {
        return input_val;
    }
    extract_select_value(doc, name)
}
