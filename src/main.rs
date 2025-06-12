use axum::body::{Body, Bytes};
use axum::extract::Request;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::{Json, Router, extract::Query, routing::get};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

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
    info!(
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
    // tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_target(true)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE),
        )
        .init();

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_origin(Any)
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    info!("Starting server on http:// ");
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/page", get(list_things))
        .layer(middleware::from_fn(make_request_response_inspecter(true)))
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

fn make_request_response_inspecter(
    log_enabled: bool,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response, (StatusCode, String)>> + Send>,
> + Clone
+ Send
+ Sync
+ 'static {
    move |req, next| {
        let fut = request_response_inspecter(req, next, log_enabled);
        Box::pin(fut)
    }
}

async fn request_response_inspecter(
    req: Request<Body>,
    next: Next,
    log_enabled: bool,
) -> Result<Response, (StatusCode, String)> {
    let (parts, body) = req.into_parts();
    let bytes = request_inspect_print("request", log_enabled, body).await?;
    tracing::info!(
        "req method = {:?}, uri = {:?}, version = {:?}",
        parts.method.clone(),
        parts.uri.clone(),
        parts.version.clone(),
    );
    let req = Request::from_parts(parts, Body::from(bytes));
    let mut res = next.run(req).await;
    if log_enabled && tracing::enabled!(tracing::Level::DEBUG) {
        let (parts, body) = res.into_parts();
        let bytes = response_print("response", body).await?;
        res = Response::from_parts(parts, Body::from(bytes));
    }

    Ok(res)
}

/// This function inspects forbidden request and collects the body data into bytes and prints it to the log.
async fn request_inspect_print<B>(
    direction: &str,
    log_enabled: bool,
    body: B,
) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        if log_enabled {
            tracing::info!("{} body = {:?}", direction, body_str);
        }
    }

    Ok(bytes)
}

async fn response_print<B>(direction: &str, body: B) -> Result<Bytes, (StatusCode, String)>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        tracing::debug!("{} body = {:?}", direction, body_str);
    }

    Ok(bytes)
}
