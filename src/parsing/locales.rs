pub const PAID_PRODUCT: &[&str] = &[
    "Оплаченный товар",
    "Оплаченные товары",
    "Оплачений товар",
    "Оплачені товари",
    "Paid product",
    "Paid products",
];

pub const SHORT_DESCRIPTION: &[&str] = &[
    "Краткое описание",
    "Короткий опис",
    "Short description",
];

pub const FULL_DESCRIPTION: &[&str] = &[
    "Полное описание",
    "Повний опис",
    "Full description",
];

pub const CATEGORY: &[&str] = &[
    "Категория",
    "Категорія",
    "Category",
    "Валюта",
    "Currency",
];

pub const AMOUNT: &[&str] = &[
    "Кол-во",
    "Кількість",
    "Amount",
];

pub const REFUND: &[&str] = &[
    "Возврат",
    "Повернення",
    "Refund",
];

pub const CLOSED: &[&str] = &[
    "Закрыт",
    "Закрито",
    "Closed",
];

pub fn matches_any(text: &str, variants: &[&str]) -> bool {
    variants.iter().any(|v| *v == text)
}
