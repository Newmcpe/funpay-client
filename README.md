# funpay-client

[![Crates.io](https://img.shields.io/crates/v/funpay-client.svg)](https://crates.io/crates/funpay-client)
[![Documentation](https://docs.rs/funpay-client/badge.svg)](https://docs.rs/funpay-client)
[![License: WTFPL](https://img.shields.io/badge/License-WTFPL-brightgreen.svg)](http://www.wtfpl.net/about/)

Unofficial async Rust client for FunPay marketplace. Authenticate via `golden_key`, receive real-time events for chats and orders.

## Features

- Real-time polling for chats and orders
- Send messages to chats
- Edit offers (price, quantity, status)
- Configurable polling intervals, retry policies, and User-Agent
- Pluggable state storage (JSON file or in-memory)
- Async/await with Tokio

## Installation

```toml
[dependencies]
funpay-client = "0.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Quick Start

```rust
use funpay_client::{FunPayAccount, Event};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let golden_key = std::env::var("FUNPAY_GOLDEN_KEY")?;

    let mut account = FunPayAccount::new(golden_key);
    account.init().await?;

    println!("Logged in as: {:?}", account.username);

    let mut rx = account.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                Event::NewMessage { message } => {
                    println!("New message in {}: {:?}", message.chat_id, message.text);
                }
                Event::NewOrder { order } => {
                    println!("New order: {} - {}", order.id, order.description);
                }
                Event::OrderStatusChanged { order } => {
                    println!("Order {} status: {:?}", order.id, order.status);
                }
                _ => {}
            }
        }
    });

    account.start_polling_loop().await?;
    Ok(())
}
```

## Configuration

Use `FunPayConfig` builder for custom settings:

```rust
use funpay_client::{FunPayAccount, FunPayConfig};
use std::time::Duration;

let config = FunPayConfig::builder()
    .user_agent("MyBot/1.0")
    .polling_interval(Duration::from_secs(2))
    .error_retry_delay(Duration::from_secs(10))
    .event_channel_capacity(1024)
    .state_storage_path("./state.json")
    .retry_policy(50, 5)  // base_ms, max_retries
    .build();

let account = FunPayAccount::with_config(golden_key, config);
```

### With Proxy

```rust
use funpay_client::{FunPayAccount, FunPayConfig};

// Simple proxy
let account = FunPayAccount::with_proxy(golden_key, "http://proxy:8080");

// Proxy with custom config
let config = FunPayConfig::builder()
    .polling_interval(Duration::from_secs(3))
    .build();
let account = FunPayAccount::with_proxy_and_config(golden_key, "http://user:pass@proxy:8080", config);
```

## Events

| Event | Description |
|-------|-------------|
| `InitialChat` | Chat loaded on startup |
| `ChatsListChanged` | Chat list updated |
| `LastChatMessageChanged` | New activity in chat |
| `NewMessage` | New message received |
| `InitialOrder` | Order loaded on startup |
| `OrdersListChanged` | Order counters changed |
| `NewOrder` | New order created |
| `OrderStatusChanged` | Order status changed |

## Sending Messages

```rust
let sender = account.create_sender()?;
sender.send_chat_message("users-123-456", "Hello!").await?;
```

## Working with Offers

```rust
use funpay_client::models::OfferEditParams;

let sender = account.create_sender()?;

// Get current offer params
let params = sender.get_offer_params(offer_id, node_id).await?;

// Update offer price
let update = OfferEditParams {
    price: Some("100.00".to_string()),
    ..Default::default()
};
sender.edit_offer(offer_id, node_id, update).await?;

// Get all my offers for a category
let offers = sender.get_my_offers(node_id).await?;
```

## Custom Gateway

Implement `FunpayGateway` trait for custom HTTP handling:

```rust
use funpay_client::{FunPayAccount, FunpayGateway};
use std::sync::Arc;

struct MyGateway { /* ... */ }

#[async_trait::async_trait]
impl FunpayGateway for MyGateway {
    // implement required methods...
}

let gateway: Arc<dyn FunpayGateway> = Arc::new(MyGateway::new());
let account = FunPayAccount::with_gateway(gateway, golden_key);
```

## Custom State Storage

Implement `StateStorage` trait for custom persistence:

```rust
use funpay_client::StateStorage;
use async_trait::async_trait;
use std::collections::HashMap;

struct RedisStorage { /* ... */ }

#[async_trait]
impl StateStorage for RedisStorage {
    async fn load(&self) -> anyhow::Result<HashMap<i64, i64>> {
        // load from Redis
    }

    async fn save(&self, data: &HashMap<i64, i64>) -> anyhow::Result<()> {
        // save to Redis
    }
}
```

## Configuration Defaults

| Parameter | Default |
|-----------|---------|
| `base_url` | `https://funpay.com` |
| `user_agent` | Chrome 123 on Windows |
| `polling_interval` | 1500ms |
| `error_retry_delay` | 5s |
| `event_channel_capacity` | 512 |
| `retry_base_ms` | 20 |
| `max_retries` | 3 |
| `redirect_limit` | 10 |

## Requirements

- Rust 1.84+
- Tokio 1.x