use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;

use zero_to_prod::startup::run;
use zero_to_prod::configuration::get_configuration;
use zero_to_prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subsciber = get_subscriber("zero_to_prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subsciber);
   
    let configuration   = get_configuration().expect("Failed to read config");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.database.with_db());
    let address         = format!("{}:{}", configuration.application.host, configuration.application.port);
    let listener        = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}