#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    Paid,
    Closed,
    Refunded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubcategoryType {
    Common,
    Currency,
}
