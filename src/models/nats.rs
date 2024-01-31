use serde_json::Number;

#[derive(serde::Deserialize)]
pub struct MarketOrder {
    #[serde(rename = "Id")]
    pub id: Number,

    #[serde(rename = "ItemTypeId")]
    pub item_id: String,

    #[serde(rename = "ItemGroupTypeId")]
    pub group_type_id: String,

    #[serde(rename = "LocationId")]
    pub location_id: Number,

    #[serde(rename = "QualityLevel")]
    pub quality_level: Number,

    #[serde(rename = "EnchantmentLevel")]
    pub enchantment_level: Number,

    #[serde(rename = "UnitPriceSilver")]
    pub unit_price_silver: Number,

    #[serde(rename = "Amount")]
    pub amount: Number,

    #[serde(rename = "AuctionType")]
    pub auction_type: String,

    #[serde(rename = "Expires")]
    pub expires: String,
}
