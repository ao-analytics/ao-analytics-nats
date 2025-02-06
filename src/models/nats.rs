use std::str::FromStr;

use bytes;
use serde::{self, Deserialize, Deserializer};
use serde_json;
use sqlx::types::chrono::{self, Utc};

#[derive(serde::Deserialize, Debug)]
pub struct MarketOrder {
    #[serde(rename = "Id")]
    pub id: i64,
    #[serde(rename = "ItemTypeId")]
    pub item_unique_name: String,
    #[serde(rename = "ItemGroupTypeId")]
    pub item_group_name: String,
    #[serde(rename = "LocationId")]
    pub location_id: i16,
    #[serde(rename = "QualityLevel")]
    pub quality_level: i16,
    #[serde(rename = "EnchantmentLevel")]
    pub enchantment_level: i16,
    #[serde(rename = "UnitPriceSilver")]
    pub unit_price_silver: i64,
    #[serde(rename = "Amount")]
    pub amount: i64,
    #[serde(rename = "AuctionType")]
    pub auction_type: String,
    #[serde(
        rename = "Expires",
        deserialize_with = "deserialize_utc_date_time_from_string"
    )]
    pub expires_at: chrono::DateTime<Utc>,
}

fn deserialize_utc_date_time_from_string<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let string: String = Deserialize::deserialize(deserializer)?;
    chrono::NaiveDateTime::from_str(string.as_str())
        .map(|ndt| ndt.and_utc())
        .map_err(serde::de::Error::custom)
}

impl MarketOrder {
    #[allow(dead_code)]
    pub fn parse_json(json: bytes::Bytes) -> Result<MarketOrder, serde_json::Error> {
        serde_json::from_slice(&json)
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct MarketHistories {
    #[allow(dead_code)]
    #[serde(rename = "AlbionId")]
    pub id: i64,
    #[serde(rename = "AlbionIdString")]
    pub item_unique_name: String,
    #[serde(rename = "LocationId")]
    pub location_id: i16,
    #[serde(rename = "QualityLevel")]
    pub quality_level: i16,
    #[serde(rename = "Timescale")]
    pub timescale: i16,
    #[serde(rename = "MarketHistories")]
    pub market_histories: Vec<MarketHistory>,
}

impl MarketHistories {
    #[allow(dead_code)]
    pub fn parse_json(json: bytes::Bytes) -> Result<MarketHistories, serde_json::Error> {
        serde_json::from_slice(&json)
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct MarketHistory {
    #[serde(rename = "ItemAmount")]
    pub item_amount: i64,
    #[serde(rename = "SilverAmount")]
    pub silver_amount: i64,
    #[serde(
        rename = "Timestamp",
        deserialize_with = "deserialize_utc_date_time_from_ticks"
    )]
    pub timestamp: chrono::DateTime<Utc>,
}

const fn ticks_to_epoch(ticks: i64) -> i64 {
    (ticks / 10_000) - 62_136_892_800_000
}

fn deserialize_utc_date_time_from_ticks<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let ticks: i64 = Deserialize::deserialize(deserializer)?;
    chrono::DateTime::from_timestamp_millis(ticks_to_epoch(ticks))
        .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
}
