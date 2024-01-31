use std::str::FromStr;
use sqlx::types::chrono;
use super::nats;

pub struct MarketOrder {
    pub id: i64,
    pub item_unique_name: String,
    pub location_id: String,
    pub quality_level: i32,
    pub enchantment_level: i32,
    pub unit_price_silver: i32,
    pub amount: i32,
    pub auction_type: String,
    pub expires_at: chrono::NaiveDateTime,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl MarketOrder {
    pub fn from_nats(market_order: &nats::MarketOrder) -> Option<MarketOrder> {

        let expires_at = chrono::NaiveDateTime::from_str(market_order.expires.as_str());

        let expires_at = match expires_at {
            Ok(expires_at) => expires_at,
            Err(_) => return None,
        };

        let now = chrono::Local::now().naive_utc();

        let market_order = MarketOrder {
            id: market_order.id.as_i64()?,
            item_unique_name: market_order.item_id.clone(),
            location_id: format!("{:0>4}", market_order.location_id.to_string()),
            quality_level: market_order.quality_level.as_i64()? as i32,
            enchantment_level: market_order.enchantment_level.as_i64()? as i32,
            unit_price_silver: market_order.unit_price_silver.as_i64()? as i32,
            amount: market_order.amount.as_i64()? as i32,
            auction_type: market_order.auction_type.clone(),
            expires_at: expires_at,
            created_at: now,
            updated_at: now,
        };

        return Some(market_order);
    }
}