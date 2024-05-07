use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::{select, signal};

use aodata_models::{db, nats};

use futures_util::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::{self};
use sqlx::{Pool, Postgres};

use tracing::{info, warn};

mod utils;

#[tokio::main]
async fn main() -> Result<(), async_nats::SubscribeError> {
    let config = utils::config::Config::from_env();

    let config = match config {
        Some(config) => config,
        None => {
            panic!("Failed to initialize config");
        }
    };

    tracing_subscriber::fmt::init();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.db_url)
        .await
        .unwrap();

    let client = async_nats::ConnectOptions::new()
        .user_and_password(config.nats_user, config.nats_password)
        .connect(&config.nats_url)
        .await
        .unwrap();

    info!("Connected to NATS server at {}", &config.nats_url);

    let mut messages = client.subscribe(config.nats_subject.to_string()).await?;

    info!("Subscribed to subject {}", &config.nats_subject);

    let market_orders: Arc<RwLock<Vec<db::MarketOrder>>> = Arc::new(RwLock::new(Vec::new()));
    let token = tokio_util::sync::CancellationToken::new();
    let mut int_signal = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
    let mut term_signal = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

    let join_handle = tokio::spawn(handle_market_orders_messages(
        market_orders.clone(),
        pool.clone(),
        token.clone(),
    ));

    info!("Started market order message handler thread");

    while let Some(msg) = select! {
        _ = int_signal.recv() => {
            info!("Received interrupt signal");
            token.cancel();
            None
        },
        _ = term_signal.recv() => {
            info!("Received terminate signal");
            token.cancel();
            None
        },
        msg = messages.next() => msg
    } {
        let parse_result = serde_json::from_slice::<nats::MarketOrder>(&msg.payload);

        let market_order = match parse_result {
            Ok(market_order) => market_order,
            Err(err) => {
                warn!(
                    "Failed to parse market order message: {:?} {}",
                    &msg.payload, err
                );
                continue;
            }
        };

        let market_order = db::MarketOrder::from_nats(&market_order);

        let market_order = match market_order {
            Some(market_order) => market_order,
            None => {
                warn!("Failed to parse market order message");
                continue;
            }
        };

        market_orders.write().await.push(market_order);
    }

    _ = tokio::join!(join_handle);

    pool.close().await;

    Ok(())
}

async fn handle_market_orders_messages(
    market_orders: Arc<RwLock<Vec<db::MarketOrder>>>,
    pool: Pool<Postgres>,
    token: tokio_util::sync::CancellationToken,
) {
    while !token.is_cancelled() {
        select! {
            _ = token.cancelled() => {},
            _ = tokio::time::sleep(Duration::from_secs(60)) => {}
        }

        let mut lock = market_orders.write().await;
        if lock.is_empty() {
            continue;
        }
        let market_orders: Vec<db::MarketOrder> = lock.drain(..).collect();
        drop(lock);

        let start = chrono::Utc::now();

        let result = utils::db::insert_market_orders(&pool, &market_orders).await;

        let end = chrono::Utc::now();

        match result {
            Ok(result) => {
                info!(
                    "Inserted {} market orders in {} ms",
                    result.rows_affected(),
                    end.signed_duration_since(start).num_milliseconds()
                );
            }
            Err(err) => {
                warn!("Failed to insert market orders: {}", err);
            }
        };

        let start = chrono::Utc::now();

        let result = utils::db::insert_market_orders_backup(&pool, &market_orders).await;

        let end = chrono::Utc::now();

        match result {
            Ok(rows_affected) => {
                info!(
                    "Backed up {} market orders in {} ms",
                    rows_affected.rows_affected(),
                    end.signed_duration_since(start).num_milliseconds()
                );
            }
            Err(err) => {
                warn!("Failed to insert market orders backup: {}", err);
            }
        }
    }
}
