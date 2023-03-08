#![feature(type_alias_impl_trait)]
//#![feature(return_position_impl_trait_in_trait)]
#![feature(async_fn_in_trait)]
#![feature(impl_trait_projections)]
mod extract;
mod handler;
mod responder;

pub use extract::FromRequest;
