use tracing::error;

#[derive(Debug, Clone)]
pub struct Config {
    pub nats_url: String,
    pub nats_user: String,
    pub nats_password: String,
    pub nats_market_order_subject: String,
    pub nats_market_history_subject: String,

    pub db_url: String,
}

impl Config {
    pub fn from_env() -> Option<Self> {
        let nats_url = get_var_from_env_or_dotenv("NATS_URL")?;
        let nats_user = get_var_from_env_or_dotenv("NATS_USER")?;
        let nats_password = get_var_from_env_or_dotenv("NATS_PASSWORD")?;
        let nats_market_order_subject = get_var_from_env_or_dotenv("NATS_MARKET_ORDER_SUBJECT")?;
        let nats_market_history_subject =
            get_var_from_env_or_dotenv("NATS_MARKET_HISTORY_SUBJECT")?;

        let db_url = get_var_from_env_or_dotenv("DATABASE_URL")?;

        Some(Config {
            nats_url,
            nats_user,
            nats_password,
            nats_market_order_subject,
            nats_market_history_subject,
            db_url,
        })
    }
}

fn get_var_from_env_or_dotenv(name: &str) -> Option<String> {
    let var = std::env::var(name).or(dotenv::var(name));

    match var {
        Ok(var) => Some(var),
        Err(_) => {
            error!("{} is not set", name);
            None
        }
    }
}
