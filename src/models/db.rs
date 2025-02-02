use std::str::FromStr;

use ao_analytics_models::json::nats;
use sqlx::{self, types::chrono::Utc};

#[derive(sqlx::FromRow, Debug)]
pub struct MarketOrder {
    pub id: i64,
    pub item_unique_name: String,
    pub location_id: String,
    pub quality_level: i32,
    pub unit_price_silver: i32,
    pub amount: i32,
    pub auction_type: String,
    pub expires_at: sqlx::types::chrono::DateTime<Utc>,
    pub created_at: sqlx::types::chrono::DateTime<Utc>,
    pub updated_at: sqlx::types::chrono::DateTime<Utc>,
}

impl MarketOrder {
    #[allow(dead_code)]
    pub fn from_nats(nats_market_order: &nats::MarketOrder) -> Option<MarketOrder> {
        let expires_at =
            sqlx::types::chrono::NaiveDateTime::from_str(nats_market_order.expires.as_str())
                .ok()?;

        Some(Self {
            id: nats_market_order.id.as_i64()?,
            item_unique_name: nats_market_order.item_id.clone(),
            location_id: format!("{:0>4}", nats_market_order.location_id.as_i64()?),
            quality_level: nats_market_order.quality_level.as_i64()? as i32,
            unit_price_silver: nats_market_order.unit_price_silver.as_i64()? as i32,
            amount: nats_market_order.amount.as_i64()? as i32,
            auction_type: nats_market_order.auction_type.clone(),
            expires_at: expires_at.and_utc(),
            created_at: sqlx::types::chrono::Utc::now(),
            updated_at: sqlx::types::chrono::Utc::now(),
        })
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct MarketHistory {
    pub item_id: String,
    pub location_id: String,
    pub quality_level: i32,
    pub timescale: i32,
    pub timestamp: sqlx::types::chrono::DateTime<Utc>,
    pub item_amount: i32,
    pub silver_amount: i32,
    pub updated_at: sqlx::types::chrono::DateTime<Utc>,
}

impl MarketHistory {
    #[allow(dead_code)]
    pub fn from_nats(nats_market_history: nats::MarketHistories) -> Option<Vec<MarketHistory>> {
        let mut market_historie_entries: Vec<MarketHistory> = Vec::new();

        for market_history in nats_market_history.market_histories {
            market_historie_entries.push(Self {
                item_id: nats_market_history.item_id.clone(),
                location_id: format!("{:0>4}", nats_market_history.location_id.to_string()),
                quality_level: nats_market_history.quality_level.as_i64()? as i32,
                timescale: nats_market_history.timescale.as_i64()? as i32,
                timestamp: sqlx::types::chrono::DateTime::from_timestamp_millis(
                    market_history.timestamp.as_i64()? / 10000 - 62136892800000,
                )?,
                item_amount: market_history.item_amount.as_i64()? as i32,
                silver_amount: market_history.silver_amount.as_i64()? as i32,
                updated_at: sqlx::types::chrono::Utc::now(),
            });
        }

        Some(market_historie_entries)
    }
}
