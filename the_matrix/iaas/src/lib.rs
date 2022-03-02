#![feature(macro_attributes_in_derive_output)]
#![deny(unused_must_use)]

#[macro_use]
pub extern crate mouse;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate paste;
#[macro_use]
extern crate converters;

pub use tokio;

pub mod azure;
pub mod mysql;
pub mod oracle;
