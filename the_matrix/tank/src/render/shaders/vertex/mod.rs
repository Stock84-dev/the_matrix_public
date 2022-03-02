use std::borrow::Cow;


use mouse::minipre::PreprocessorContext;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

use crate::render::shaders::StaticShader;

pub struct LineVert;

impl StaticShader for LineVert {
    fn source() -> &'static str {
        include_str!("line.wgsl")
    }
}

pub struct ParallelCoordinatesVert(pub ShaderModule);

impl ParallelCoordinatesVert {
    pub fn new(context: &mut PreprocessorContext, device: &Device) -> Self {
        let source = include_str!("parallel_coordinates.wgsl");
        let source = mouse::minipre::process_str(&source, context).unwrap();
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(Cow::Owned(source)),
        });
        Self(shader)
    }
}
