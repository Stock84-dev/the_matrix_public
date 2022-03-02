#![deny(unused_must_use)]
#![feature(extend_one)]
#![feature(try_blocks)]
#![feature(bufreader_seek_relative)]
#![feature(option_result_unwrap_unchecked)]

#[allow(pub_use_of_private_extern_crate)]
#[macro_use]
extern crate macros;
#[macro_use]
extern crate log;
#[macro_use]
pub extern crate speedy;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate serde;
//#[macro_use]
// extern crate num_enum; // do not load, it overrides `Default` derive macro
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate mouse;



pub use serde::{Deserialize, Serialize};
pub use speedy::{Readable, Writable};
pub use uuid;

pub mod account;
pub mod candles;
pub mod candles_builder;
pub mod compression;
pub mod error;
pub mod hlcv;
pub mod minable_models;
pub mod model_snapshot;
pub mod non_minable_models;
pub mod order;
pub mod output_reader;
pub mod structs;
pub mod variable;

mod a {
    

    use speedy::{Readable, Writable};

    #[derive(Clone, Readable, Writable)]
    struct Dummy {
        a: u64,
        b: u32,
        c: u16,
        d: u8,
        e: f32,
        f: f64,
        g: bool,
    }

    // speedy in memory read overhead 11 ms
    // 17.49 ms
    // 18.56 ms
    // 28.649 ms
    #[test]
    fn read_many_small_structs_file_only() {
        let mut buffer: Vec<Dummy> = Vec::new();
        let mut file = std::fs::File::open("/home/stock/cache/rsi/run.seek_f32").unwrap();
        let mut buffer = vec![0u8; 1024 * 1024 * std::mem::size_of::<Dummy>()];
        let now = std::time::Instant::now();
        file.read_exact(&mut buffer).unwrap();
        println!("{} us", now.elapsed().as_micros());
        let mut file2 =
            std::io::BufWriter::new(std::fs::File::create("/home/stock/data/speedy").unwrap());
        let slice: &[Dummy] = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const _,
                buffer.len() / std::mem::size_of::<Dummy>(),
            )
        };
        let now = std::time::Instant::now();
        Writable::write_to_stream_with_ctx(slice, Endianness::NATIVE, file2).unwrap();
        println!("{} us", now.elapsed().as_micros());
    }

    // 2492 ms
    #[test]
    fn read_speedy_many_small_structs_file_unbuffered() {
        let file = std::fs::File::open("/home/stock/data/speedy").unwrap();
        let now = std::time::Instant::now();
        let deserialized: Vec<Dummy> =
            Readable::read_from_stream_unbuffered_with_ctx(Endianness::NATIVE, &file).unwrap();
        println!("{} us", now.elapsed().as_micros());
    }

    // 34.888 ms
    // 62 ms
    // 34 ms
    #[test]
    fn read_speedy_many_small_structs_file_buffered() {
        let file = std::fs::File::open("/home/stock/data/speedy").unwrap();
        let dummy = Dummy {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: 5.0,
            f: 6.0,
            g: true,
        };
        let now = std::time::Instant::now();
        let deserialized: Vec<Dummy> =
            Readable::read_from_stream_buffered_with_ctx(Endianness::NATIVE, file).unwrap();
        println!("{} us", now.elapsed().as_micros());
    }
}
