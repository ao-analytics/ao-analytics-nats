use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::{select, signal};

use models::nats;

use futures_util::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::{self};
use sqlx::{Pool, Postgres};

use tracing::{info, warn};

mod models;
mod utils;

#[tokio::main]
async fn main() -> Result<(), async_nats::SubscribeError> {
    tracing_subscriber::fmt::init();

    let config = utils::config::Config::from_env();

    let config = match config {
        Some(config) => config,
        None => {
            panic!("Failed to initialize config");
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .unwrap();

    let client = async_nats::ConnectOptions::new()
        .user_and_password(
            config.nats_user.to_string(),
            config.nats_password.to_string(),
        )
        .connect(&config.nats_url)
        .await
        .unwrap();

    info!("Connected to NATS server at {}", &config.nats_url);

    let token = tokio_util::sync::CancellationToken::new();
    let mut int_signal = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
    let mut term_signal = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();

    let market_orders_join_handle = tokio::spawn(handle_market_orders_messages(
        client.clone(),
        pool.clone(),
        token.clone(),
        config.clone(),
    ));

    let market_history_join_handle = tokio::spawn(handle_market_histories_messages(
        client.clone(),
        pool.clone(),
        token.clone(),
        config.clone(),
    ));

    select! {
        _ = int_signal.recv() => {
            info!("Received interrupt signal");
            token.cancel();
        },
        _ = term_signal.recv() => {
            info!("Received terminate signal");
            token.cancel();
        }
    };

    _ = tokio::join! {
        market_orders_join_handle,
        market_history_join_handle,
    };

    pool.close().await;

    Ok(())
}

async fn handle_market_orders_messages(
    client: async_nats::Client,
    pool: Pool<Postgres>,
    token: tokio_util::sync::CancellationToken,
    config: utils::config::Config,
) {
    let market_orders: Arc<RwLock<Vec<nats::MarketOrder>>> = Arc::new(RwLock::new(Vec::new()));

    let subscription = client
        .subscribe(config.nats_market_order_subject.to_string())
        .await;

    let mut subscription = match subscription {
        Ok(subscription) => subscription,
        Err(err) => {
            warn!("Failed to subscribe to market orders: {}", err);
            return;
        }
    };

    let inserter_join_handle = tokio::spawn({
        let token = token.clone();
        let global_market_orders = market_orders.clone();

        async move {
            while !token.is_cancelled() {
                select! {
                        _ = token.cancelled() => {},
                        _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                }

                let mut market_orders: Vec<nats::MarketOrder> = {
                    let mut lock = global_market_orders.write().await;
                    if lock.is_empty() {
                        continue;
                    }
                    lock.drain(..).collect()
                };

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
                        global_market_orders
                            .write()
                            .await
                            .append(&mut market_orders);
                    }
                }
            }
        }
    });

    while let Some(msg) = select! {
        _ = token.cancelled() => None,
        msg = subscription.next() => msg,
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

        market_orders.write().await.push(market_order);
    }

    _ = tokio::join!(inserter_join_handle);
}

async fn handle_market_histories_messages(
    client: async_nats::Client,
    pool: Pool<Postgres>,
    token: tokio_util::sync::CancellationToken,
    config: utils::config::Config,
) {
    let market_histories: Arc<RwLock<Vec<nats::MarketHistories>>> =
        Arc::new(RwLock::new(Vec::new()));

    let subscription = client
        .subscribe(config.nats_market_history_subject.to_string())
        .await;

    let mut subscription = match subscription {
        Ok(subscription) => subscription,
        Err(err) => {
            warn!("Failed to subscribe to market histories: {}", err);
            return;
        }
    };

    let inserter_join_handle = tokio::spawn({
        let token = token.clone();
        let global_market_histories = market_histories.clone();

        async move {
            while !token.is_cancelled() {
                select! {
                    _ = token.cancelled() => {},
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                }

                {
                    let lock = global_market_histories.read().await;
                    if lock.is_empty() {
                        continue;
                    }
                }

                let mut market_histories: Vec<nats::MarketHistories> = {
                    let mut lock = global_market_histories.write().await;
                    if lock.is_empty() {
                        continue;
                    }
                    lock.drain(..).collect()
                };

                let start = chrono::Utc::now();

                let result = utils::db::insert_market_histories(&pool, &market_histories).await;

                let end = chrono::Utc::now();

                match result {
                    Ok(result) => {
                        info!(
                            "Inserted {} market history entries in {} ms",
                            result.rows_affected(),
                            end.signed_duration_since(start).num_milliseconds()
                        );
                    }
                    Err(err) => {
                        warn!("Failed to insert market history entries: {}", err);
                        global_market_histories
                            .write()
                            .await
                            .append(&mut market_histories);
                    }
                };
            }
        }
    });

    while let Some(msg) = select! {
        _ = token.cancelled() => None,
        msg = subscription.next() => msg
    } {
        let parse_result = serde_json::from_slice::<nats::MarketHistories>(&msg.payload);

        let market_history = match parse_result {
            Ok(market_history) => market_history,
            Err(err) => {
                warn!(
                    "Failed to parse market history message: {:?} {}",
                    &msg.payload, err
                );
                continue;
            }
        };

        market_histories.write().await.push(market_history);
    }

    _ = tokio::join!(inserter_join_handle);
}
