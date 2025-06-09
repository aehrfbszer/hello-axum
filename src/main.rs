use axum::{Json, Router, extract::Query, routing::get};

use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Deserialize)]
struct Pagination {
    page: usize,
    page_size: usize,
}
#[derive(Debug, Serialize)]
struct SomeData {
    id: usize,
    name: String,
}

// This will parse query strings like `?page=2&per_page=30` into `Pagination`
// structs.
async fn list_things(Query(pagination): Query<Pagination>) -> Json<Vec<SomeData>> {
    println!(
        "page: {}, page_size: {}",
        pagination.page, pagination.page_size
    );
    let vec = Vec::from_iter(
        (0..pagination.page_size)
            .map(|i| SomeData {
                id: i + 1,
                name: format!("Item {}", i + 1),
            })
            .collect::<Vec<_>>(),
    );
    Json(vec)
}

#[tokio::main]
async fn main() {
    let cors_layer = CorsLayer::new()
        .allow_origin(Any) // Open access to selected route
        .allow_methods(Any);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/page", get(list_things))
        .layer(
            ServiceBuilder::new().layer(cors_layer).layer(
                CompressionLayer::new()
                    .gzip(true)
                    .br(true)
                    .deflate(true)
                    .zstd(true),
            ),
        );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
