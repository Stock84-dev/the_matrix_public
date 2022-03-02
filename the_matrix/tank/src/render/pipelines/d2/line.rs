





use bevy::prelude::*;
use bytemuck::Pod;


use rgb::RGBA;

use wgpu::{
    Device, VertexFormat, VertexStepMode,
};

use crate::flex::{Flex};
use crate::niobe::{View, ZComponent};
use crate::render::pipelines::d2::line_strip::{LineStripPipeline, SampledLineStripPipeline};
use crate::render::shaders::fragment::{ColorForwardFrag, SampledFrag};
use crate::render::shaders::vertex::LineVert;
use crate::render::shaders::{AppShaderExt, Shader};
use crate::render::utils::{GroupMaterial, PipelineBuilder, VertBufferLayout};
use crate::render::{
    add_material_systems, pixel_length_to_screen_space,
    screen_length_to_pixel_space, BufferId, Buffers, GpuMesh,
    Material, MaterialLayout, MeshClip, MyBuffer, PreferredSurfaceFormat,
    PrevRenderTargetSize, RedrawRenderTarget, RenderCommands,
    RenderPipelines, RenderPlugin, SampledMaterial, ScissorRect,
};
use crate::{
    find_root_parent, LineMaterialInPixelSpace, RenderTargetSize,
};

pub struct LinePlugin;

impl Plug for LinePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(LineCorePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.startup_wgpu_pipeline_system(startup_line::<LinePipeline>)
            .startup_wgpu_pipeline_system(startup_sampled_line::<SampledLinePipeline>)
            .render_system(line_system::<LinePipeline, LineMaterial>)
            .render_system(line_system::<SampledLinePipeline, SampledLineMaterial>)
    }
}

pub(super) struct LineCorePlugin;

impl Plug for LineCorePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(RenderPlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        add_material_systems::<LineMaterialData>(app);
        app.init_shader(LineVert)
            .init_shader(ColorForwardFrag)
            .init_shader(SampledFrag)
            .startup_wgpu_system(startup_line_segment_buffer)
            .post_system(rescale_pixel_space_for_line_material.label(RescalePixelSpaceMeshLabel))
            .post_system(line_material_pixel_to_screen_space.after(RescalePixelSpaceMeshLabel))
    }
}

pub struct LinePipeline;

pub(super) fn startup_line<PipelineKind: 'static>(
    device: Res<Device>,
    vert: Res<Shader<LineVert>>,
    frag: Res<Shader<ColorForwardFrag>>,
    format: Res<PreferredSurfaceFormat>,
    mut pipelines: ResMut<RenderPipelines>,
) {
    let mut layout = VertBufferLayout::new()
        .step_mode(VertexStepMode::Instance)
        .attribute(VertexFormat::Float32x2)
        .attribute(VertexFormat::Float32x2);
    if PipelineKind::id() == LineStripPipeline::id() {
        layout = layout.stride(VertexFormat::Float32x2.size());
    }
    let pipeline = PipelineBuilder::default()
        .vert_buffer_layout(VertBufferLayout::new().attribute(VertexFormat::Float32x2))
        .vert_buffer_layout(layout)
        .material(GroupMaterial::uninitialized().aligned_layout())
        .material(LineMaterialData::uninitialized().aligned_layout())
        .build_with(&*vert, &*frag, &device, &format);
    pipelines.add_static::<PipelineKind>(pipeline);
}

pub struct SampledLinePipeline;

pub(super) fn startup_sampled_line<PipelineKind: 'static>(
    device: Res<Device>,
    vert: Res<Shader<LineVert>>,
    frag: Res<Shader<SampledFrag>>,
    format: Res<PreferredSurfaceFormat>,
    mut pipelines: ResMut<RenderPipelines>,
) {
    let mut layout = VertBufferLayout::new()
        .step_mode(VertexStepMode::Instance)
        .attribute(VertexFormat::Float32x2)
        .attribute(VertexFormat::Float32x2);
    if PipelineKind::id() == SampledLineStripPipeline::id() {
        layout = layout.stride(VertexFormat::Float32x2.size());
    }
    let pipeline = PipelineBuilder::default()
        .vert_buffer_layout(VertBufferLayout::new().attribute(VertexFormat::Float32x2))
        .vert_buffer_layout(layout)
        .material(GroupMaterial::uninitialized().aligned_layout())
        .material(LineMaterialData::uninitialized().aligned_layout())
        .material_group(SampledMaterial::layout(&device))
        .build_with(&*vert, &*frag, &device, &format);
    pipelines.add_static::<PipelineKind>(pipeline);
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct LineMaterialData {
    pub color: RGBA<f32>,
    /// Pixel width from middle, 0.5 means that line will have a width of 1 pixel
    pub width: Vec2,
}

impl MaterialLayout for LineMaterialData {}

#[derive(Component)]
pub struct LineMaterial;
#[derive(Component)]
pub struct SampledLineMaterial;

pub(super) fn line_system<PipelineKind: 'static, MaterialKind: Component>(
    flex: Flex,
    mut commands: ResMut<RenderCommands>,
    mut reader: UniqueEventReader<RedrawRenderTarget>,
    render_targets: Query<&Children>,
    buffers: Res<Buffers>,
    views: Query<(Option<&ScissorRect>, &Children), With<View>>,
    groups: Query<(&Material<GroupMaterial>, &Children)>,
    line_segment: Res<LineSegmentBuffer>,
    materials: Query<
        (
            &Material<LineMaterialData>,
            &GpuMesh,
            Option<&ZComponent>,
            Option<&MeshClip>,
            Option<&SampledMaterial>,
        ),
        With<MaterialKind>,
    >,
) {
    for e in reader.iter() {
        let children = ok_loop!(render_targets.get(e.id));
        let mut rt_drawer = commands.render_target(e.id);
        for child in children.iter() {
            let (scissor, children) = ok_loop!(views.get(*child));
            let mut scissor_rect = Default::default();
            if let Some(layout) = flex.layout(*child) {
                trace!("setting scissor {:?}", layout);
                scissor_rect = ScissorRect {
                    x: layout.location.x as u32,
                    y: layout.location.y as u32,
                    w: layout.size.x as u32,
                    h: layout.size.y as u32,
                };
            }
            if let Some(s) = scissor {
                if *s != ScissorRect::default() {
                    scissor_rect = s.clone();
                }
            }
            //            debug!("set scissor {:#?}", scissor_rect);
            for child in children.iter() {
                let (group, children): (&Material<GroupMaterial>, &Children) =
                    ok_loop!(groups.get(*child));
                for child in children.iter() {
                    let (material, mesh, z, clip, sampled_material): (
                        &Material<LineMaterialData>,
                        &GpuMesh,
                        Option<&ZComponent>,
                        Option<&MeshClip>,
                        Option<&SampledMaterial>,
                    ) = ok_loop!(materials.get(*child));
                    let builder = rt_drawer.drawer::<PipelineKind>(z, scissor_rect);
                    if PipelineKind::id() == SampledLinePipeline::id()
                        || PipelineKind::id() == SampledLineStripPipeline::id()
                    {
                        if let Some(sampled_material) = sampled_material {
                            builder.set_bind_group(2, sampled_material.id);
                        } else {
                            continue;
                        }
                    }
                    let buffer = buffers.get(mesh.buffer_id);
                    let buffer_size = buffer.size();
                    builder.set_bind_group(0, group.id);
                    builder.set_bind_group(1, material.id);

                    builder.set_vertex_buffer(0, line_segment.0, None);
                    builder.set_vertex_buffer(1, mesh.buffer_id, None);
                    if let Some(clip) = clip {
                        builder.draw(0..6, clip.visible_range.clone());
                    } else {
                        if PipelineKind::id() == LinePipeline::id()
                            || PipelineKind::id() == SampledLinePipeline::id()
                        {
                            //                            trace!("line system");
                            builder.draw(0..6, 0..buffer_size / (Vec2::size() * 2));
                        } else {
                            trace!("line strip system {:?}", *child);
                            // mesh component could be spawned but not uploaded
                            if buffer_size > 1 {
                                builder.draw(0..6, 0..buffer_size / (Vec2::size()) - 1);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn startup_line_segment_buffer(
    device: Res<Device>,
    mut buffers: ResMut<Buffers>,
    mut commands: Commands,
) {
    const SEGMENT: [[f32; 2]; 6] = [
        [0.0f32, -0.5],
        [1., -0.5],
        [1., 0.5],
        [0., -0.5],
        [1., 0.5],
        [0., 0.5],
    ];
    commands.insert_resource(LineSegmentBuffer(
        buffers.push_buffer(MyBuffer::new(&SEGMENT, &*device)),
    ));
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
struct RescalePixelSpaceMeshLabel;

fn rescale_pixel_space_for_line_material(
    changed_render_targets: Query<
        (&RenderTargetSize, &PrevRenderTargetSize),
        Changed<RenderTargetSize>,
    >,
    parents: Query<&Parent>,
    mut materials: Query<(
        &mut Material<LineMaterialData>,
        &mut LineMaterialInPixelSpace,
        &Parent,
    )>,
) {
    for (mut material, mut ignore, parent) in materials.iter_mut().filter(|x| !x.0.is_changed()) {
        let root = find_root_parent(&parents, **parent);
        let (size, prev_size) = match changed_render_targets.get(root) {
            Ok(x) => x,
            Err(_) => continue,
        };
        ignore.0 = true;
        material.data.width = pixel_length_to_screen_space(
            screen_length_to_pixel_space(material.data.width, prev_size.0),
            size.0,
        );
    }
}

fn line_material_pixel_to_screen_space(
    parents: Query<&Parent>,
    mut changed_materials: Query<
        (
            &mut Material<LineMaterialData>,
            &mut LineMaterialInPixelSpace,
            &Parent,
        ),
        Changed<Material<LineMaterialData>>,
    >,
    render_targets_sizes: Query<&RenderTargetSize>,
) {
    for (mut material, mut ignore, parent) in changed_materials.iter_mut() {
        if ignore.0 {
            ignore.0 = false;
            continue;
        }
        let root = find_root_parent(&parents, **parent);
        let size = render_targets_sizes.get(root).unwrap();
        material.data.width = pixel_length_to_screen_space(material.data.width, size.0);
    }
}

pub(super) struct LineSegmentBuffer(BufferId);
