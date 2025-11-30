use crate::models::{ChatShortcut, Message, OrderShortcut};

#[derive(Debug, Clone)]
pub enum Event {
    InitialChat { chat: ChatShortcut },
    ChatsListChanged,
    LastChatMessageChanged { chat: ChatShortcut },
    NewMessage { message: Message },
    InitialOrder { order: OrderShortcut },
    OrdersListChanged { purchases: i32, sales: i32 },
    NewOrder { order: OrderShortcut },
    OrderStatusChanged { order: OrderShortcut },
}
