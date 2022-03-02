use serde::Serializer;

use crate::timeframes::*;

pub fn timeframe_serializer<S>(x: &u32, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let timeframes = [y1, M1, w1, d1, h1, m1, s1];
    let mut t = s1;
    for timeframe in &timeframes {
        if x % timeframe == 0 {
            t = *timeframe;
            break;
        }
    }
    let count = x / t;
    let mut string = count.to_string();
    string.push_str(timeframe_to_label(t));
    s.serialize_str(&string)
}

pub fn timeframe_to_label(timeframe: u32) -> &'static str {
    #![allow(non_upper_case_globals)]
    match timeframe {
        y1 => "y",
        M1 => "M",
        w1 => "w",
        d1 => "d",
        h1 => "h",
        m1 => "m",
        s1 => "s",
        _ => panic!("Invalid timeframe"),
    }
}
