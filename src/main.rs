#[macro_use]
extern crate dotenv_codegen;

use bytes::Bytes;
use futures_util::stream::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
use tokio::sync::RwLock;

mod models;
use models::nats;

mod utils;

type Mutex = Arc<RwLock<Vec<Bytes>>>;

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    let db_url = dotenv!("DATABASE_URL");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .unwrap();

    let mutex: Mutex = Arc::new(RwLock::new(Vec::new()));

    let nats_url = dotenv!("NATS_URL");
    let nats_subject = dotenv!("NATS_SUBJECT");
    let nats_user = dotenv!("NATS_USER").to_string();
    let nats_password = dotenv!("NATS_PASSWORD").to_string();

    let client = async_nats::ConnectOptions::new()
        .user_and_password(nats_user, nats_password)
        .connect(nats_url)
        .await
        .unwrap();

    let message_handler = tokio::spawn(handle_messages(mutex.clone(), pool.clone()));

    print!(
        "{} Connected to NATS server at {}...\n",
        chrono::Local::now(),
        nats_url
    );

    let mut subscriber = client.subscribe(nats_subject).await.unwrap();

    print!(
        "{} Subscribed to subject {}...\n",
        chrono::Local::now(),
        nats_subject
    );

    while let Some(msg) = subscriber.next().await {
        mutex.write().await.push(msg.payload);
    }

    _ = tokio::join!(message_handler);

    pool.close().await;

    Ok(())
}

async fn handle_messages(mutex: Mutex, pool: Pool<Postgres>) -> Result<(), async_nats::Error> {
    print!(
        "{} handle_messages: Starting message handler thread...\n",
        chrono::Local::now()
    );

    loop {
        let queue_size = mutex.read().await.len();

        if queue_size < 1000 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            continue;
        }

        let start = chrono::Utc::now();

        let mut queue_lock = mutex.write().await;
        let queue_slice: Vec<Bytes> = queue_lock.drain(0..queue_size).collect();
        drop(queue_lock);

        let mut messages: Vec<nats::MarketOrder> = Vec::new();

        for message in queue_slice {
            let parse_result = serde_json::from_slice::<nats::MarketOrder>(&message);

            let message = match parse_result {
                Ok(message) => message,
                Err(err) => {
                    print!(
                        "{} handle_messages: Failed to parse message: {}\n",
                        chrono::Local::now(),
                        err
                    );
                    continue;
                }
            };

            messages.push(message)
        }

        let result = utils::db::insert_market_orders(&pool, messages).await;

        let rows_affected = match result {
            Ok(rows_affected) => rows_affected.rows_affected(),
            Err(err) => {
                print!(
                    "{} handle_messages: Failed to insert market orders: {}\n",
                    chrono::Local::now(),
                    err
                );
                continue;
            }
        };

        let end = chrono::Utc::now();

        print!(
            "{} handle_messages: Inserted {} market orders in {} ms...\n",
            chrono::Local::now(),
            rows_affected,
            end.signed_duration_since(start).num_milliseconds()
        );
    }
}
