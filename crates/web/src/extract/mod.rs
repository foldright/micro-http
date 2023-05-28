mod extract_body;
mod extract_header;
mod extract_tuple;
mod extract_url;
mod from_request;

pub use from_request::FromRequest;

/// Represented as form data
///
/// when `post` as a `application/x-www-form-urlencoded`, we can using this struct to inject data,
/// note: the struct must impl [`de::Deserialize`] and [`Send`]
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
pub struct Form<T>(pub T);

/// Represented as json data
///
/// when `post` as a `application/json`, we can using this struct to inject data,
/// note: the struct must impl [`de::Deserialize`] and [`Send`]
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
pub struct Json<T>(pub T);

/// Represented as url query data
///
/// when request with url query, we can using this struct to inject data,
/// note: the struct must impl [`de::Deserialize`] and [`Send`]
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
pub struct Query<T>(pub T);
