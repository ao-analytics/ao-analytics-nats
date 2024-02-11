#[macro_use]
extern crate dotenv_codegen;

use tokio::sync::RwLock;
use std::sync::Arc;

use aodata_models::nats;

use futures_util::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono;
use sqlx::{Pool, Postgres};

use tracing::{info, warn};
use tracing_subscriber;

mod utils;

#[tokio::main]
async fn main() -> Result<(), async_nats::SubscribeError> {
    tracing_subscriber::fmt::init();

    let db_url = dotenv!("DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .unwrap();

    let nats_url = dotenv!("NATS_URL");
    let nats_market_orders_subject = dotenv!("NATS_MARKET_ORDERS_SUBJECT");
    let nats_market_histories_subject = dotenv!("NATS_MARKET_HISTORIES_SUBJECT");
    let nats_user = dotenv!("NATS_USER").to_string();
    let nats_password = dotenv!("NATS_PASSWORD").to_string();

    let client = async_nats::ConnectOptions::new()
        .user_and_password(nats_user, nats_password)
        .connect(nats_url)
        .await
        .unwrap();

    info!("Connected to NATS server at {}", nats_url);

    let market_orders_subcriber = client.subscribe(nats_market_orders_subject).await?;

    info!("Subscribed to subject {}", nats_market_orders_subject);

    let market_histories_subcriber = client.subscribe(nats_market_histories_subject).await?;

    info!("Subscribed to subject {}", nats_market_histories_subject);

    let market_orders: Arc<RwLock<Vec<nats::MarketOrder>>> = Arc::new(RwLock::new(Vec::new()));
    let market_histories: Arc<RwLock<Vec<nats::MarketHistories>>> = Arc::new(RwLock::new(Vec::new()));

    let market_order_message_handler = tokio::spawn(handle_market_orders_messages(
        market_orders.clone(),
        pool.clone(),
    ));

    info!("Started market order message handler thread");

    let market_history_message_handler = tokio::spawn(handle_market_histories_messages(
        market_histories.clone(),
        pool.clone(),
    ));

    info!("Started market history message handler thread");

    let mut messages =
        futures_util::stream::select_all([market_orders_subcriber, market_histories_subcriber]);

    while let Some(msg) = messages.next().await {

        if msg.subject.as_str() == nats_market_orders_subject {
            let parse_result = serde_json::from_slice::<nats::MarketOrder>(&msg.payload);
            
            let market_order = match parse_result {
                Ok(market_order) => market_order,
                Err(err) => {
                    warn!("Failed to parse market order message: {}", err);
                    continue;
                }
            };

            market_orders.write().await.push(market_order);
            continue;
        }

        if msg.subject.as_str() == nats_market_histories_subject {
            let parse_result = serde_json::from_slice::<nats::MarketHistories>(&msg.payload);

            let market_history = match parse_result {
                Ok(market_history) => market_history,
                Err(err) => {
                    warn!("Failed to parse market history message: {}", err);
                    continue;
                }
            };

            market_histories.write().await.push(market_history);
            continue;
        }

        warn!("Unknown subject: {}", msg.subject);
    }

    _ = tokio::join!(market_order_message_handler, market_history_message_handler);

    pool.close().await;

    Ok(())
}

async fn handle_market_orders_messages(
    market_orders: Arc<RwLock<Vec<nats::MarketOrder>>>,
    pool: Pool<Postgres>,
) -> Result<(), async_nats::Error> {
    loop {
        let queue_size = market_orders.read().await.len();

        if queue_size < 1000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        let start = chrono::Utc::now();

        let mut queue_lock = market_orders.write().await;
        let messages: Vec<nats::MarketOrder> = queue_lock.drain(0..queue_size).collect();
        drop(queue_lock);

        let result = utils::db::insert_market_orders(&pool, messages).await;

        let rows_affected = match result {
            Ok(rows_affected) => rows_affected.rows_affected(),
            Err(err) => {
                warn!("Failed to insert market orders: {}", err);
                continue;
            }
        };

        let end = chrono::Utc::now();

        info!(
            "Inserted {} market orders in {} ms",
            rows_affected,
            end.signed_duration_since(start).num_milliseconds()
        );
    }
}

async fn handle_market_histories_messages(
    market_histories: Arc<RwLock<Vec<nats::MarketHistories>>>,
    pool: Pool<Postgres>,
) -> Result<(), async_nats::Error> {
    loop {
        let queue_size = market_histories.read().await.len();

        if queue_size < 1000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        let start = chrono::Utc::now();

        let mut queue_lock = market_histories.write().await;
        let messages: Vec<nats::MarketHistories>= queue_lock.drain(0..queue_size).collect();
        drop(queue_lock);

        let result = utils::db::insert_market_histories(&pool, messages).await;

        let rows_affected = match result {
            Ok(rows_affected) => rows_affected.rows_affected(),
            Err(err) => {
                warn!("Failed to insert market histories: {}", err);
                continue;
            }
        };

        let end = chrono::Utc::now();

        info!(
            "Inserted {} market histories in {} ms",
            rows_affected,
            end.signed_duration_since(start).num_milliseconds()
        );
    }
}
