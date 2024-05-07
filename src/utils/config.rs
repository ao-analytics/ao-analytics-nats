use tracing::error;

pub struct Config {
    pub nats_url: String,
    pub nats_user: String,
    pub nats_password: String,
    pub nats_subject: String,

    pub db_url: String,
}


impl Config {
    pub fn from_env() -> Option<Self> {
        let nats_url = get_var_from_env_or_dotenv("NATS_URL")?;
        let nats_user = get_var_from_env_or_dotenv("NATS_USER")?;
        let nats_password = get_var_from_env_or_dotenv("NATS_PASSWORD")?;
        let nats_subject = get_var_from_env_or_dotenv("NATS_SUBJECT")?;

        let db_url = get_var_from_env_or_dotenv("DATABASE_URL")?;

        Some(Config {
            nats_url,
            nats_user,
            nats_password,
            nats_subject,
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
        },
    }
}