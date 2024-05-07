use aodata_models::db;
use sqlx::{postgres::PgQueryResult, types::chrono::{self, Utc}, Pool, Postgres};
use tracing::warn;

pub async fn insert_market_orders(
    pool: &Pool<Postgres>,
    market_orders: &[db::MarketOrder],
) -> Result<PgQueryResult, sqlx::Error> {
    let mut ids: Vec<i64> = Vec::new();
    let mut item_unique_names: Vec<String> = Vec::new();
    let mut location_ids: Vec<String> = Vec::new();
    let mut quality_levels: Vec<i32> = Vec::new();
    let mut unit_prices_silver: Vec<i32> = Vec::new();
    let mut amounts: Vec<i32> = Vec::new();
    let mut auction_types: Vec<String> = Vec::new();
    let mut expires_ats: Vec<chrono::DateTime<Utc>> = Vec::new();
    let mut updated_ats: Vec<chrono::DateTime<Utc>> = Vec::new();
    let mut created_ats: Vec<chrono::DateTime<Utc>> = Vec::new();


    for market_order in market_orders.iter().rev() {
        ids.push(market_order.id);
        item_unique_names.push(market_order.item_unique_name.clone());
        location_ids.push(market_order.location_id.clone());
        quality_levels.push(market_order.quality_level);
        unit_prices_silver.push(market_order.unit_price_silver);
        amounts.push(market_order.amount);
        auction_types.push(market_order.auction_type.clone());
        expires_ats.push(market_order.expires_at);
        updated_ats.push(market_order.updated_at);
        created_ats.push(market_order.created_at);
    }

    let transaction = pool.begin().await.unwrap();


    // ensure unique ids (this is faster than a trigger)
    let result = sqlx::query!(
        "
DELETE FROM market_order WHERE id IN (SELECT id FROM UNNEST($1::BIGINT[]) as id(id))",
        &ids
    )
    .execute(pool)
    .await;

    match result {
        Ok(_) => {}
        Err(e) => {
            warn!("Failed to delete market orders: {:?}", e);
        }
        
    }

    let result = sqlx::query!(
        "
INSERT INTO market_order (
    id,
    item_unique_name,
    location_id,
    quality_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    updated_at)
SELECT DISTINCT ON (id)
    id,
    item_unique_name,
    location_id,
    quality_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    updated_at
FROM UNNEST(
    $1::BIGINT[],
    $2::VARCHAR[],
    $3::VARCHAR[],
    $4::INT[],
    $5::INT[],
    $6::INT[],
    $7::VARCHAR[],
    $8::TIMESTAMPTZ[],
    $9::TIMESTAMPTZ[]) 
    AS market_order(
        id,
        item_unique_name,
        location_id,
        quality_level,
        unit_price_silver,
        amount,
        auction_type,
        expires_at,
        updated_at)
    WHERE item_unique_name IN (SELECT unique_name FROM item)
ON CONFLICT DO NOTHING",
        &ids,
        &item_unique_names,
        &location_ids,
        &quality_levels,
        &unit_prices_silver,
        &amounts,
        &auction_types,
        &expires_ats,
        &updated_ats
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            transaction.commit().await.unwrap();
            Ok(result)
        }
        Err(e) => {
            transaction.rollback().await.unwrap();
            Err(e)
        }
    }
}

pub async fn insert_market_orders_backup(
    pool: &Pool<Postgres>,
    market_orders: &[db::MarketOrder],
) -> Result<PgQueryResult, sqlx::Error> {
    let mut ids: Vec<i64> = Vec::new();
    let mut item_unique_names: Vec<String> = Vec::new();
    let mut location_ids: Vec<String> = Vec::new();
    let mut quality_levels: Vec<i32> = Vec::new();
    let mut unit_prices_silver: Vec<i32> = Vec::new();
    let mut amounts: Vec<i32> = Vec::new();
    let mut auction_types: Vec<String> = Vec::new();
    let mut expires_ats: Vec<chrono::DateTime<Utc>> = Vec::new();
    let mut updated_ats: Vec<chrono::DateTime<Utc>> = Vec::new();
    let mut created_ats: Vec<chrono::DateTime<Utc>> = Vec::new();


    for market_order in market_orders.iter().rev() {
        ids.push(market_order.id);
        item_unique_names.push(market_order.item_unique_name.clone());
        location_ids.push(market_order.location_id.clone());
        quality_levels.push(market_order.quality_level);
        unit_prices_silver.push(market_order.unit_price_silver);
        amounts.push(market_order.amount);
        auction_types.push(market_order.auction_type.clone());
        expires_ats.push(market_order.expires_at);
        updated_ats.push(market_order.updated_at);
        created_ats.push(market_order.created_at);
    }

    let transaction = pool.begin().await.unwrap();

    let result = sqlx::query!(
        "
INSERT INTO market_order_backup (
    id,
    item_unique_name,
    location_id,
    quality_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    updated_at)
SELECT DISTINCT ON (id)
    id,
    item_unique_name,
    location_id,
    quality_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    updated_at
FROM UNNEST(
    $1::BIGINT[],
    $2::VARCHAR[],
    $3::VARCHAR[],
    $4::INT[],
    $5::INT[],
    $6::INT[],
    $7::VARCHAR[],
    $8::TIMESTAMPTZ[],
    $9::TIMESTAMPTZ[]) 
    AS market_order(
        id,
        item_unique_name,
        location_id,
        quality_level,
        unit_price_silver,
        amount,
        auction_type,
        expires_at,
        updated_at)
        ",
        &ids,
        &item_unique_names,
        &location_ids,
        &quality_levels,
        &unit_prices_silver,
        &amounts,
        &auction_types,
        &expires_ats,
        &updated_ats
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            transaction.commit().await.unwrap();
            Ok(result)
        }
        Err(e) => {
            transaction.rollback().await.unwrap();
            Err(e)
        }
    }
}
