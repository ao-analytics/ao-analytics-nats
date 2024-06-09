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
    let market_orders: Arc<RwLock<Vec<db::MarketOrder>>> = Arc::new(RwLock::new(Vec::new()));

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
        let market_orders = market_orders.clone();

        async move {
            while !token.is_cancelled() {
                select! {
                        _ = token.cancelled() => {},
                        _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                }

                let market_orders = {
                    let mut lock = market_orders.write().await;
                    if lock.is_empty() {
                        continue;
                    }
                    lock.drain(..).collect::<Vec<db::MarketOrder>>()
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
                    }
                }

                let start = chrono::Utc::now();
                let result = utils::db::insert_market_orders_backup(&pool, &market_orders).await;
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

    _ = tokio::join!(inserter_join_handle);
}

async fn handle_market_histories_messages(
    client: async_nats::Client,
    pool: Pool<Postgres>,
    token: tokio_util::sync::CancellationToken,
    config: utils::config::Config,
) {
    let market_histories: Arc<RwLock<Vec<Vec<db::MarketHistory>>>> =
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
        let market_history = market_histories.clone();

        async move {
            while !token.is_cancelled() {
                select! {
                    _ = token.cancelled() => {},
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                }

                {
                    let lock = market_history.read().await;
                    if lock.is_empty() {
                        continue;
                    }
                }

                let market_histories = {
                    let mut lock = market_history.write().await;
                    if lock.is_empty() {
                        continue;
                    }
                    let market_history: Vec<Vec<db::MarketHistory>> = lock.drain(..).collect();
                    market_history
                };

                info!("received {} market histories", market_histories.len());

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
                    }
                };

                let start = chrono::Utc::now();

                let result =
                    utils::db::insert_market_histories_backup(&pool, &market_histories).await;

                let end = chrono::Utc::now();

                match result {
                    Ok(rows_affected) => {
                        info!(
                            "Backed up {} market history entries in {} ms",
                            rows_affected.rows_affected(),
                            end.signed_duration_since(start).num_milliseconds()
                        );
                    }
                    Err(err) => {
                        warn!("Failed to insert market history backup: {}", err);
                    }
                }
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

        let market_history = db::MarketHistory::from_nats(market_history);

        let market_history = match market_history {
            Some(market_history) => market_history,
            None => {
                warn!("Failed to parse market history message");
                continue;
            }
        };

        market_histories.write().await.push(market_history);
    }

    _ = tokio::join!(inserter_join_handle);
}
