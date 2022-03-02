use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

use bevy::prelude::*;
use bytemuck::{from_bytes, Pod};
use mouse::mem::{Arena, Const};
use mouse::num::traits::{PrimInt, ToPrimitive, Unsigned};
use zigzag::ZigZag;

pub struct CompressionPlugin;

impl Plug for CompressionPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(PipelinePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.init_resource::<Arena<Vec<u8>>>()
            .add_pipeline::<Decompress>()
            .add_pipeline::<Decompressed>()
            .add_pipeline::<Compress>()
            .add_pipeline::<Compressed>()
            .add_system(decompress)
            .add_system(compress)
    }
}

pub struct Decompress {
    pub method: DecompressionMethod,
    pub data: Const<Vec<u8>>,
}

pub struct Decompressed(pub Const<Vec<u8>>);

pub struct Compress {
    pub id: Entity,
    pub level: i32,
    pub method: CompressionMethod,
    pub data: Const<Vec<u8>>,
}

pub struct Compressed(pub Const<Vec<u8>>);

#[non_exhaustive]
pub enum CompressionMethod {
    None,
    Zstd,
}

#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum DecompressionMethod {
    None,
    Zstd,
}

fn decompress(
    arena: Res<Arena<Vec<u8>>>,
    writer: PipelinedWriter<Decompressed>,
    reader: PipelinedReader<Decompress>,
) {
    reader.par_iter().for_each(|e| {
        match e.method {
            DecompressionMethod::None => {
                writer.send(Decompressed(e.data.clone()), e.id);
                return;
            }
            _ => {}
        }
        let mut output = arena.alloc();
        match e.method {
            DecompressionMethod::None => {}
            DecompressionMethod::Zstd => {
                let mut decoder = zstd::stream::read::Decoder::with_buffer(&**e.data).unwrap();
                std::io::copy(&mut decoder, &mut *output).unwrap();
            }
        }
        writer.send(Decompressed(output.into()), e.id);
    })
}

fn compress(
    arena: Res<Arena<Vec<u8>>>,
    writer: PipelinedWriter<Compressed>,
    reader: PipelinedReader<Compress>,
) {
    reader.par_iter().for_each(|e| {
        match e.method {
            CompressionMethod::None => {
                writer.send(Compressed(e.data.clone()), e.id);
                return;
            }
            _ => {}
        }
        let mut output = arena.alloc();
        match e.method {
            CompressionMethod::None => {}
            CompressionMethod::Zstd => {
                let mut decoder =
                    zstd::stream::read::Encoder::with_buffer(&**e.data, e.level).unwrap();
                std::io::copy(&mut decoder, &mut *output).unwrap();
            }
        }
        writer.send(Compressed(output.into()), e.id);
    });
}

#[cfg(target_endian = "big")]
compile_error!("big endian architectures aren't supported");

// (1000, 1010, 1020) => 10 bits of storage
// delta(-) encoding removes offset (1000, 1010, 1020) -> (0, 10, 20) => 5 bits of storage
// obelus(/) encoding removes stride (must be applied after delta encoding)
// (0, 10, 20) -> (0, 1, 2) => 2 bits of storage
// fast floating point compression of price data
// discard sign bit
// delta encode exponent
// delta encode mantissa
// obelus encode mantissa
// zigzag exponent
// zigzag mantissa
// altough xoring mantissa is faster (because we don't have to do zigzag) we get bigger numbers and
// lower compression ratio
// whenever delta of exponent is not zero or mantissa is not divisible then start new block
// if it becomes not divisible that means that exchange has changed tick size
// TODO: adaptive varint encoding using googles algorithm but with dynamic word size
//  (google uses byte) we could use 2 bits or more just loop over bits and test if it is set from
//  left to right if it is set then go to next f32 else next bit, put that into lookup where we
//  count frequencies then calculate total bits required for different encoding sizes
//  after zigzag

struct BlockHeader {
    n_deltas: u8,
}

pub struct DeltaEncoder {}

impl DeltaEncoder {
    fn delta_pass<T: Unsigned + PrimInt + bytemuck::Pod>(&mut self, input: &[T], output: &mut [T]) {
        // find min/max -> determine bits
        //        output.extend_from_slice(input[0].to_be().as_bytes());
        //        let mut prev_value = input[0];
        //        for i in 0..input.len() {
        //            let delta =
        //        }
    }
}

// fn unsigned_to_delta<T>(input: &[T::UInt], output: &mut [T]) -> f32
// where
//    T: PrimInt + Pod + ZigZag + TryFrom<<T as ZigZag>::UInt>,
//    <T as ZigZag>::UInt: Pod + PrimInt,
//    <T as TryFrom<<T as ZigZag>::UInt>>::Error: Debug
//{
//    output[0] = *from_bytes(input[0].as_bytes());
//    for i in 1..output.len() {
//        let cur: T = input[i].try_into().unwrap();
//        let prev: T = input[i - 1].try_into().unwrap();
//        output[i] = cur - prev;
//    }
//}
// fn delta<T>(input: &[T], output: &mut [T]) -> f32
//    where
//        T: PrimInt + ToPrimitive,
//{
//    output[0] = input[0];
//    let mut average = 0.0f32;
//    for i in 1..output.len() {
//        let delta = input[i] - input[i - 1];
//        average += delta.to_f32().unwrap();
//        output[i] = delta;
//    }
//    average
//}
//
//#[cfg(test)]
// mod t_compression {
//    #[test]
//    fn t_zigzag() {}
//}
