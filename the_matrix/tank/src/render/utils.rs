

use bevy::prelude::*;
use wgpu::*;

use crate::render::{
    PreferredSurfaceFormat,
};

enum Layout {
    Entry(BindGroupLayoutEntry),
    Entries(BindGroupLayout),
}

#[derive(Setters, Default)]
pub struct PipelineBuilder {
    topology: PrimitiveTopology,
    #[setters(strip_option)]
    strip_index_format: Option<IndexFormat>,
    #[setters(skip)]
    materials: Vec<Layout>,
    #[setters(strip_option)]
    format: Option<TextureFormat>,
    #[setters(skip)]
    vert_buffer_layouts: Vec<VertBufferLayout>,
    #[setters(strip_option)]
    blend: Option<BlendState>,
}

impl PipelineBuilder {
    pub fn vert_buffer_layout(mut self, layout: VertBufferLayout) -> Self {
        self.vert_buffer_layouts.push(layout);
        self
    }

    pub fn material_group(mut self, layout: BindGroupLayout) -> Self {
        self.materials.push(Layout::Entries(layout));
        self
    }

    pub fn material(mut self, layout: BindGroupLayoutEntry) -> Self {
        self.materials.push(Layout::Entry(layout));
        self
    }

    pub fn build_with(
        mut self,
        vert: &ShaderModule,
        frag: &ShaderModule,
        device: &Device,
        format: &PreferredSurfaceFormat,
    ) -> RenderPipeline {
        let format = match self.format {
            None => format.0,
            Some(f) => f,
        };
        let bind_groups: Vec<_> = self
            .materials
            .into_iter()
            .map(|x| match x {
                Layout::Entry(x) => device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[x],
                }),
                Layout::Entries(x) => x,
            })
            .collect();
        let layouts: Vec<&BindGroupLayout> = bind_groups.iter().collect();
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &layouts[..],
            push_constant_ranges: &[],
        });
        let vertex_buffer_layout: Vec<_> = self
            .vert_buffer_layouts
            .iter_mut()
            .enumerate()
            .map(|(offset, x)| {
                let stride = if x.stride == 0 {
                    x.attributes.iter().map(|x| x.format.size()).sum()
                } else {
                    x.stride
                };
                x.attributes
                    .iter_mut()
                    .for_each(|x| x.shader_location += offset as ShaderLocation);
                VertexBufferLayout {
                    array_stride: stride,
                    step_mode: x.step_mode,
                    attributes: &x.attributes,
                }
            })
            .collect();
        debug!("{:#?}", vertex_buffer_layout);
        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: vert,
                entry_point: "main",
                buffers: &vertex_buffer_layout,
            },
            primitive: PrimitiveState {
                topology: self.topology,
                strip_index_format: self.strip_index_format,
                front_face: Default::default(),
                cull_mode: None,
                clamp_depth: false,
                polygon_mode: Default::default(),
                conservative: false,
            },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: frag,
                entry_point: "main",
                targets: &[ColorTargetState {
                    format,
                    blend: self.blend,
                    write_mask: Default::default(),
                }],
            }),
        })
    }
}

#[derive(Setters)]
pub struct VertBufferLayout {
    stride: u64,
    step_mode: VertexStepMode,
    #[setters(skip)]
    attributes: Vec<VertexAttribute>,
}

impl VertBufferLayout {
    pub fn new() -> Self {
        Self {
            stride: 0,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![],
        }
    }

    pub fn attribute(self, format: VertexFormat) -> Self {
        let offset: BufferAddress = self.attributes.iter().map(|x| x.format.size()).sum();
        self.with_attribute_offset(format, offset as usize)
    }

    pub fn with_attribute_offset(mut self, format: VertexFormat, offset: usize) -> Self {
        self.attributes.push(VertexAttribute {
            format,
            offset: offset as _,
            shader_location: self.attributes.len() as _,
        });
        self
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct GroupMaterial {
    scale: Vec2,
    pub translate: Vec2,
    prev_scale: Vec2,
}

impl GroupMaterial {
    pub fn new(scale: Vec2, translate: Vec2) -> Self {
        Self {
            scale,
            translate,
            prev_scale: scale,
        }
    }
    pub fn scale(&self) -> Vec2 {
        self.scale
    }

    pub fn prev_scale(&self) -> Vec2 {
        self.prev_scale
    }

    pub fn set_scale(&mut self, scale: Vec2) {
        self.prev_scale = self.scale;
        self.scale = scale;
    }

    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        dbg!((screen - self.translate) / self.scale)
    }
}

impl Default for GroupMaterial {
    fn default() -> Self {
        GroupMaterial::new(Vec2::ONE, Vec2::ZERO)
    }
}
