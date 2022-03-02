#![deny(unused_must_use)]
#![feature(with_options)]
#![feature(thread_id_value)]
#![feature(try_blocks)]
#![feature(try_trait)]
#![feature(async_closure)]
#![feature(nll)]
#![feature(drain_filter)]
#![feature(in_band_lifetimes)]
#![feature(async_stream)]
#![feature(num_as_ne_bytes)]
#![recursion_limit = "512"]

#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate tokio;
#[macro_use]
extern crate getset;

pub mod agents;
mod error;
mod event;
