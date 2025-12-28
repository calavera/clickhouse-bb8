//! Basic example of using ClickHouse-BB8 with a connection pool.

use clickhouse_bb8::{ConnectionBuilder, ConnectionManager, Pool};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a connection builder with configuration
    let builder = ConnectionBuilder::new()
        .with_url("http://localhost:8123")
        .with_database("default");

    // Create a connection manager
    let manager = ConnectionManager::new(builder);

    // Create a connection pool
    let pool = Pool::builder()
        .max_size(5)
        .min_idle(Some(1))
        .connection_timeout(Duration::from_secs(10))
        .build(manager)
        .await?;

    // Get a connection from the pool
    let conn = pool.get().await?;

    // Use the connection to query
    let _result = conn
        .query("SELECT 1 as value")
        .fetch_optional::<u8>()
        .await?;

    println!("Connection from pool successful!");

    Ok(())
}
