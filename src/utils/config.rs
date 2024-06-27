use config::Config;

#[derive(Debug, Clone, Config)]
pub struct Config {
    pub nats_url: String,
    pub nats_user: String,
    pub nats_password: String,
    pub nats_market_order_subject: String,
    pub nats_market_history_subject: String,

    pub database_url: String,
}
