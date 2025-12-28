# ClickHouse-BB8

ClickHouse-BB8 is a [ClickHouse client](https://docs.rs/clickhouse/latest/clickhouse/) pool manager compatible with [BB8](https://docs.rs/bb8/latest/bb8).

It allows you to configure a pool of Rust clients for ClickHouse that can be reused without
sharing the underlying http client connection.

## Installation

ClickHouse-BB8 is a Rust crate that can be added to your project with Cargo:

```sh
cargo add clickhouse-bb8
```

## Usage

The first step to use the pool manager is to create a `ConnectionBuilder`.
This builder exposes the same configuration options as the `clickhouse::Client`,
allowing you to configure URL, database, credentials, and other settings.

```rust
use clickhouse_bb8::ConnectionBuilder;

let builder = ConnectionBuilder::new()
    .with_url("http://localhost:8123")
    .with_database("my_database")
    .with_user("default")
    .with_password("password");
```

Then, create a new `ConnectionManager` with the builder. This connection manager
implements BB8's `ManageConnection` trait to keep track of the established connections.
Internally the `ConnectionManager` executes `select 1;` to verify that connections are
still valid. If the health check fails, the `ConnectionManager` marks the connection as
broken and recycles it.

```rust
use clickhouse_bb8::ConnectionManager;

let manager = ConnectionManager::new(builder);
```

Once the `ConnectionManager` is initialized, use the `Pool` to create your
connection pool. This `Pool` is a type alias for `bb8::Pool<ConnectionManager>`
that allows you to configure the same options as the BB8 builder directly without 
having to import the BB8 crate yourself into your project.

```rust
use clickhouse_bb8::Pool;

let pool = Pool::builder()
    .max_size(5)
    .min_idle(Some(1))
    .build(manager)
    .await
    .unwrap();
```

Finally, use your connection pool instance to get clients as necessary in your codebase.
For example, you can add your pool as part of the state for your Axum API, and use it
to perform queries on your database when your API gets specific requests.

```rust
use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use clickhouse_bb8::{ConnectionBuilder, ConnectionManager, Pool};
use serde_json::json;

#[derive(Clone)]
struct AppState {
    pool: Pool,
}

async fn query_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.pool.get().await {
        Ok(conn) => {
            // Use the connection to perform queries
            // The Connection implements Deref to Client, so you can use it like a Client
            let _result = conn
                .query("SELECT 1")
                .fetch_optional::<u8>()
                .await;
            Json(json!({"status": "ok"}))
        }
        Err(_) => Json(json!({"status": "error"})),
    }
}

#[tokio::main]
async fn main() {
    let builder = ConnectionBuilder::new()
        .with_url("http://localhost:8123")
        .with_database("default");

    let manager = ConnectionManager::new(builder);
    let pool = Pool::builder()
        .max_size(5)
        .build(manager)
        .await
        .unwrap();

    let app = Router::new()
        .route("/query", get(query_handler))
        .with_state(AppState { pool });

    // Start your server...
}
```

## LICENSE

MIT
