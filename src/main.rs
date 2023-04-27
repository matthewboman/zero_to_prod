use secrecy::ExposeSecret;
use sqlx::PgPool;
use std::net::TcpListener;

use zero_to_prod::startup::run;
use zero_to_prod::configuration::get_configuration;
use zero_to_prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subsciber = get_subscriber("zero_to_prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subsciber);
   
    let configuration   = get_configuration().expect("Failed to read config");
    let connection_pool = PgPool::connect(&configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to db");
    let address         = format!("127.0.0.1:{}", configuration.application_port);
    let listener        = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}