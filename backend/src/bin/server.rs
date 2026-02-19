use axum::{Json, Router, routing::post};
use backend::{CompareRequest, CompareResponse, handle_compare};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    // CORS so the Electron/frontend can call us easily
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/compare-diagrams", post(compare_endpoint))
        .layer(cors);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("Server running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

async fn compare_endpoint(Json(req): Json<CompareRequest>) -> Json<CompareResponse> {
    let resp = handle_compare(req);
    Json(resp)
}
