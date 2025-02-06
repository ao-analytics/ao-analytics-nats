use crate::models::nats;
use sqlx::{
    postgres::PgQueryResult,
    types::chrono::{self, Utc},
    Pool, Postgres,
};

pub async fn insert_market_orders(
    pool: &Pool<Postgres>,
    market_orders: &[nats::MarketOrder],
) -> Result<PgQueryResult, sqlx::Error> {
    let mut ids: Vec<i64> = Vec::new();
    let mut item_unique_names: Vec<String> = Vec::new();
    let mut item_group_names: Vec<String> = Vec::new();
    let mut location_ids: Vec<i16> = Vec::new();
    let mut quality_levels: Vec<i16> = Vec::new();
    let mut enchantment_levels: Vec<i16> = Vec::new();
    let mut unit_prices_silver: Vec<i64> = Vec::new();
    let mut amounts: Vec<i64> = Vec::new();
    let mut auction_types: Vec<String> = Vec::new();
    let mut expires_ats: Vec<chrono::DateTime<Utc>> = Vec::new();

    for market_order in market_orders.iter() {
        ids.push(market_order.id);
        item_group_names.push(market_order.item_group_name.clone());
        item_unique_names.push(market_order.item_unique_name.clone());
        location_ids.push(market_order.location_id);
        quality_levels.push(market_order.quality_level);
        enchantment_levels.push(market_order.enchantment_level);
        unit_prices_silver.push(market_order.unit_price_silver);
        amounts.push(market_order.amount);
        auction_types.push(market_order.auction_type.clone());
        expires_ats.push(market_order.expires_at);
    }

    let transaction = pool.begin().await?;

    insert_items(
        pool,
        &item_group_names,
        &item_unique_names,
        &enchantment_levels,
    )
    .await?;

    insert_locations(pool, &location_ids).await?;

    // ensure unique ids (this is faster than a trigger)
    sqlx::query!(
        "
DELETE FROM market_order WHERE id IN (SELECT id FROM UNNEST($1::BIGINT[]) as id(id))",
        &ids
    )
    .execute(pool)
    .await?;

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
    expires_at)
SELECT DISTINCT ON (id)
    id,
    item_unique_name,
    location_id,
    quality_level,
    unit_price_silver,
    amount,
    auction_type,
    expires_at
FROM UNNEST(
    $1::BIGINT[],
    $2::VARCHAR[],
    $3::SMALLINT[],
    $4::SMALLINT[],
    $5::BIGINT[],
    $6::BIGINT[],
    $7::VARCHAR[],
    $8::TIMESTAMPTZ[])
    AS market_order(
        id,
        item_unique_name,
        location_id,
        quality_level,
        unit_price_silver,
        amount,
        auction_type,
        expires_at)
ON CONFLICT DO NOTHING",
        &ids,
        &item_unique_names,
        &location_ids,
        &quality_levels,
        &unit_prices_silver,
        &amounts,
        &auction_types,
        &expires_ats,
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            transaction.commit().await?;
            Ok(result)
        }
        Err(e) => {
            transaction.rollback().await?;
            Err(e)
        }
    }
}

pub async fn insert_market_histories(
    pool: &Pool<Postgres>,
    market_histories: &[nats::MarketHistories],
) -> Result<PgQueryResult, sqlx::Error> {
    let mut item_unique_names: Vec<String> = Vec::new();
    let mut item_group_names: Vec<String> = Vec::new();
    let mut location_ids: Vec<i16> = Vec::new();
    let mut quality_levels: Vec<i16> = Vec::new();
    let mut enchantment_levels: Vec<i16> = Vec::new();
    let mut timescales: Vec<i16> = Vec::new();
    let mut silver_amounts: Vec<i64> = Vec::new();
    let mut item_amounts: Vec<i64> = Vec::new();
    let mut timestamps: Vec<chrono::DateTime<Utc>> = Vec::new();
    let mut updated_ats: Vec<chrono::DateTime<Utc>> = Vec::new();

    for market_history in market_histories.iter() {
        for histories in market_history.market_histories.iter() {
            item_unique_names.push(market_history.item_unique_name.clone());
            item_group_names.push(
                match market_history
                    .item_unique_name
                    .split('@')
                    .next()
                    .map(|str| str.to_string())
                {
                    Some(group_name) => group_name,
                    None => continue,
                },
            );
            location_ids.push(market_history.location_id);
            quality_levels.push(market_history.quality_level);
            enchantment_levels.push(
                market_history
                    .item_unique_name
                    .split('@')
                    .rev()
                    .next()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0),
            );
            timescales.push(market_history.timescale);
            silver_amounts.push(histories.silver_amount);
            item_amounts.push(histories.item_amount);
            timestamps.push(histories.timestamp);
            updated_ats.push(sqlx::types::chrono::Utc::now());
        }
    }

    let transaction = pool.begin().await?;

    insert_items(
        pool,
        &item_group_names,
        &item_unique_names,
        &enchantment_levels,
    )
    .await?;

    insert_locations(pool, &location_ids).await?;

    let result = sqlx::query!(
        "
INSERT INTO market_history (
    item_unique_name,
    location_id,
    quality_level,
    silver_amount,
    item_amount,
    timescale,
    timestamp,
    updated_at)
SELECT DISTINCT ON (item_unique_name, location_id, quality_level, timescale, timestamp)
    item_unique_name,
    location_id,
    quality_level,
    silver_amount,
    item_amount,
    timescale,
    timestamp,
    updated_at
FROM UNNEST(
    $1::VARCHAR[],
    $2::SMALLINT[],
    $3::SMALLINT[],
    $4::BIGINT[],
    $5::BIGINT[],
    $6::SMALLINT[],
    $7::TIMESTAMPTZ[],
    $8::TIMESTAMPTZ[]) AS market_history(
        item_unique_name,
        location_id,
        quality_level,
        silver_amount,
        item_amount,
        timescale,
        timestamp,
        updated_at)
ON CONFLICT (item_unique_name, location_id, quality_level, timescale, timestamp)
DO UPDATE SET
    updated_at = EXCLUDED.updated_at,
    silver_amount = EXCLUDED.silver_amount,
    item_amount = EXCLUDED.item_amount
        ",
        &item_unique_names,
        &location_ids,
        &quality_levels,
        &silver_amounts,
        &item_amounts,
        &timescales,
        &timestamps,
        &updated_ats
    )
    .execute(pool)
    .await;

    match result {
        Ok(result) => {
            transaction.commit().await?;
            Ok(result)
        }
        Err(e) => {
            transaction.rollback().await?;
            Err(e)
        }
    }
}

async fn insert_items(
    pool: &Pool<Postgres>,
    item_group_names: &[String],
    item_unique_names: &[String],
    enchantment_levels: &[i16],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
    INSERT INTO item_group (name)
    SELECT DISTINCT ON (name)
        name
    FROM UNNEST($1::TEXT[]) AS
        item_group(name)
    ON CONFLICT DO NOTHING",
        &item_group_names,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "
    INSERT INTO item (
        unique_name,
        item_group_name,
        enchantment_level)
    SELECT DISTINCT ON (unique_name)
        unique_name,
        item_group_name,
        enchantment_level
    FROM UNNEST(
        $1::TEXT[],
        $2::TEXT[],
        $3::SMALLINT[]) AS item (
            unique_name,
            item_group_name,
            enchantment_level)
    ON CONFLICT DO NOTHING",
        &item_unique_names,
        &item_group_names,
        &enchantment_levels,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_locations(pool: &Pool<Postgres>, location_ids: &[i16]) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
    INSERT INTO location (id)
    SELECT DISTINCT ON (id)
        id
    FROM UNNEST($1::SMALLINT[]) AS
        location(id)
    ON CONFLICT DO NOTHING",
        &location_ids,
    )
    .execute(pool)
    .await?;

    Ok(())
}
