use crate::models::{
    CategoryFilter, CategoryFilterOption, CategoryFilterType, CategorySubcategory,
    CategorySubcategoryType,
};
use regex::Regex;
use scraper::{Html, Selector};

pub fn parse_category_subcategories(html: &str) -> Vec<CategorySubcategory> {
    let doc = Html::parse_document(html);
    let sel_container = Selector::parse("div.counter-list.counter-list-pills").unwrap();
    let sel_item = Selector::parse("a.counter-item").unwrap();
    let sel_name = Selector::parse("div.counter-param").unwrap();
    let sel_count = Selector::parse("div.counter-value").unwrap();

    let re_id = Regex::new(r"/(lots|chips)/(\d+)/?").unwrap();

    let mut subcategories = Vec::new();

    let Some(container) = doc.select(&sel_container).next() else {
        return subcategories;
    };

    for item in container.select(&sel_item) {
        let href = item.value().attr("href").unwrap_or("");

        let Some(captures) = re_id.captures(href) else {
            continue;
        };

        let type_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        let subcategory_type = match type_str {
            "lots" => CategorySubcategoryType::Lots,
            "chips" => CategorySubcategoryType::Chips,
            _ => continue,
        };

        let Some(id) = captures.get(2).and_then(|m| m.as_str().parse::<i64>().ok()) else {
            continue;
        };

        let name = item
            .select(&sel_name)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let offer_count = item
            .select(&sel_count)
            .next()
            .and_then(|el| {
                el.text()
                    .collect::<String>()
                    .trim()
                    .replace(' ', "")
                    .parse::<u32>()
                    .ok()
            })
            .unwrap_or(0);

        let is_active = item.value().classes().any(|c| c == "active");

        subcategories.push(CategorySubcategory {
            id,
            name,
            offer_count,
            subcategory_type,
            is_active,
        });
    }

    subcategories
}

pub fn parse_category_filters(html: &str) -> Vec<CategoryFilter> {
    let doc = Html::parse_document(html);
    let sel_filters = Selector::parse("div.showcase-filters").unwrap();
    let sel_lot_field = Selector::parse("div.lot-field").unwrap();
    let sel_select = Selector::parse("select.lot-field-input").unwrap();
    let sel_option = Selector::parse("option").unwrap();
    let sel_radio_box = Selector::parse("div.lot-field-radio-box").unwrap();
    let sel_button = Selector::parse("button").unwrap();
    let sel_range_box = Selector::parse("div.lot-field-range-box").unwrap();
    let sel_label = Selector::parse("label.control-label").unwrap();
    let sel_checkbox = Selector::parse("input[type=\"checkbox\"].showcase-filter-input").unwrap();
    let sel_checkbox_label = Selector::parse("label.showcase-filter-label").unwrap();

    let mut filters = Vec::new();

    let Some(container) = doc.select(&sel_filters).next() else {
        return filters;
    };

    for field in container.select(&sel_lot_field) {
        let Some(field_id) = field.value().attr("data-id") else {
            continue;
        };

        if let Some(select) = field.select(&sel_select).next() {
            let name = select
                .value()
                .attr("name")
                .map(|n| n.strip_prefix("f-").unwrap_or(n).to_string())
                .unwrap_or_else(|| field_id.to_string());

            let options: Vec<CategoryFilterOption> = select
                .select(&sel_option)
                .filter_map(|opt| {
                    let value = opt.value().attr("value")?.to_string();
                    if value.is_empty() {
                        return None;
                    }
                    let label = opt.text().collect::<String>().trim().to_string();
                    Some(CategoryFilterOption { value, label })
                })
                .collect();

            if !options.is_empty() {
                filters.push(CategoryFilter {
                    id: field_id.to_string(),
                    name,
                    filter_type: CategoryFilterType::Select,
                    options,
                });
            }
        } else if let Some(radio_box) = field.select(&sel_radio_box).next() {
            let name = field_id.to_string();

            let options: Vec<CategoryFilterOption> = radio_box
                .select(&sel_button)
                .filter_map(|btn| {
                    let value = btn.value().attr("value")?.to_string();
                    if value.is_empty() {
                        return None;
                    }
                    let label = btn.text().collect::<String>().trim().to_string();
                    Some(CategoryFilterOption { value, label })
                })
                .collect();

            if !options.is_empty() {
                filters.push(CategoryFilter {
                    id: field_id.to_string(),
                    name,
                    filter_type: CategoryFilterType::RadioBox,
                    options,
                });
            }
        } else if field.select(&sel_range_box).next().is_some() {
            let name = field
                .select(&sel_label)
                .next()
                .map(|l| l.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| field_id.to_string());

            filters.push(CategoryFilter {
                id: field_id.to_string(),
                name,
                filter_type: CategoryFilterType::Range,
                options: vec![],
            });
        }
    }

    for label in container.select(&sel_checkbox_label) {
        if let Some(checkbox) = label.select(&sel_checkbox).next() {
            let name = checkbox
                .value()
                .attr("name")
                .unwrap_or("unknown")
                .to_string();
            let label_text = label.text().collect::<String>().trim().to_string();

            filters.push(CategoryFilter {
                id: name.clone(),
                name: label_text,
                filter_type: CategoryFilterType::Checkbox,
                options: vec![],
            });
        }
    }

    filters
}
