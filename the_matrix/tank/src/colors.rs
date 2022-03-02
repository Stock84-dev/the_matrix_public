use rgb::{RGBA};
// TODO: find color library with constants defined
// code modified from

/// See https://en.wikipedia.org/wiki/Web_colors

// 16 Original "Web" Colors
pub const WHITE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const SILVER: RGBA<f32> = RGBA {
    r: 0xC0 as f32 / 255.,
    g: 0xC0 as f32 / 255.,
    b: 0xC0 as f32 / 255.,
    a: 1.0f32,
};
pub const GRAY: RGBA<f32> = RGBA {
    r: 0x80 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x80 as f32 / 255.,
    a: 1.0f32,
};
pub const BLACK: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const RED: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const MAROON: RGBA<f32> = RGBA {
    r: 0x80 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const YELLOW: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const OLIVE: RGBA<f32> = RGBA {
    r: 0x80 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const LIME: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const GREEN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const AQUA: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const TEAL: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x80 as f32 / 255.,
    a: 1.0f32,
};
pub const BLUE: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const NAVY: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x80 as f32 / 255.,
    a: 1.0f32,
};
pub const FUCHSIA: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const PURPLE: RGBA<f32> = RGBA {
    r: 0x80 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x80 as f32 / 255.,
    a: 1.0f32,
};

// Extended "X11" Colors
pub const PINK: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xC0 as f32 / 255.,
    b: 0xCB as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_PINK: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xB6 as f32 / 255.,
    b: 0xC1 as f32 / 255.,
    a: 1.0f32,
};
pub const HOT_PINK: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x69 as f32 / 255.,
    b: 0xB4 as f32 / 255.,
    a: 1.0f32,
};
pub const DEEP_PINK: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x14 as f32 / 255.,
    b: 0x93 as f32 / 255.,
    a: 1.0f32,
};
pub const PALE_VIOLET_RED: RGBA<f32> = RGBA {
    r: 0xDB as f32 / 255.,
    g: 0x70 as f32 / 255.,
    b: 0x93 as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_VIOLET_RED: RGBA<f32> = RGBA {
    r: 0xC7 as f32 / 255.,
    g: 0x15 as f32 / 255.,
    b: 0x85 as f32 / 255.,
    a: 1.0f32,
};

pub const LIGHT_SALMON: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xA0 as f32 / 255.,
    b: 0x7A as f32 / 255.,
    a: 1.0f32,
};
pub const SALMON: RGBA<f32> = RGBA {
    r: 0xFA as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x72 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_SALMON: RGBA<f32> = RGBA {
    r: 0xE9 as f32 / 255.,
    g: 0x96 as f32 / 255.,
    b: 0x7A as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_CORAL: RGBA<f32> = RGBA {
    r: 0xF0 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x80 as f32 / 255.,
    a: 1.0f32,
};
pub const INDIAN_RED: RGBA<f32> = RGBA {
    r: 0xCD as f32 / 255.,
    g: 0x5C as f32 / 255.,
    b: 0x5C as f32 / 255.,
    a: 1.0f32,
};
pub const CRIMSON: RGBA<f32> = RGBA {
    r: 0xDC as f32 / 255.,
    g: 0x14 as f32 / 255.,
    b: 0x3C as f32 / 255.,
    a: 1.0f32,
};
pub const FIREBRICK: RGBA<f32> = RGBA {
    r: 0xB2 as f32 / 255.,
    g: 0x22 as f32 / 255.,
    b: 0x22 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_RED: RGBA<f32> = RGBA {
    r: 0x8B as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};

pub const ORANGE_RED: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x45 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const TOMATO: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x63 as f32 / 255.,
    b: 0x47 as f32 / 255.,
    a: 1.0f32,
};
pub const CORAL: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x7f as f32 / 255.,
    b: 0x50 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_ORANGE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x8C as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const ORANGE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xA5 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};

pub const LIGHT_YELLOW: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xE0 as f32 / 255.,
    a: 1.0f32,
};
pub const LEMON_CHIFFON: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFA as f32 / 255.,
    b: 0xCD as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_GOLDENROD_YELLOW: RGBA<f32> = RGBA {
    r: 0xFA as f32 / 255.,
    g: 0xFA as f32 / 255.,
    b: 0xD2 as f32 / 255.,
    a: 1.0f32,
};
pub const PAPAYA_WHIP: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xEF as f32 / 255.,
    b: 0xD5 as f32 / 255.,
    a: 1.0f32,
};
pub const MOCCASIN: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xE4 as f32 / 255.,
    b: 0xB5 as f32 / 255.,
    a: 1.0f32,
};
pub const PEACH_PUFF: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xDA as f32 / 255.,
    b: 0xB9 as f32 / 255.,
    a: 1.0f32,
};
pub const PALE_GOLDENROD: RGBA<f32> = RGBA {
    r: 0xEE as f32 / 255.,
    g: 0xE8 as f32 / 255.,
    b: 0xAA as f32 / 255.,
    a: 1.0f32,
};
pub const KHAKI: RGBA<f32> = RGBA {
    r: 0xF0 as f32 / 255.,
    g: 0xE6 as f32 / 255.,
    b: 0x8C as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_KHAKI: RGBA<f32> = RGBA {
    r: 0xBD as f32 / 255.,
    g: 0xB7 as f32 / 255.,
    b: 0x6B as f32 / 255.,
    a: 1.0f32,
};
pub const GOLD: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xD7 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};

pub const CORNSILK: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xF8 as f32 / 255.,
    b: 0xDC as f32 / 255.,
    a: 1.0f32,
};
pub const BLANCHED_ALMOND: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xEB as f32 / 255.,
    b: 0xCD as f32 / 255.,
    a: 1.0f32,
};
pub const BISQUE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xE4 as f32 / 255.,
    b: 0xC4 as f32 / 255.,
    a: 1.0f32,
};
pub const NAVAJO_WHITE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xDE as f32 / 255.,
    b: 0xAD as f32 / 255.,
    a: 1.0f32,
};
pub const WHEAT: RGBA<f32> = RGBA {
    r: 0xF5 as f32 / 255.,
    g: 0xDE as f32 / 255.,
    b: 0xB3 as f32 / 255.,
    a: 1.0f32,
};
pub const BURLYWOOD: RGBA<f32> = RGBA {
    r: 0xDE as f32 / 255.,
    g: 0xB8 as f32 / 255.,
    b: 0x87 as f32 / 255.,
    a: 1.0f32,
};
pub const TAN: RGBA<f32> = RGBA {
    r: 0xD2 as f32 / 255.,
    g: 0xB4 as f32 / 255.,
    b: 0x8C as f32 / 255.,
    a: 1.0f32,
};
pub const ROSY_BROWN: RGBA<f32> = RGBA {
    r: 0xBC as f32 / 255.,
    g: 0x8F as f32 / 255.,
    b: 0x8F as f32 / 255.,
    a: 1.0f32,
};
pub const SANDY_BROWN: RGBA<f32> = RGBA {
    r: 0xF4 as f32 / 255.,
    g: 0xA4 as f32 / 255.,
    b: 0x60 as f32 / 255.,
    a: 1.0f32,
};
pub const GOLDENROD: RGBA<f32> = RGBA {
    r: 0xDA as f32 / 255.,
    g: 0xA5 as f32 / 255.,
    b: 0x20 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_GOLDENROD: RGBA<f32> = RGBA {
    r: 0xB8 as f32 / 255.,
    g: 0x86 as f32 / 255.,
    b: 0x0B as f32 / 255.,
    a: 1.0f32,
};
pub const PERU: RGBA<f32> = RGBA {
    r: 0xCD as f32 / 255.,
    g: 0x85 as f32 / 255.,
    b: 0x3F as f32 / 255.,
    a: 1.0f32,
};
pub const CHOCOLATE: RGBA<f32> = RGBA {
    r: 0xD2 as f32 / 255.,
    g: 0x69 as f32 / 255.,
    b: 0x1E as f32 / 255.,
    a: 1.0f32,
};
pub const SADDLE_BROWN: RGBA<f32> = RGBA {
    r: 0x8B as f32 / 255.,
    g: 0x45 as f32 / 255.,
    b: 0x13 as f32 / 255.,
    a: 1.0f32,
};
pub const SIENNA: RGBA<f32> = RGBA {
    r: 0xA0 as f32 / 255.,
    g: 0x52 as f32 / 255.,
    b: 0x2D as f32 / 255.,
    a: 1.0f32,
};
pub const BROWN: RGBA<f32> = RGBA {
    r: 0xA5 as f32 / 255.,
    g: 0x2A as f32 / 255.,
    b: 0x2A as f32 / 255.,
    a: 1.0f32,
};

pub const DARK_OLIVE_GREEN: RGBA<f32> = RGBA {
    r: 0x55 as f32 / 255.,
    g: 0x6B as f32 / 255.,
    b: 0x2F as f32 / 255.,
    a: 1.0f32,
};
pub const OLIVE_DRAB: RGBA<f32> = RGBA {
    r: 0x6B as f32 / 255.,
    g: 0x8E as f32 / 255.,
    b: 0x23 as f32 / 255.,
    a: 1.0f32,
};
pub const YELLOW_GREEN: RGBA<f32> = RGBA {
    r: 0x9A as f32 / 255.,
    g: 0xCD as f32 / 255.,
    b: 0x32 as f32 / 255.,
    a: 1.0f32,
};
pub const LIME_GREEN: RGBA<f32> = RGBA {
    r: 0x32 as f32 / 255.,
    g: 0xCD as f32 / 255.,
    b: 0x32 as f32 / 255.,
    a: 1.0f32,
};
pub const LAWN_GREEN: RGBA<f32> = RGBA {
    r: 0x7C as f32 / 255.,
    g: 0xFC as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const CHARTREUSE: RGBA<f32> = RGBA {
    r: 0x7F as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};
pub const GREEN_YELLOW: RGBA<f32> = RGBA {
    r: 0xAD as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0x2F as f32 / 255.,
    a: 1.0f32,
};
pub const SPRING_GREEN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0x7F as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_SPRING_GREEN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xFA as f32 / 255.,
    b: 0x9A as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_GREEN: RGBA<f32> = RGBA {
    r: 0x90 as f32 / 255.,
    g: 0xEE as f32 / 255.,
    b: 0x90 as f32 / 255.,
    a: 1.0f32,
};
pub const PALE_GREEN: RGBA<f32> = RGBA {
    r: 0x98 as f32 / 255.,
    g: 0xFB as f32 / 255.,
    b: 0x98 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_SEA_GREEN: RGBA<f32> = RGBA {
    r: 0x8F as f32 / 255.,
    g: 0xBC as f32 / 255.,
    b: 0x8F as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_AQUAMARINE: RGBA<f32> = RGBA {
    r: 0x66 as f32 / 255.,
    g: 0xCD as f32 / 255.,
    b: 0xAA as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_SEA_GREEN: RGBA<f32> = RGBA {
    r: 0x3C as f32 / 255.,
    g: 0xB3 as f32 / 255.,
    b: 0x71 as f32 / 255.,
    a: 1.0f32,
};
pub const SEA_GREEN: RGBA<f32> = RGBA {
    r: 0x2E as f32 / 255.,
    g: 0x8B as f32 / 255.,
    b: 0x57 as f32 / 255.,
    a: 1.0f32,
};
pub const FOREST_GREEN: RGBA<f32> = RGBA {
    r: 0x22 as f32 / 255.,
    g: 0x8B as f32 / 255.,
    b: 0x22 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_GREEN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x64 as f32 / 255.,
    b: 0x00 as f32 / 255.,
    a: 1.0f32,
};

pub const CYAN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_CYAN: RGBA<f32> = RGBA {
    r: 0xE0 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const PALE_TURQUOISE: RGBA<f32> = RGBA {
    r: 0xAF as f32 / 255.,
    g: 0xEE as f32 / 255.,
    b: 0xEE as f32 / 255.,
    a: 1.0f32,
};
pub const AQUAMARINE: RGBA<f32> = RGBA {
    r: 0x7F as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xD4 as f32 / 255.,
    a: 1.0f32,
};
pub const TURQUOISE: RGBA<f32> = RGBA {
    r: 0x40 as f32 / 255.,
    g: 0xE0 as f32 / 255.,
    b: 0xD0 as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_TURQUOISE: RGBA<f32> = RGBA {
    r: 0x48 as f32 / 255.,
    g: 0xD1 as f32 / 255.,
    b: 0xCC as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_TURQUOISE: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xCE as f32 / 255.,
    b: 0xD1 as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_SEA_GREEN: RGBA<f32> = RGBA {
    r: 0x20 as f32 / 255.,
    g: 0xB2 as f32 / 255.,
    b: 0xAA as f32 / 255.,
    a: 1.0f32,
};
pub const CADET_BLUE: RGBA<f32> = RGBA {
    r: 0x5F as f32 / 255.,
    g: 0x9E as f32 / 255.,
    b: 0xA0 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_CYAN: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x8B as f32 / 255.,
    b: 0x8B as f32 / 255.,
    a: 1.0f32,
};

pub const LIGHT_STEEL_BLUE: RGBA<f32> = RGBA {
    r: 0xB0 as f32 / 255.,
    g: 0xC4 as f32 / 255.,
    b: 0xDE as f32 / 255.,
    a: 1.0f32,
};
pub const POWDER_BLUE: RGBA<f32> = RGBA {
    r: 0xB0 as f32 / 255.,
    g: 0xE0 as f32 / 255.,
    b: 0xE6 as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_BLUE: RGBA<f32> = RGBA {
    r: 0xAD as f32 / 255.,
    g: 0xD8 as f32 / 255.,
    b: 0xE6 as f32 / 255.,
    a: 1.0f32,
};
pub const SKY_BLUE: RGBA<f32> = RGBA {
    r: 0x87 as f32 / 255.,
    g: 0xCE as f32 / 255.,
    b: 0xEB as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_SKY_BLUE: RGBA<f32> = RGBA {
    r: 0x87 as f32 / 255.,
    g: 0xCE as f32 / 255.,
    b: 0xFA as f32 / 255.,
    a: 1.0f32,
};
pub const DEEP_SKY_BLUE: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0xBF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const DODGER_BLUE: RGBA<f32> = RGBA {
    r: 0x1E as f32 / 255.,
    g: 0x90 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const CORNFLOWER_BLUE: RGBA<f32> = RGBA {
    r: 0x64 as f32 / 255.,
    g: 0x95 as f32 / 255.,
    b: 0xED as f32 / 255.,
    a: 1.0f32,
};
pub const STEEL_BLUE: RGBA<f32> = RGBA {
    r: 0x46 as f32 / 255.,
    g: 0x82 as f32 / 255.,
    b: 0xB4 as f32 / 255.,
    a: 1.0f32,
};
pub const ROYAL_BLUE: RGBA<f32> = RGBA {
    r: 0x41 as f32 / 255.,
    g: 0x69 as f32 / 255.,
    b: 0xE1 as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_BLUE: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0xCD as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_BLUE: RGBA<f32> = RGBA {
    r: 0x00 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x8B as f32 / 255.,
    a: 1.0f32,
};
pub const MIDNIGHT_BLUE: RGBA<f32> = RGBA {
    r: 0x19 as f32 / 255.,
    g: 0x19 as f32 / 255.,
    b: 0x70 as f32 / 255.,
    a: 1.0f32,
};

pub const LAVENDER: RGBA<f32> = RGBA {
    r: 0xE6 as f32 / 255.,
    g: 0xE6 as f32 / 255.,
    b: 0xFA as f32 / 255.,
    a: 1.0f32,
};
pub const THISTLE: RGBA<f32> = RGBA {
    r: 0xD8 as f32 / 255.,
    g: 0xBF as f32 / 255.,
    b: 0xD8 as f32 / 255.,
    a: 1.0f32,
};
pub const PLUM: RGBA<f32> = RGBA {
    r: 0xDD as f32 / 255.,
    g: 0xA0 as f32 / 255.,
    b: 0xDD as f32 / 255.,
    a: 1.0f32,
};
pub const VIOLET: RGBA<f32> = RGBA {
    r: 0xEE as f32 / 255.,
    g: 0x82 as f32 / 255.,
    b: 0xEE as f32 / 255.,
    a: 1.0f32,
};
pub const ORCHID: RGBA<f32> = RGBA {
    r: 0xDA as f32 / 255.,
    g: 0x70 as f32 / 255.,
    b: 0xD6 as f32 / 255.,
    a: 1.0f32,
};
pub const MAGENTA: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_ORCHID: RGBA<f32> = RGBA {
    r: 0xBA as f32 / 255.,
    g: 0x55 as f32 / 255.,
    b: 0xD3 as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_PURPLE: RGBA<f32> = RGBA {
    r: 0x93 as f32 / 255.,
    g: 0x70 as f32 / 255.,
    b: 0xDB as f32 / 255.,
    a: 1.0f32,
};
pub const BLUE_VIOLET: RGBA<f32> = RGBA {
    r: 0x8A as f32 / 255.,
    g: 0x2B as f32 / 255.,
    b: 0xE2 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_VIOLET: RGBA<f32> = RGBA {
    r: 0x94 as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0xD3 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_ORCHID: RGBA<f32> = RGBA {
    r: 0x99 as f32 / 255.,
    g: 0x32 as f32 / 255.,
    b: 0xCC as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_MAGENTA: RGBA<f32> = RGBA {
    r: 0x8B as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x8B as f32 / 255.,
    a: 1.0f32,
};
pub const INDIGO: RGBA<f32> = RGBA {
    r: 0x4B as f32 / 255.,
    g: 0x00 as f32 / 255.,
    b: 0x82 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_SLATE_BLUE: RGBA<f32> = RGBA {
    r: 0x4B as f32 / 255.,
    g: 0x3D as f32 / 255.,
    b: 0x8B as f32 / 255.,
    a: 1.0f32,
};
pub const SLATE_BLUE: RGBA<f32> = RGBA {
    r: 0x6A as f32 / 255.,
    g: 0x5A as f32 / 255.,
    b: 0xCD as f32 / 255.,
    a: 1.0f32,
};
pub const MEDIUM_SLATE_BLUE: RGBA<f32> = RGBA {
    r: 0x7B as f32 / 255.,
    g: 0x68 as f32 / 255.,
    b: 0xEE as f32 / 255.,
    a: 1.0f32,
};

pub const SNOW: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFA as f32 / 255.,
    b: 0xFA as f32 / 255.,
    a: 1.0f32,
};
pub const HONEYDEW: RGBA<f32> = RGBA {
    r: 0xF0 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xF0 as f32 / 255.,
    a: 1.0f32,
};
pub const MINT_CREAM: RGBA<f32> = RGBA {
    r: 0xF5 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFA as f32 / 255.,
    a: 1.0f32,
};
pub const AZURE: RGBA<f32> = RGBA {
    r: 0xF0 as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const ALICE_BLUE: RGBA<f32> = RGBA {
    r: 0xF0 as f32 / 255.,
    g: 0xF8 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const GHOST_WHITE: RGBA<f32> = RGBA {
    r: 0xF8 as f32 / 255.,
    g: 0xF8 as f32 / 255.,
    b: 0xFF as f32 / 255.,
    a: 1.0f32,
};
pub const WHITE_SMOKE: RGBA<f32> = RGBA {
    r: 0xF5 as f32 / 255.,
    g: 0xF5 as f32 / 255.,
    b: 0xF5 as f32 / 255.,
    a: 1.0f32,
};
pub const SEASHELL: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xF5 as f32 / 255.,
    b: 0xEE as f32 / 255.,
    a: 1.0f32,
};
pub const BEIGE: RGBA<f32> = RGBA {
    r: 0xF5 as f32 / 255.,
    g: 0xF5 as f32 / 255.,
    b: 0xDC as f32 / 255.,
    a: 1.0f32,
};
pub const OLD_LACE: RGBA<f32> = RGBA {
    r: 0xFD as f32 / 255.,
    g: 0xF5 as f32 / 255.,
    b: 0xE6 as f32 / 255.,
    a: 1.0f32,
};
pub const FLORAL_WHITE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFA as f32 / 255.,
    b: 0xF0 as f32 / 255.,
    a: 1.0f32,
};
pub const IVORY: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xFF as f32 / 255.,
    b: 0xF0 as f32 / 255.,
    a: 1.0f32,
};
pub const ANTINQUE_WHITE: RGBA<f32> = RGBA {
    r: 0xFA as f32 / 255.,
    g: 0xEB as f32 / 255.,
    b: 0xD7 as f32 / 255.,
    a: 1.0f32,
};
pub const LINEN: RGBA<f32> = RGBA {
    r: 0xFA as f32 / 255.,
    g: 0xF0 as f32 / 255.,
    b: 0xE6 as f32 / 255.,
    a: 1.0f32,
};
pub const LAVENDER_BLUSH: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xF0 as f32 / 255.,
    b: 0xF5 as f32 / 255.,
    a: 1.0f32,
};
pub const MISTY_ROSE: RGBA<f32> = RGBA {
    r: 0xFF as f32 / 255.,
    g: 0xE4 as f32 / 255.,
    b: 0xE1 as f32 / 255.,
    a: 1.0f32,
};

pub const GAINSBORO: RGBA<f32> = RGBA {
    r: 0xDC as f32 / 255.,
    g: 0xDC as f32 / 255.,
    b: 0xDC as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_GRAY: RGBA<f32> = RGBA {
    r: 0xD3 as f32 / 255.,
    g: 0xD3 as f32 / 255.,
    b: 0xD3 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_GRAY: RGBA<f32> = RGBA {
    r: 0xA9 as f32 / 255.,
    g: 0xA9 as f32 / 255.,
    b: 0xA9 as f32 / 255.,
    a: 1.0f32,
};
pub const DIM_GRAY: RGBA<f32> = RGBA {
    r: 0x69 as f32 / 255.,
    g: 0x69 as f32 / 255.,
    b: 0x69 as f32 / 255.,
    a: 1.0f32,
};
pub const LIGHT_SLATE_GRAY: RGBA<f32> = RGBA {
    r: 0x77 as f32 / 255.,
    g: 0x88 as f32 / 255.,
    b: 0x99 as f32 / 255.,
    a: 1.0f32,
};
pub const SLATE_GRAY: RGBA<f32> = RGBA {
    r: 0x70 as f32 / 255.,
    g: 0x80 as f32 / 255.,
    b: 0x90 as f32 / 255.,
    a: 1.0f32,
};
pub const DARK_SLATE_GRAY: RGBA<f32> = RGBA {
    r: 0x2F as f32 / 255.,
    g: 0x4F as f32 / 255.,
    b: 0x4F as f32 / 255.,
    a: 1.0f32,
};
