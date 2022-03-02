#![feature(test)]
#![feature(bench_black_box)]
#![allow(warnings)]

use std::time::Instant;

use bevy::utils::{HashMap, HashSet};
use bitvec::order::{Lsb0, Msb0};
use bitvec::prelude::BitVec;
use bytemuck::{cast_slice, Pod};
use dozer::provider::load_hlcv;
use ieee754::Ieee754;
use mouse::num::NumExt;
use mouse::prelude::*;
use mouse::rand::random;
use plotters::prelude::*;
use zigzag::ZigZag;
// custom compression is not worth it
// if we use custom compression we could get 20% higher ratios at cost of 100x slower compression
// speed

// compression using only zstd on raw data
// high
// 727.4048 MiB -> 727.4048 MiB ratio: 1 297 ms 2446.5806 MiB/s ours
// 727.4048 MiB -> 36.805603 MiB ratio: 19.763426 7.2778883 ms 99.947235 MiB/s, level: 9
// low
// 727.4048 MiB -> 727.4048 MiB ratio: 1 274 ms 2653.5012 MiB/s ours
// 727.4048 MiB -> 37.214825 MiB ratio: 19.546103 7.3450994 ms 99.03267 MiB/s, level: 9
// close
// 727.4048 MiB -> 727.4048 MiB ratio: 1 224 ms 3240.6912 MiB/s ours
// 727.4048 MiB -> 40.05561 MiB ratio: 18.159872 7.8914523 ms 92.176285 MiB/s, level: 9
// volume
// 727.4048 MiB -> 727.4048 MiB ratio: 1 236 ms 3077.7427 MiB/s ours
// 727.4048 MiB -> 150.46733 MiB ratio: 4.834304 24.159534 ms 30.108395 MiB/s, level: 9

// close
// 727.4048 MiB -> 727.4048 MiB ratio: 1 238 ms 3050.9424 MiB/s ours
// 727.4048 MiB -> 34.91545 MiB ratio: 20.83332 194.627 ms 3.73743 MiB/s, level: 19

// clos + high + low
// 2182.2144 MiB -> 2182.2144 MiB ratio: 1 1100 ms 1983.1332 MiB/s ours
// 2182.2144 MiB -> 114.07414 MiB ratio: 19.12979 24.16854 ms 90.29153 MiB/s, level: 9

fn count_bits_le(mut n: u32) -> u32 {
    let mut count = 0;
    // While loop will run until we get n = 0
    while n != 0 {
        count += 1;
        n = n >> 1;
    }
    count
}

fn count_zeros_le(mut n: u32) -> u32 {
    let mut count = 0;
    // While loop will run until we get n = 0
    while n != 0 {
        count += 1;
        n = n << 1;
    }
    31 - count
}

fn extend_u8(bits: &mut BitVec<Lsb0, u8>, mask: u8, value: u8) {
    let lookup = [
        1 << 7,
        1 << 6,
        1 << 5,
        1 << 4,
        1 << 3,
        1 << 2,
        1 << 1,
        1 << 0,
    ];
    for i in 0..u8::size() * 8 {
        if lookup[i] & mask != 0 {
            bits.push(lookup[i] & value != 0);
        }
    }
}

fn extend_u32(bits: &mut BitVec<Lsb0, u8>, mask: u32, value: u32) {
    let lookup = [
        1 << 31,
        1 << 30,
        1 << 29,
        1 << 28,
        1 << 27,
        1 << 26,
        1 << 25,
        1 << 24,
        1 << 23,
        1 << 22,
        1 << 21,
        1 << 20,
        1 << 19,
        1 << 18,
        1 << 17,
        1 << 16,
        1 << 15,
        1 << 14,
        1 << 13,
        1 << 12,
        1 << 11,
        1 << 10,
        1 << 9,
        1 << 8,
        1 << 7,
        1 << 6,
        1 << 5,
        1 << 4,
        1 << 3,
        1 << 2,
        1 << 1,
        1 << 0,
    ];
    for i in 0..u32::size() * 8 {
        dbg!(mask);
        if lookup[i] & mask != 0 {
            dbg!(lookup[i] & value != 0);
            bits.push(lookup[i] & value != 0);
        }
        println!("end");
    }
}

macro_rules! transmute {
    ($data:expr) => {
        unsafe { std::mem::transmute($data) }
    };
}

fn bytes<T: Pod>(data: &[T]) -> Vec<u8> {
    let mut b = Vec::with_capacity(T::size() * data.len());
    data.iter()
        .map(|x| x.as_bytes())
        .for_each(|x| b.extend_from_slice(x));
    b
}

// 727.4048 MiB -> 727.4048 MiB ratio: 1 224 ms 3240.6912 MiB/s ours
// 727.4048 MiB -> 40.05561 MiB ratio: 18.159872 7.8914523 ms 92.176285 MiB/s, level: 9
fn normal(data: &[f32]) -> Vec<f32> {
    Vec::from(data)
}

// 727.4048 MiB -> 727.4048 MiB ratio: 1 633 ms 1148.2742 MiB/s ours
// 727.4048 MiB -> 41.087997 MiB ratio: 17.703583 8.029728 ms 90.588974 MiB/s, level: 9
fn xor(data: &[f32]) -> Vec<u32> {
    let mut out = Vec::<u32>::new();
    out.push(transmute!(data[0]));
    for i in 1..data.len() {
        let a: u32 = transmute!(data[i]);
        let b: u32 = transmute!(data[i - 1]);
        out.push(a ^ b);
    }
    out
}
// 727.4048 MiB -> 909.256 MiB ratio: 0.8 1664 ms 437.033 MiB/s ours
// 909.256 MiB -> 41.112724 MiB ratio: 22.116169 9.508516 ms 95.625435 MiB/s, level: 9
fn xor_split(data: &[f32]) -> Vec<u8> {
    let mut exp = Vec::new();
    let mut mantissa = Vec::new();
    exp.push(data[0].decompose_raw().1);
    mantissa.push(data[0].decompose_raw().2);
    for i in 1..data.len() {
        let a: u32 = transmute!(data[i]);
        let b: u32 = transmute!(data[i - 1]);
        let xor: f32 = transmute!(a ^ b);
        exp.push(xor.decompose_raw().1);
        mantissa.push(xor.decompose_raw().2);
    }
    mantissa
        .iter()
        .map(|x| x.as_bytes())
        .for_each(|x| exp.extend_from_slice(x));
    exp
}
// 727.4048 MiB -> 909.256 MiB ratio: 0.8 1694 ms 429.34918 MiB/s ours
// 909.256 MiB -> 40.19878 MiB ratio: 22.618994 9.754444 ms 93.21454 MiB/s, level: 9
fn delta_split(data: &[f32]) -> Vec<i8> {
    let mut exp = Vec::with_capacity(data.len());
    let mut mantissa = Vec::with_capacity(data.len());
    exp.push(data[0].decompose_raw().1 as i8);
    mantissa.push(data[0].decompose_raw().2 as i32);
    let mut prev_exp = exp[0] as i8;
    let mut prev_mantissa = mantissa[0] as i32;
    for i in 1..data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        let de = e - prev_exp;
        let dm = m - prev_mantissa;
        exp.push(de);
        mantissa.push(dm);
        prev_exp = e;
        prev_mantissa = m;
    }
    mantissa
        .iter()
        .map(|x| x.as_bytes())
        .for_each(|x| exp.extend_from_slice(transmute!(x)));
    exp
}

// 727.4048 MiB -> 909.256 MiB ratio: 0.8 2039 ms 356.6945 MiB/s ours
// 909.256 MiB -> 41.586334 MiB ratio: 21.864298 11.825518 ms 76.88932 MiB/s, level: 9
fn delta_split_zigzag(data: &[f32]) -> Vec<u8> {
    let mut exp = Vec::with_capacity(data.len());
    let mut mantissa = Vec::with_capacity(data.len());
    exp.push(data[0].decompose_raw().1);
    mantissa.push(data[0].decompose_raw().2);
    let mut prev_exp = exp[0] as i8;
    let mut prev_mantissa = mantissa[0] as i32;
    for i in 1..data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        let de = e - prev_exp;
        let dm = m - prev_mantissa;
        let zde = ZigZag::encode(de);
        let zdm = ZigZag::encode(dm);
        exp.push(zde);
        mantissa.push(zdm);
        prev_exp = e;
        prev_mantissa = m;
    }
    mantissa
        .iter()
        .map(|x| x.as_bytes())
        .for_each(|x| exp.extend_from_slice(transmute!(x)));
    exp
}

// 727.4048 MiB -> 591.01636 MiB ratio: 1.2307693 28133 ms 25.855822 MiB/s ours
// 591.01636 MiB -> 37.125057 MiB ratio: 15.919608 37.557785 ms 15.736188 MiB/s, level: 9
fn delta_split_zigzag_fix_len(data: &[f32]) -> Vec<u8> {
    let mut bits = BitVec::<Lsb0, u8>::new();
    let mut exp = Vec::with_capacity(data.len());
    let mut mantissa = Vec::with_capacity(data.len());
    exp.push(data[0].decompose_raw().1);
    mantissa.push(data[0].decompose_raw().2);
    let mut mim = u32::MAX;
    let mut mie = u8::MAX;
    let mut mam = u32::MIN;
    let mut mae = u8::MIN;
    let mut prev_exp = exp[0] as i8;
    let mut prev_mantissa = mantissa[0] as i32;
    for i in 1..data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        let de = e - prev_exp;
        let dm = m - prev_mantissa;
        let zde = ZigZag::encode(de);
        let zdm = ZigZag::encode(dm);
        mim.min_mut(zdm);
        mie.min_mut(zde);
        mam.max_mut(zdm);
        mae.max_mut(zde);
        exp.push(zde);
        mantissa.push(zdm);
        prev_exp = e;
        prev_mantissa = m;
    }
    let range_e = mae - mie;
    let range_m = mam - mim;
    let bits_e = count_bits_le(range_e as u32);
    let bits_m = count_bits_le(range_m as u32);
    let mask_e = (1 << bits_e) - 1;
    let mask_m = (1 << bits_m) - 1;

    for i in 0..data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        let de = e - prev_exp;
        let dm = m - prev_mantissa;
        let zde = ZigZag::encode(de);
        let zdm = ZigZag::encode(dm);
        extend_u8(&mut bits, mask_e, zde);
        prev_exp = e;
        prev_mantissa = m;
    }
    for i in 0..data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        let de = e - prev_exp;
        let dm = m - prev_mantissa;
        let zde = ZigZag::encode(de);
        let zdm = ZigZag::encode(dm);
        extend_u32(&mut bits, mask_m, zdm);
        prev_exp = e;
        prev_mantissa = m;
    }
    bits.into_vec()
}

// 727.4048 MiB -> 727.45056 MiB ratio: 0.99993706 1326 ms 548.53687 MiB/s ours
// 727.45056 MiB -> 40.205975 MiB ratio: 18.093096 9.233443 ms 78.784325 MiB/s, level: 9
fn delta_split_redelta(data: &[f32]) -> Vec<i8> {
    let mut exp = Vec::with_capacity(data.len());
    let mut mantissa = Vec::with_capacity(data.len());
    exp.push(data[0].decompose_raw().1 as i8);
    mantissa.push(data[0].decompose_raw().2 as i32);
    let mut prev_exp = exp[0] as i8;
    let mut prev_mantissa = mantissa[0] as i32;
    let mut i = 1;
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            exp.push(random());
            exp.push(random());
            exp.push(random());
            exp.push(random());
            exp.push(e);
            mantissa.push(m);
            prev_exp = e;
        } else {
            let dm = m - prev_mantissa;
            mantissa.push(dm);
        }
        prev_mantissa = m;
        i += 1;
    }
    mantissa
        .iter()
        .map(|x| x.as_bytes())
        .for_each(|x| exp.extend_from_slice(cast_slice(x)));
    exp
}
// 727.4048 MiB -> 457.45932 MiB ratio: 1.5900972 28318 ms 25.686317 MiB/s ours
// 457.45932 MiB -> 40.30569 MiB ratio: 11.349745 35.805374 ms 12.776276 MiB/s, level: 9
fn delta_split_redelta_zigzag_fix_len(data: &[f32]) -> Vec<u8> {
    let mut e_bits = BitVec::<Lsb0, u8>::new();
    let mut m_bits = BitVec::<Lsb0, u8>::new();
    let mut mim = u32::MAX;
    let mut mie = u8::MAX;
    let mut mam = u32::MIN;
    let mut mae = u8::MIN;
    let mut prev_exp = data[0].decompose_raw().1 as i8;
    let mut prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut i = 1;
    let mut ranges = Vec::new();
    ranges.push(u32::MAX..u32::MIN);
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            ranges.push(u32::MAX..u32::MIN);
            mie.min_mut(zde);
            mae.max_mut(zde);
            prev_exp = e;
        } else {
            let dm = m - prev_mantissa;
            let zdm = ZigZag::encode(dm);
            ranges.last_mut().unwrap().start.min_mut(zdm);
            ranges.last_mut().unwrap().end.max_mut(zdm);
        }
        prev_mantissa = m;
        i += 1;
    }
    let range_e = mae - mie;
    let bits_e = count_bits_le(range_e as u32);
    let mask_e = (1 << bits_e) - 1;
    let mut i = 1;
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut range_i = 0;
    let mut masks: Vec<_> = ranges
        .iter()
        .map(|x| {
            let mut range_m = x.end - x.start;
            if x.end < 0 && x.start > 0 {
                range_m += 1;
            }
            let bits_m = count_bits_le(range_m as u32);
            let mask_m = (1 << bits_m) - 1;
            mask_m
        })
        .collect();
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            extend_u8(&mut e_bits, mask_e, zde);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            m_bits.extend_from_raw_slice(m.as_bytes());
            prev_exp = e;
            dbg!(ranges[range_i].end - ranges[range_i].start);
            range_i += 1;
        } else {
            let dm = m - prev_mantissa;
            let zdm = ZigZag::encode(dm);
            extend_u32(&mut m_bits, masks[range_i], zdm);
        }
        prev_mantissa = m;
        i += 1;
    }
    e_bits.extend_from_bitslice(&m_bits);
    e_bits.into_vec()
}
// 727.4048 MiB -> 320.60992 MiB ratio: 2.2688155 22740 ms 31.987625 MiB/s ours
// 320.60992 MiB -> 35.149017 MiB ratio: 9.121448 29.502419 ms 10.867242 MiB/s, level: 9
// BROKEN
fn delta_split_redelta_obelus_zigzag_fix_len(data: &[f32]) -> Vec<u8> {
    fn find_shift(shift: &mut u32, value: u32) {
        loop {
            if *shift & value == 0 {
                break;
            }
            *shift = (*shift) >> 1;
        }
    }
    let mut e_bits = BitVec::<Lsb0, u8>::new();
    let mut m_bits = BitVec::<Lsb0, u8>::new();
    let mut mim = u32::MAX;
    let mut mie = u8::MAX;
    let mut mam = u32::MIN;
    let mut mae = u8::MIN;
    let mut prev_exp = data[0].decompose_raw().1 as i8;
    let mut prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut i = 1;
    let mut ranges = Vec::new();
    ranges.push(u32::MAX..u32::MIN);
    let mut shifts = Vec::new();
    shifts.push(u32::MAX);
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            ranges.push(u32::MAX..u32::MIN);
            mie.min_mut(zde);
            mae.max_mut(zde);
            prev_exp = e;
            shifts.push(u32::MAX);
        } else {
            let dm = m - prev_mantissa;
            let zdm = ZigZag::encode(dm);
            ranges.last_mut().unwrap().start.min_mut(zdm);
            ranges.last_mut().unwrap().end.max_mut(zdm);
            find_shift(shifts.last_mut().unwrap(), transmute!(dm));
        }
        prev_mantissa = m;
        i += 1;
    }
    let range_e = mae - mie;
    let bits_e = count_bits_le(range_e as u32);
    let mask_e = (1 << bits_e) - 1;
    let mut i = 1;
    let mut shift_amounts = Vec::new();
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut range_i = 0;
    let mut masks: Vec<_> = ranges
        .iter()
        .zip(&shifts)
        .map(|(x, shift)| {
            let mut range_m = x.end - x.start;
            if x.end < 0 && x.start > 0 {
                range_m += 1;
            }
            let bits_m = count_bits_le(range_m as u32);
            let mask_m = (1 << bits_m) - 1;
            let shift_amount = count_bits_le(*shift);
            let mask_m = mask_m >> shift_amount;
            shift_amounts.push(shift_amount);
            mask_m
        })
        .collect();
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            extend_u8(&mut e_bits, mask_e, zde);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            m_bits.extend_from_raw_slice(m.as_bytes());
            prev_exp = e;
            //            dbg!(ranges[range_i].end - ranges[range_i].start);
            range_i += 1;
        } else {
            let dm = m - prev_mantissa;
            //            let zdm = ZigZag::encode(dm);
            let mut shifted: u32 = transmute!(dm);
            shifted = shifted >> shift_amounts[range_i];
            extend_u32(&mut m_bits, masks[range_i], shifted);
            if dm != 0 {
                println!("{:#034b} {:#034b}", dm, shifted);
            }
        }
        prev_mantissa = m;
        i += 1;
    }
    e_bits.extend_from_bitslice(&m_bits);
    e_bits.into_vec()
}
fn delta_split_redelta_obelus_zigzag_truncate_zigzag_fix_len(data: &[f32]) -> Vec<u8> {
    fn extend_u32(bits: &mut BitVec<Lsb0, u8>, amt: u32, value: u32) {
        let mut mask = 1;
        for i in 0..amt {
            bits.push(mask & value != 0);
            //            dbg!(mask & value != 0);
            mask <<= 1;
        }
    }
    fn find_right_shift(shift: &mut (u32, u32), value: u32) {
        if value == 0 {
            return;
        }
        loop {
            if shift.0 & value == 0 {
                break;
            }
            shift.0 = (shift.0) >> 1;
            shift.1 += 1;
        }
    }
    fn find_left_shift(shift: &mut u32, count: &mut u32, value: u32) {
        if value == 0 {
            return;
        }
        loop {
            if *shift & value == 0 {
                break;
            }
            *shift = (*shift) << 1;
            *count += 1;
        }
    }
    let mut e_bits = BitVec::<Lsb0, u8>::new();
    let mut m_bits = BitVec::<Lsb0, u8>::new();
    let mut prev_exp = data[0].decompose_raw().1 as i8;
    let mut prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut i = 1;
    //    let mut ranges = Vec::new();
    //    ranges.push(u32::MAX..u32::MIN);
    let mut right_shifts = Vec::new();
    right_shifts.push((u32::MAX, 0));
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            //            ranges.push(u32::MAX..u32::MIN);
            prev_exp = e;
            right_shifts.push((u32::MAX, 0));
        } else {
            let dm = m - prev_mantissa;
            //            let zdm = ZigZag::encode(dm);
            //            ranges.last_mut().unwrap().start.min_mut(zdm);
            //            ranges.last_mut().unwrap().end.max_mut(zdm);
            find_right_shift(right_shifts.last_mut().unwrap(), transmute!(dm));
        }
        prev_mantissa = m;
        i += 1;
    }
    right_shifts.iter_mut().for_each(|x| x.1 = 32 - x.1);
    let mut i = 1;
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut range_i = 0;
    let mut left_shifts = Vec::new();
    left_shifts.push((1, 0));
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            range_i += 1;
            left_shifts.push((1, 0));
            prev_exp = e;
        } else {
            let dm = m - prev_mantissa;
            let amt: i32 = transmute!(right_shifts[range_i].1);
            let zdm = ZigZag::encode(dm >> amt);
            //            print!("{:#034b} ", dm);
            //            println!("{:#034b}", zdm);
            let mut ls = left_shifts.last_mut().unwrap();
            find_left_shift(&mut ls.0, &mut ls.1, zdm);
        }
        prev_mantissa = m;
        i += 1;
    }
    let mut masks: Vec<_> = left_shifts
        .iter()
        .map(|(_, amt)| {
            dbg!(amt);
            *amt
        })
        .collect();
    let mut i = 1;
    let mut range_i = 0;
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            //            let de = e - prev_exp;
            //            let zde = ZigZag::encode(de);
            e_bits.extend_from_raw_slice(e.as_bytes());
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            m_bits.extend_from_raw_slice(m.as_bytes());
            prev_exp = e;
            //            dbg!(ranges[range_i].end - ranges[range_i].start);
            range_i += 1;
        } else {
            let dm = m - prev_mantissa;
            let mut rs: i32 = transmute!(right_shifts[range_i].1);
            let value = ZigZag::encode(dm >> rs);
            extend_u32(&mut m_bits, masks[range_i], value);
            //            if dm != 0 {
            //                println!("{:#034b} {:#034b}", dm, value);
            //            }
        }
        prev_mantissa = m;
        i += 1;
    }
    e_bits.extend_from_bitslice(&m_bits);
    e_bits.into_vec()
}

fn delta_split_redelta_obelus_zigzag_truncate_zigzag_fix_len_huffman(data: &[f32]) -> Vec<u8> {
    fn extend_u32(bits: &mut BitVec<Lsb0, u8>, amt: u32, value: u32) {
        let mut mask = 1;
        for i in 0..amt {
            bits.push(mask & value != 0);
            //            dbg!(mask & value != 0);
            mask <<= 1;
        }
    }
    fn find_right_shift(shift: &mut (u32, u32), value: u32) {
        if value == 0 {
            return;
        }
        loop {
            if shift.0 & value == 0 {
                break;
            }
            shift.0 = (shift.0) >> 1;
            shift.1 += 1;
        }
    }
    fn find_left_shift(shift: &mut u32, count: &mut u32, value: u32) {
        if value == 0 {
            return;
        }
        loop {
            if *shift & value == 0 {
                break;
            }
            *shift = (*shift) << 1;
            *count += 1;
        }
    }
    let mut e_bits = BitVec::<Lsb0, u8>::new();
    let mut m_bits = BitVec::<Lsb0, u8>::new();
    let mut prev_exp = data[0].decompose_raw().1 as i8;
    let mut prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut i = 1;
    //    let mut ranges = Vec::new();
    //    ranges.push(u32::MAX..u32::MIN);
    let mut right_shifts = Vec::new();
    right_shifts.push((u32::MAX, 0));
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            let de = e - prev_exp;
            let zde = ZigZag::encode(de);
            //            ranges.push(u32::MAX..u32::MIN);
            prev_exp = e;
            right_shifts.push((u32::MAX, 0));
        } else {
            let dm = m - prev_mantissa;
            //            let zdm = ZigZag::encode(dm);
            //            ranges.last_mut().unwrap().start.min_mut(zdm);
            //            ranges.last_mut().unwrap().end.max_mut(zdm);
            find_right_shift(right_shifts.last_mut().unwrap(), transmute!(dm));
        }
        prev_mantissa = m;
        i += 1;
    }
    right_shifts.iter_mut().for_each(|x| x.1 = 32 - x.1);
    let mut i = 1;
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut range_i = 0;
    let mut left_shifts = Vec::new();
    left_shifts.push((1, 0));
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            range_i += 1;
            left_shifts.push((1, 0));
            prev_exp = e;
        } else {
            let dm = m - prev_mantissa;
            let amt: i32 = transmute!(right_shifts[range_i].1);
            let zdm = ZigZag::encode(dm >> (amt));
            //            if dm != 0 {
            //                print!("{:#034b} ", dm);
            //                println!("{:#034b}", zdm);
            //            }
            let mut ls = left_shifts.last_mut().unwrap();
            find_left_shift(&mut ls.0, &mut ls.1, zdm);
        }
        prev_mantissa = m;
        i += 1;
    }
    let mut masks: Vec<_> = left_shifts
        .iter()
        .map(|(_, amt)| {
            //            dbg!(amt);
            *amt
        })
        .collect();

    let mut i = 1;
    let mut range_i = 0;
    prev_exp = data[0].decompose_raw().1 as i8;
    prev_mantissa = data[0].decompose_raw().2 as i32;
    let mut model = SourceModelBuilder::new().num_bits(masks[0]).build();
    let mut output = vec![];
    let mut compressed = Some(Cursor::new(&mut output));
    let mut encoder = ArithmeticEncoder::new(masks[0] as u64);
    let mut compressed_writer = BitWriter::new(compressed.take().unwrap());
    masks.push(u32::MAX);
    while i < data.len() {
        let e = data[i].decompose_raw().1 as i8;
        let m = data[i].decompose_raw().2 as i32;
        if prev_exp != e {
            range_i += 1;
            if masks[range_i - 1] <= 4 {
                encoder
                    .encode(model.eof(), &model, &mut compressed_writer)
                    .unwrap();
                encoder.finish_encode(&mut compressed_writer).unwrap();
                compressed_writer.pad_to_byte().unwrap();
            }

            model = SourceModelBuilder::new().num_bits(masks[range_i]).build();
            encoder = ArithmeticEncoder::new(masks[range_i] as u64);
            //            let de = e - prev_exp;
            //            let zde = ZigZag::encode(de);
            e_bits.extend_from_raw_slice(e.as_bytes());
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(&[random()]);
            e_bits.extend_from_raw_slice(m.as_bytes());
            prev_exp = e;
            //            dbg!(ranges[range_i].end - ranges[range_i].start);
            println!("{}", i as f32 / data.len() as f32);
        } else {
            let dm = m - prev_mantissa;
            let mut rs: i32 = transmute!(right_shifts[range_i].1);
            let value = ZigZag::encode(dm >> rs);
            if masks[range_i] > 4 {
                extend_u32(&mut m_bits, masks[range_i], value);
            } else {
                //                println!("{}", value);
                //                            print!("{:#034b} ", dm);
                //                println!("{:#034b}", value);
                encoder
                    .encode(value as u32, &model, &mut compressed_writer)
                    .unwrap();
                model.update_symbol(value as u32);
            }
            //            extend_u32(&mut m_bits, masks[range_i], value);
            //            if dm != 0 {
            //                println!("{:#034b} {:#034b}", dm, value);
            //            }
        }
        prev_mantissa = m;
        i += 1;
    }
    e_bits.extend_from_raw_slice(&output);
    e_bits.extend_from_bitslice(&m_bits);
    e_bits.into_vec()
}

use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};

use arcode::bitbit::BitWriter;
use arcode::encode::encoder::ArithmeticEncoder;
use arcode::util::source_model_builder::{EOFKind, SourceModelBuilder};
use merovingian::variable;
use merovingian::variable::Variable;
use ordered_float::OrderedFloat;
use plotters::prelude::{BitMapBackend, LineSeries};
extern crate test;
use test::Bencher;
// fn delta_split_fixed(data: &[f32]) -> Vec<u8> {
//    let mut exp = Vec::with_capacity(data.len());
//    let mut mantissa = Vec::with_capacity(data.len());
//    exp.push(data[0].decompose_raw().1);
//    mantissa.push(data[0].decompose_raw().2);
//    let mut prev_exp = exp[0];
//    let mut prev_mantissa = mantissa[0];
//    let mut bits = BitVec::<Lsb0, u8>::new();
//    let mut max_bits
//    for i in 1..data.len() {
//        let e = data[i].decompose_raw().1;
//        let m = data[i].decompose_raw().2;
//        exp.push(e - prev_exp);
//        mantissa.push(m - prev_mantissa);
//    }
//    bits.extend_from_raw_slice(exp[0].as_bytes());
//    bits.extend_from_raw_slice(mantissa[0].as_bytes());
//    for i in 1..data.len() {
//
//    }
//    mantissa
//        .iter()
//        .map(|x| x.as_bytes())
//        .for_each(|x| exp.extend_from_slice(x));
//    exp
//}

#[bench]
fn map10(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(HashSet::<u64>::default());
        for i in 0..10 {
            if map.insert(i as u64) {
                println!("err");
            }
        }
    });
}

#[bench]
fn array10(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(Vec::<u64>::default());
        for i in 0..10 {
            if map.contains(&(i as u64)) {
                println!("err");
            } else {
                map.push(i as u64);
            }
        }
    });
}

#[bench]
fn map100(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(HashSet::<u64>::default());
        for i in 0..100 {
            if map.insert(i as u64) {
                println!("err");
            }
        }
    });
}

#[bench]
fn array100(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(Vec::<u64>::new());
        for i in 0..100 {
            if map.contains(&(i as u64)) {
                println!("err");
            } else {
                map.push(i as u64);
            }
        }
    });
}

#[bench]
fn map1000(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(HashSet::<u64>::default());
        for i in 0..1000 {
            if map.insert(i as u64) {
                println!("err");
            }
        }
    });
}

#[bench]
fn array1000(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(Vec::<u64>::new());
        for i in 0..1000 {
            if map.contains(&(i as u64)) {
                println!("err");
            } else {
                map.push(i as u64);
            }
        }
    });
}

#[bench]
fn map10000(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(HashSet::<u64>::default());
        for i in 0..10000 {
            if map.insert(i as u64) {
                println!("err");
            }
        }
    });
}

#[bench]
fn array10000(b: &mut Bencher) {
    b.iter(|| {
        let mut map = std::hint::black_box(Vec::<u64>::new());
        for i in 0..10000 {
            if map.contains(&(i as u64)) {
                println!("err");
            } else {
                map.push(i as u64);
            }
        }
    });
}

fn main() -> Result<()> {
    //    wtf();

    panic!();
    unsafe {
        config::load("/home/stock/ssd/projects/the_matrix/the_matrix/config.yaml").unwrap();
    }
    plot()?;
    panic!();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(set_tokio_handle());
    let mapped = rt
        .block_on(load_hlcv(
            "BitMEX",
            "XBTUSD",
            1443184465,
            1633869265 - 1443184465,
        ))
        .unwrap();
    let mut map: Vec<_> = mapped.as_ref().iter().map(|x| x.close).collect();
    map.extend(mapped.as_ref().iter().map(|x| x.high));
    map.extend(mapped.as_ref().iter().map(|x| x.low));

    //    let map = Vec::from(&map[map.len() - 100..]);
    let mut now = Instant::now();
    let mega = 1024 as f32 * 1024 as f32;
    let close = map.size() as f32 / mega;
    println!("encoding...");
    let b = normal(&map); //////////////////////////////////////////////////////
    let input = b.size() as f32 / mega;
    println!(
        "// {} MiB -> {} MiB ratio: {} {} ms {} MiB/s ours",
        close,
        input,
        close / input,
        now.elapsed().as_millis(),
        close / now.elapsed().as_nanos() as f32 * 1e9,
    );
    let encoding_elapsed = now.elapsed().as_nanos() as f32 / 1e9;
    for i in 9..10 {
        let mut comp_now = Instant::now();
        let mut encoder = zstd::stream::read::Encoder::with_buffer(cast_slice(&b), i).unwrap();
        let mut out = Vec::new();
        std::io::copy(&mut encoder, &mut out).unwrap();
        let output = out.size() as f32 / mega;
        let comp_elapsed = comp_now.elapsed().as_nanos() as f32 / 1e9;
        let total_elapsed = encoding_elapsed + comp_elapsed;

        println!(
            "// {} MiB -> {} MiB ratio: {} {} ms {} MiB/s, level: {}",
            input,
            output,
            input / output,
            total_elapsed,
            input / total_elapsed,
            i,
        );
    }
    Ok(())

    //    z1(mapped.as_ref());
}

fn wtf() {
    fn p(val: i16) {
        println!("{}", val);
        println!("val: {:#018b}", val);
        println!("zig: {:#018b}", ZigZag::encode(val));
        println!("shf: {:#018b}", ZigZag::encode(val) >> 1);
        println!("bzi: {:#018b}", ZigZag::encode(val.to_be()));
        println!("bshf: {:#018b}", ZigZag::encode(val.to_be()) >> 1);
        println!("lzi: {:#018b}", ZigZag::encode(val.to_le()));
        println!("lshf: {:#018b}", ZigZag::encode(val.to_le()) >> 1);
    }
    println!("1.23 = {:#034b}", unsafe {
        *(&1.23f32 as *const _ as *const u32)
    });
    println!("-1.23 = {:#034b}", unsafe {
        *(&-1.23f32 as *const _ as *const u32)
    });
    p(3);
    p(3 >> 1);
    p(0);
    p(1);
    p(-1);
    p(-32768);
    p(-32767);
    p(32767);
    p(1337);
    p(-1337);
}

fn plot() -> Result<()> {
    let mut balance = 0.0f32;
    let mut variables = vec![
        Variable {
            min: 60.,
            max: 13440.,
            value: 60.,
            stride: 60.,
        },
        Variable {
            min: 0.0,
            max: 4.0,
            value: 0.0,
            stride: 1.0,
        },
        // there are some between 400 and 500 but not between 500 and 1000
        Variable {
            min: 2.,
            max: 482.,
            value: 2.,
            stride: 1.,
        },
        Variable {
            min: 0.,
            max: 100.,
            value: 0.,
            stride: 1.,
        },
        Variable {
            min: 0.,
            max: 100.,
            value: 0.,
            stride: 1.,
        },
    ];
    let var_id = 0;

    #[derive(Debug)]
    struct Value {
        min: f32,
        max: f32,
        average: f32,
        count: f32,
        stddev: f32,
    }
    let mut values = HashMap::default();
    let mut file = BufReader::with_capacity(
        8192 * 16,
        std::fs::File::open("/home/stock/ssd/normal.bin")?,
    );
    let mut now = Instant::now();
    loop {
        if file
            .read_exact(unsafe { balance.as_u8_slice_mut() })
            .is_err()
        {
            break;
        }
        //        if now.elapsed().as_secs() > 10 {
        //            break;
        //        }
        variable::increase(&mut variables, 1);
        match values.get_mut(&OrderedFloat(variables[var_id].value)) {
            None => {
                values.insert(
                    OrderedFloat(variables[var_id].value),
                    Value {
                        min: balance,
                        max: balance,
                        average: balance,
                        count: 1.,
                        stddev: 0.,
                    },
                );
            }
            Some(value) => {
                //                value.min.min_mut(balance);
                value.max.max_mut(balance);
                value.average += balance;
                value.count += 1.;
            }
        }
    }
    println!("stage complete");
    for (x, value) in &mut values {
        value.average /= value.count;
    }
    file.seek(SeekFrom::Start(0))?;
    variable::reset(&mut variables);
    let mut now = Instant::now();
    loop {
        //        if now.elapsed().as_secs() > 5 {
        //            break;
        //        }
        if file
            .read_exact(unsafe { balance.as_u8_slice_mut() })
            .is_err()
        {
            break;
        }
        variable::increase(&mut variables, 1);
        let mut value = values
            .get_mut(&OrderedFloat(variables[var_id].value))
            .unwrap();
        value.stddev += (balance - value.average).powi(2);
    }
    println!("plotting");
    for item in &mut values {
        item.1.stddev = (item.1.stddev / item.1.count).sqrt();
    }
    let mut series: Vec<_> = values.iter().sorted_by_key(|x| x.0).collect();
    let mut max_rel = values
        .iter()
        .map(|x| OrderedFloat(x.1.stddev / x.1.average))
        .max()
        .unwrap()
        .0;
    let mut min_rel = values
        .iter()
        .map(|x| OrderedFloat(x.1.stddev / x.1.average))
        .min()
        .unwrap()
        .0;
    let mut max = values
        .iter()
        .map(|x| OrderedFloat(x.1.average))
        .max()
        .unwrap()
        .0;
    let mut min = values
        .iter()
        .map(|x| OrderedFloat(x.1.average))
        .min()
        .unwrap()
        .0;
    let path = format!("{}.png", var_id);
    let root = BitMapBackend::new(&path, (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        //        .caption("y=x^2", ("sans-serif", 50).into_font())
        .margin(5u32)
        .x_label_area_size(30u32)
        .y_label_area_size(40u32)
        .right_y_label_area_size(40u32)
        .build_cartesian_2d(variables[var_id].min..variables[var_id].max, min..max)?
        .set_secondary_coord(
            variables[var_id].min..variables[var_id].max,
            min_rel..max_rel,
        );
    let y1_desc = "average";
    let y2_desc = "stddev / average";

    chart.configure_mesh().y_desc(y1_desc).draw()?;
    chart.configure_secondary_axes().y_desc(y2_desc).draw()?;

    chart
        .draw_secondary_series(LineSeries::new(
            series.iter().map(|x| (x.0 .0, x.1.stddev / x.1.average)),
            &BLUE,
        ))?
        .label(y2_desc)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));
    chart
        .draw_series(LineSeries::new(
            series.iter().map(|x| (x.0 .0, x.1.average)),
            &RED,
        ))?
        .label(y1_desc)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;
    dbg!(values);

    Ok(())
}
