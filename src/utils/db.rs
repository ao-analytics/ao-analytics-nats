use aodata_models::db;
use sqlx::{postgres::PgQueryResult, types::chrono, Pool, Postgres};

pub async fn insert_market_orders(
    pool: &Pool<Postgres>,
    market_orders: Vec<db::MarketOrder>,
) -> Result<PgQueryResult, sqlx::Error> {
    let mut ids: Vec<i64> = Vec::new();
    let mut item_unique_names: Vec<String> = Vec::new();
    let mut location_ids: Vec<String> = Vec::new();
    let mut quality_levels: Vec<i32> = Vec::new();
    let mut enchantment_levels: Vec<i32> = Vec::new();
    let mut unit_prices_silver: Vec<i32> = Vec::new();
    let mut amounts: Vec<i32> = Vec::new();
    let mut auction_types: Vec<String> = Vec::new();
    let mut expires_ats: Vec<chrono::NaiveDateTime> = Vec::new();
    let mut created_ats: Vec<chrono::NaiveDateTime> = Vec::new();
    let mut updated_ats: Vec<chrono::NaiveDateTime> = Vec::new();

    for market_order in market_orders.iter().rev() {
        ids.push(market_order.id);
        item_unique_names.push(market_order.item_unique_name.clone());
        location_ids.push(market_order.location_id.clone());
        quality_levels.push(market_order.quality_level);
        enchantment_levels.push(market_order.enchantment_level);
        unit_prices_silver.push(market_order.unit_price_silver);
        amounts.push(market_order.amount);
        auction_types.push(market_order.auction_type.clone());
        expires_ats.push(market_order.expires_at);
        created_ats.push(market_order.created_at);
        updated_ats.push(market_order.updated_at);
    }

    let transaction = pool.begin().await.unwrap();

    let result = sqlx::query!(
        "
INSERT INTO market_order (
    id,
    item_unique_name,
    location_id,
    quality_level,
    enchantment_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    created_at,
    updated_at)
SELECT DISTINCT ON (id)
    id,
    item_unique_name,
    location_id,
    quality_level,
    enchantment_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at,
    created_at,
    updated_at
FROM UNNEST(
    $1::BIGINT[],
    $2::VARCHAR[],
    $3::VARCHAR[],
    $4::INT[],
    $5::INT[],
    $6::INT[],
    $7::INT[],
    $8::VARCHAR[],
    $9::TIMESTAMP[],
    $10::TIMESTAMP[],
    $11::TIMESTAMP[]) 
    AS market_order(
        id,
        item_unique_name,
        location_id,
        quality_level,
        enchantment_level,
        unit_price_silver,
        amount,
        auction_type,
        expires_at,
        created_at,
        updated_at)
    WHERE item_unique_name IN (SELECT unique_name FROM item)
ON CONFLICT (id) DO
    UPDATE SET
        unit_price_silver = EXCLUDED.unit_price_silver,
        amount = EXCLUDED.amount,
        expires_at = EXCLUDED.expires_at,
        updated_at = EXCLUDED.updated_at",
        &ids,
        &item_unique_names,
        &location_ids,
        &quality_levels,
        &enchantment_levels,
        &unit_prices_silver,
        &amounts,
        &auction_types,
        &expires_ats,
        &created_ats,
        &updated_ats
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            transaction.commit().await.unwrap();
            return Ok(result);
        }
        Err(e) => {
            transaction.rollback().await.unwrap();
            return Err(e);
        }
    }
}
