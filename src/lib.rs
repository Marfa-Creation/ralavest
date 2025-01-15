//! # Example
//! ```
//! use ralavest::{
//!     extractor::Query,
//!     http::IntoResponse,
//!     route::{get, post, serve, Router},
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::new()
//!         .route(
//!             "/",
//!             get(|| "<h1>hello world</h1>").put(|| "1230912 row affected"),
//!         )
//!         .route("/login", post(login_handler));

//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:1945")
//!         .await
//!         .unwrap();

//!     serve(listener, router).await.unwrap();
//! }

//! fn login_handler(Query(mut query): Query) -> impl IntoResponse {
//!     let usr = query.remove("usr").unwrap_or_default();
//!     let pw = query.remove("pw").unwrap_or_default();

//!     if usr != "admin" || pw != "admin1234" {
//!         return "<h1>unauthorized</h1>";
//!     }

//!     return "<h1>admin panel</h1>";
//! }
//! ```


pub mod extractor;
pub mod handler;
pub mod http;
pub mod route;
