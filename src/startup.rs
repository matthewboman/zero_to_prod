use actix_session::{storage::RedisSessionStore, SessionMiddleware};
use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use actix_web::cookie::Key;
use actix_web_flash_messages::FlashMessagesFramework;
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_lab::middleware::from_fn;
use secrecy::{ExposeSecret, Secret};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

use crate::authentication::reject_anonymous_users;
use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, 
    change_password, 
    change_password_form, 
    confirm, 
    health_check, 
    home, 
    login, 
    login_form,
    log_out,
    publish_newsletter, 
    subscribe
};


pub struct Application {
    port:   u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
    
        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.auth_token,
            timeout,
        );
    
        let address  = format!("{}:{}", configuration.application.host, configuration.application.port);
        let listener = TcpListener::bind(address)?;
        let port     = listener.local_addr().unwrap().port();
        let server   = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        ).await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub struct ApplicationBaseUrl(pub String);

pub fn get_connection_pool(db_config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(db_config.with_db())
}

async fn run(
    listener:     TcpListener, 
    db_pool:      PgPool,
    email_client: EmailClient,
    base_url:     String,
    hmac_secret:  Secret<String>,
    redis_uri:    Secret<String>,
) -> Result<Server, anyhow::Error> {
    let base_url      = web::Data::new(ApplicationBaseUrl(base_url));
    let db_pool       = web::Data::new(db_pool);
    let email_client  = web::Data::new(email_client);
    let secret_key    = Key::from(hmac_secret.expose_secret().as_bytes());
    let msg_store     = CookieMessageStore::builder(secret_key.clone()).build();
    let msg_framework = FlashMessagesFramework::builder(msg_store).build(); 
    let redis_store   = RedisSessionStore::new(redis_uri.expose_secret()).await?;
    let server        = HttpServer::new(move || {
        App::new()
            .wrap(msg_framework.clone())
            .wrap(SessionMiddleware::new(redis_store.clone(), secret_key.clone()))
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_users))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password))
                    .route("/logout", web::post().to(log_out)),
            )
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}