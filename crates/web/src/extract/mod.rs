//! Request data extraction module
//!
//! This module provides functionality for extracting typed data from HTTP requests.
//! It includes extractors for common data formats and patterns:
//!
//! - Form data (`Form<T>`) - For `application/x-www-form-urlencoded` request bodies
//! - JSON data (`Json<T>`) - For `application/json` request bodies  
//! - Query parameters (`Query<T>`) - For URL query strings
//! - Headers and other request metadata
//! - Raw request body as bytes or string
//!
//! # Core Concepts
//!
//! The module is built around the [`FromRequest`] trait, which defines how to extract
//! typed data from requests. Types implementing this trait can be used as parameters
//! in request handlers.
//!
//! # Common Extractors
//!
//! ## Form Data
//! ```no_run
//! # use serde::Deserialize;
//! # use micro_web::extract::Form;
//! #[derive(Deserialize)]
//! struct LoginForm {
//!     username: String,
//!     password: String,
//! }
//!
//! async fn handle_login(Form(form): Form<LoginForm>) {
//!     println!("Login attempt from: {}", form.username);
//! }
//! ```
//!
//! ## JSON Data
//! ```no_run
//! # use serde::Deserialize;
//! # use micro_web::extract::Json;
//! #[derive(Deserialize)]
//! struct User {
//!     name: String,
//!     email: String,
//! }
//!
//! async fn create_user(Json(user): Json<User>) {
//!     println!("Creating user: {}", user.name);
//! }
//! ```
//!
//! ## Query Parameters
//! ```no_run
//! # use serde::Deserialize;
//! # use micro_web::extract::Query;
//! #[derive(Deserialize)]
//! struct Pagination {
//!     page: u32,
//!     per_page: u32,
//! }
//!
//! async fn list_items(Query(params): Query<Pagination>) {
//!     println!("Listing page {} with {} items", params.page, params.per_page);
//! }
//! ```
//!
//! # Optional Extraction
//!
//! All extractors can be made optional by wrapping them in `Option<T>`:
//!
//! ```
//! # use serde::Deserialize;
//! # use micro_web::extract::Json;
//! #[derive(Deserialize)]
//! struct UpdateUser {
//!     name: Option<String>,
//!     email: Option<String>,
//! }
//!
//! async fn update_user(Json(update): Json<UpdateUser>) {
//!     if let Some(name) = update.name {
//!         println!("Updating name to: {}", name);
//!     }
//! }
//! ```
//!
//! # Multiple Extractors
//!
//! Multiple extractors can be combined using tuples:
//!
//! ```
//! # use serde::Deserialize;
//! # use micro_web::extract::{Json, Query};
//! # use http::Method;
//! #[derive(Deserialize)]
//! struct SearchParams {
//!     q: String,
//! }
//!
//! #[derive(Deserialize)]
//! struct Payload {
//!     data: String,
//! }
//!
//! async fn handler(
//!     method: Method,
//!     Query(params): Query<SearchParams>,
//!     Json(payload): Json<Payload>,
//! ) {
//!     // Access to method, query params, and JSON payload
//! }
//! ```

mod from_request;
mod extract_tuple;
mod extract_body;
mod extract_header;
mod extract_url;

pub use from_request::FromRequest2;
use serde::Deserialize;

/// Represented as form data
///
/// when `post` as a `application/x-www-form-urlencoded`, we can using this struct to inject data,
/// note: the struct must impl [`serde::Deserialize`] and [`Send`]
///
/// # Example
/// ```
/// # use serde::Deserialize;
/// # use micro_web::extract::Form;
/// # #[allow(dead_code)]
/// #[derive(Deserialize, Debug)]
/// struct Params {
///     name: String,
///     zip: String,
/// }
///
/// pub async fn handle(Form(params) : Form<Params>) -> String {
///     format!("received params: {:?}", params)
/// }
/// ```
pub struct Form<T>(pub T)
where
    T: for<'de> Deserialize<'de> + Send;

/// Represented as json data
///
/// when `post` as a `application/json`, we can using this struct to inject data,
/// note: the struct must impl [`serde::Deserialize`] and [`Send`]
///
/// # Example
/// ```
/// # use serde::Deserialize;
/// # use micro_web::extract::Json;
/// # #[allow(dead_code)]
/// #[derive(Deserialize, Debug)]
/// struct Params {
///     name: String,
///     zip: String,
/// }
///
/// pub async fn handle(Json(params) : Json<Params>) -> String {
///     format!("received params: {:?}", params)
/// }
/// ```
pub struct Json<T>(pub T)
where
    T: for<'de> Deserialize<'de> + Send;

/// Represented as url query data
///
/// when request with url query, we can using this struct to inject data,
/// note: the struct must impl [`serde::Deserialize`] and [`Send`]
///
/// # Example
/// ```
/// # use serde::Deserialize;
/// # use micro_web::extract::Query;
/// # #[allow(dead_code)]
/// #[derive(Deserialize, Debug)]
/// struct Params {
///     name: String,
///     zip: String,
/// }
///
/// pub async fn handle(Query(params) : Query<Params>) -> String {
///     format!("received params: {:?}", params)
/// }
/// ```
pub struct Query<T>(pub T)
where
    T: for<'de> Deserialize<'de> + Send;
