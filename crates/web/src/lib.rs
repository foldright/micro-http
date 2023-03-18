#![feature(type_alias_impl_trait)]
//#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
mod extract;
mod fn_trait;
mod handler;
mod responder;
mod body;

pub use extract::FromRequest;
pub use handler::FnHandler;