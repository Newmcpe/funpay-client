use funpay_client::{Event, FunPayAccount};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let golden_key = std::env::var("FUNPAY_GOLDEN_KEY")
        .expect("FUNPAY_GOLDEN_KEY environment variable required");

    let mut account = FunPayAccount::new(golden_key);
    account.init().await?;

    println!(
        "Logged in as: {:?} (id: {:?})",
        account.username, account.id
    );

    let mut rx = account.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            match event {
                Event::InitialChat { chat } => {
                    println!("[INIT] Chat: {} (id: {})", chat.name, chat.id);
                }
                Event::NewMessage { message } => {
                    println!(
                        "[MSG] Chat {}: {:?}",
                        message.chat_id,
                        message.text.as_deref().unwrap_or("<no text>")
                    );
                }
                Event::NewOrder { order } => {
                    println!("[ORDER] New: {} - {}", order.id, order.description);
                }
                Event::OrderStatusChanged { order } => {
                    println!("[ORDER] Status changed: {} -> {:?}", order.id, order.status);
                }
                Event::OrdersListChanged { purchases, sales } => {
                    println!("[ORDERS] Purchases: {}, Sales: {}", purchases, sales);
                }
                _ => {}
            }
        }
    });

    println!("Starting polling loop...");
    account.start_polling_loop().await?;

    Ok(())
}
