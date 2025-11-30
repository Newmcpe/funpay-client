use funpay_client::{Event, FunPayAccount, FunPayConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let golden_key = std::env::var("FUNPAY_GOLDEN_KEY")
        .expect("FUNPAY_GOLDEN_KEY environment variable required");

    let config = FunPayConfig::builder()
        .user_agent("FunPayBot/1.0 (Rust)")
        .polling_interval(Duration::from_secs(2))
        .error_retry_delay(Duration::from_secs(10))
        .event_channel_capacity(256)
        .state_storage_path("./funpay_state.json")
        .retry_policy(50, 5)
        .build();

    let mut account = FunPayAccount::with_config(golden_key, config);
    account.init().await?;

    println!("Logged in as: {:?}", account.username);
    println!("Config: polling every 2s, state saved to ./funpay_state.json");

    let mut rx = account.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                Event::NewMessage { message } => {
                    println!("[MSG] {}: {:?}", message.chat_id, message.text);
                }
                Event::NewOrder { order } => {
                    println!("[ORDER] {}: {}", order.id, order.description);
                }
                _ => {}
            }
        }
    });

    account.start_polling_loop().await?;
    Ok(())
}
