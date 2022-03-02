




use crate::render::shaders::StaticShader;

pub struct ColorForwardFrag;

impl StaticShader for ColorForwardFrag {
    fn source() -> &'static str {
        include_str!("color_forward.wgsl")
    }
}

pub struct SampledFrag;

impl StaticShader for SampledFrag {
    fn source() -> &'static str {
        include_str!("sampled.wgsl")
    }
}

