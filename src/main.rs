use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};
use zero2prod::email_client::EmailClient;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    println!("Loading configuration...");
    let configuration = get_configuration().expect("Failed to read configuration.");
    println!("Configuration loaded successfully");

    println!(
        "Connecting to database at {}:{}",
        configuration.database.host, configuration.database.port
    );

    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(10))
        .connect_lazy_with(configuration.database.without_db());

    println!("Testing database connection...");
    match sqlx::query("SELECT 1").execute(&connection_pool).await {
        Ok(_) => println!("Database connection successful!"),
        Err(e) => {
            eprintln!("Failed to connect to the database: {}", e);
            eprintln!("Database connection details (without password):");
            eprintln!("  Host: {}", configuration.database.host);
            eprintln!("  Port: {}", configuration.database.port);
            eprintln!("  User: {}", configuration.database.username);
            eprintln!("  Database: {}", configuration.database.database_name);
            eprintln!("  SSL Required: {}", configuration.database.require_ssl);
            // Continue execution despite the error - the app might recover later
        }
    }
    println!("Database connection pool created");

    let sender_email = configuration.email_client.sender()
        .expect("Invalid sender email.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(configuration.email_client.base_url, sender_email, configuration.email_client.authorization_token, timeout);

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    println!("Starting server on {}", address);
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool, email_client)?.await
}
