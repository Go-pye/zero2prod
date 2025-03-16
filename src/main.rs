use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    println!("Loading configuration...");
    let configuration = get_configuration().expect("Failed to read configuration.");
    println!("Configuration loaded successfully");
    
    println!("Connecting to database at {}:{}", 
             configuration.database.host, configuration.database.port);
    
    let connection_pool =
        PgPoolOptions::new().connect_lazy_with(configuration.database.without_db());
    println!("Database connection pool created");
    
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    println!("Starting server on {}", address);
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
