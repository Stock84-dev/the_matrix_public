
use memmap2::Mmap;
use mouse::ext::StaticSize;
use mouse::helpers::ptr_as_slice;

use mouse::num::NumExt;
use rayon::prelude::*;

#[derive(Serialize, Deserialize, Readable, Writable, PartialEq, Debug, Clone)]
pub struct Hlcv {
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: f32,
}

pub fn change_timeframe_src_offset(
    src_start_ts: u32,
    src_timeframe: u32,
    dest_timeframe: u32,
) -> usize {
    let ratio = dest_timeframe / src_timeframe;
    let offset = if src_start_ts % dest_timeframe == 0 {
        0
    } else {
        let offset_s = dest_timeframe - src_start_ts % dest_timeframe;
        let offset_s = offset_s.div_ceil(ratio);
        offset_s as usize
    };
    offset
}
// returns the number of dest items if candles aren't alligned in timestamp then they could be
// different
pub fn change_timeframe(
    src: &[Hlcv],
    src_start_ts: u32,
    src_timeframe: u32,
    dest_timeframe: u32,
    dest: &mut [Hlcv],
) -> usize {
    change_timeframe_with(
        src,
        src_start_ts,
        src_timeframe,
        dest_timeframe,
        |id, hlcv| {
            let dest = unsafe { &mut *(dest as *const _ as *mut [Hlcv]) };
            dest[id] = hlcv;
        },
    )
}

pub fn change_timeframe_dest_len(
    src_len: usize,
    src_start_ts: u32,
    src_timeframe: u32,
    dest_timeframe: u32,
) -> usize {
    let ratio = dest_timeframe / src_timeframe;
    let offset = change_timeframe_src_offset(src_start_ts, src_timeframe, dest_timeframe);
    let dest_count = (src_len - offset) / ratio as usize;
    dest_count
}

pub fn change_timeframe_with(
    src: &[Hlcv],
    src_start_ts: u32,
    src_timeframe: u32,
    dest_timeframe: u32,
    on_new_hlcv: impl Fn(usize, Hlcv) + Send + Sync,
) -> usize {
    if dest_timeframe % src_timeframe != 0 {
        panic!("Invalid timeframe");
    }
    let ratio = dest_timeframe / src_timeframe;
    let offset = change_timeframe_src_offset(src_start_ts, src_timeframe, dest_timeframe);
    let src_slice = &src[offset..];
    let dest_count = src_slice.len() / ratio as usize;
    src_slice
        .par_chunks_exact(ratio as usize)
        .enumerate()
        .for_each(|(id, chunk)| unsafe {
            let mut chunk_iter = chunk.iter();
            let mut hlcv = chunk_iter.next().unwrap_unchecked().clone();
            for cur_hlcv in chunk_iter {
                hlcv.high.max_mut(cur_hlcv.high);
                hlcv.low.min_mut(cur_hlcv.low);
                hlcv.volume += cur_hlcv.volume;
            }
            hlcv.close = chunk.get_unchecked(chunk.len() - 1).close;
            on_new_hlcv(id, hlcv);
        });
    dest_count
}

#[derive(Default, Debug, Clone)]
pub struct Hlcvs {
    pub hlcvs: Vec<Hlcv>,
    pub start_ts: u32,
}

pub struct MappedHlcvs {
    pub map: Mmap,
    pub start_ts: u32,
}

impl AsRef<[Hlcv]> for MappedHlcvs {
    fn as_ref(&self) -> &[Hlcv] {
        let bytes = self.map.as_ref();
        unsafe { ptr_as_slice(bytes.as_ptr(), bytes.len() / Hlcv::size()) }
    }
}
