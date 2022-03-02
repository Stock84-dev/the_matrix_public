use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};
use std::convert::{TryFrom};

use std::num::NonZeroU64;
use std::ops::{Range};


use std::time::Instant;

use arrayvec::ArrayVec;
use bevy::prelude::*;
use bytemuck::Pod;


use imgui_wgpu::RendererConfig;
use ordered_float::OrderedFloat;

use wgpu::util::{BufferInitDescriptor, DeviceExt, StagingBelt};
use wgpu::{
    Adapter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindingResource, Buffer,
    BufferAddress, BufferBinding, BufferDescriptor, BufferUsages, CommandEncoder, CommandEncoderDescriptor, Device, IndexFormat,
    Instance, LoadOp, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor, ShaderModule, Surface, TextureFormat, TextureView,
    TextureViewDescriptor,
};

use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder, GlyphCruncher};



// use crate::app::AppExt;
use crate::imgui_plugin::{ImguiContext, ImguiDrawDatas, SharedFontAtlas};
use crate::niobe::{Text, ZComponent};
use crate::render::{
    create_surface_configuration, AsBytes, GpuMesh, Material,
    MaterialLayout, Mesh, ParentRenderTarget, PreferredSurfaceFormat,
    RedrawRenderTarget, RedrawWindowOnly, RenderKey, RenderPipelines, RenderSurface, RenderTarget,
    RenderTargetBundle, RenderTextureId, ScissorRect, TextBatch,
};
use crate::winit_plugin::{
    RedrawWindow, Window, WindowClosing, WindowCreated,
};
use crate::{
    InitStages, RenderTargetSize, ResourceInsertSafe, Stages,
};

pub fn build(app: &mut App) {
    app.init_resource::<GlyphBrushes>()
        .add_event::<RedrawWindowOnly>()
        .add_startup_system_to_stage(InitStages::Window, startup_renderer)
        .add_startup_system_to_stage(StartupStage::PreStartup, startup_ui_renderer)
        .add_system_to_stage(Stages::PrePreUpdate, create_render_target_for_window)
        .add_system_to_stage(CoreStage::PostUpdate, remove_render_target_for_window)
        .add_system_to_stage(Stages::Draw, draw);
}

fn process_commands(
    view: &TextureView,
    render_target: &RenderTarget,
    render_target_size: &RenderTargetSize,
    render_target_id: Entity,
    buffers: &Buffers,
    arena: &BindGroupArena,
    device: &Device,
    sections: &Query<&Text>,
    batch_sections: &Query<&TextBatch>,
    pipelines: &RenderPipelines,
    encoder: &mut CommandEncoder,
    staging_belt: &mut StagingBelt,
    glyph_brushes: &mut GlyphBrushes,
    commands: &mut RenderCommands,
) {
    trace!("process commands");
    // TODO: might not need a hashmap since we use default format for everything
    let format = render_target.format;
    let z_map = commands.commands.get_mut(&render_target_id).unwrap();
    let glyph_brush = if let Some(brush) = glyph_brushes.get_mut(&format) {
        brush
    } else {
        glyph_brushes.insert(
            format,
            GlyphBrushBuilder::using_font(
                ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf"))
                    .unwrap(),
            )
            .build(&device, format),
        );
        glyph_brushes.get_mut(&format).unwrap()
    };
    let mut cleared = false;
    for (_, command_map) in z_map {
//        let pipeline_id = command_map.iter().find_map(|(k, v)| {
////            if !v.render_commands.is_empty() && pipelines.pipelines.get(&k.pipeline).is_some() {
//            if !v.render_commands.is_empty() {
//                return Some(k.pipeline);
//            }
//            None
//        });
//        let mut pipeline_id = match pipeline_id {
//            None => continue,
//            Some(id) => id,
//        };
//        debug!("{:#?}", render_target.ops);
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: if cleared {
                    Operations {
                        load: LoadOp::Load,
                        store: true,
                    }
                } else {
                    render_target.ops
                },
            }],
            depth_stencil_attachment: None,
        });
        cleared = true;
//        trace!("new pass");
        let mut iter = command_map.iter_mut().peekable();
        let mut pipeline_id = match iter.peek() {
            None => continue,
            Some((k, _)) => k.pipeline,
        };
        if pipeline_id != TextPipeline::id() {
            pass.set_pipeline(&pipelines.pipelines[&pipeline_id]);
        }
        let rt_rect = ScissorRect {
            x: 0,
            y: 0,
            w: render_target_size.x,
            h: render_target_size.y,
        };
        let mut rect = rt_rect.clone();
//        debug!("{:#?}", render_target_size.0);
        for (key, commands) in iter {
            if commands.render_commands.is_empty() {
                continue;
            }
            if pipeline_id != key.pipeline {
                if let Some(pipeline) = pipelines.pipelines.get(&key.pipeline) {
                    pass.set_pipeline(pipeline);
                    pipeline_id = key.pipeline;
                } else {
                    continue;
                }
            }
            if key.rect != rect {
                if key.rect == ScissorRect::default() {
                    pass.set_scissor_rect(rt_rect.x, rt_rect.y, rt_rect.w, rt_rect.h);
                    rect = rt_rect;
                } else {
                    pass.set_scissor_rect(key.rect.x, key.rect.y, key.rect.w, key.rect.h);
                    rect = key.rect;
                }
                trace!("set scissor {:?}", rect);
            }
//            error!("command {:?}", commands.render_commands);
            for command in commands.render_commands.drain(..) {
                match command {
                    RenderCommand::SetVertexBuffer { slot, id, range } => {
                        let buffer = buffers.buffers[id].as_ref().unwrap();
                        if range.end == 0 {
                            pass.set_vertex_buffer(slot as u32, buffer.buffer.slice(range.start..))
                        } else {
                            pass.set_vertex_buffer(slot as u32, buffer.buffer.slice(range.clone()))
                        }
                    }
                    RenderCommand::SetIndexBuffer { format, id, range } => {
                        let buffer = buffers.buffers[id].as_ref().unwrap();
                        if range.end == 0 {
                            pass.set_index_buffer(buffer.buffer.slice(range.start..), format)
                        } else {
                            pass.set_index_buffer(buffer.buffer.slice(range.clone()), format)
                        }
                    }
                    RenderCommand::SetBindGroup { slot, id } => {
                        pass.set_bind_group(
                            slot as u32,
                            arena.get(id),
                            &[],
                        );
                    }
                    RenderCommand::DrawIndexed {
                        indices,
                        base_vertex,
                        instances,
                    } => {
                        pass.draw_indexed(indices.clone(), base_vertex, instances.clone());
                    }
                    RenderCommand::Draw {
                        vertices,
                        instances,
                    } => {
                        pass.draw(vertices.clone(), instances.clone());
                    }
                }
            }
            commands.clear_render_commands();
        }
        drop(pass);
        for (_key, commands) in command_map.iter_mut().filter(|x| {
            x.0.pipeline == TypeId::of::<TextPipeline>() && !x.1.text_commands.is_empty()
        }) {
            let mut current_angle = match commands.text_commands.first().unwrap() {
                RenderTextCommand::Section { angle_deg: angle, .. } => *angle,
                RenderTextCommand::Batch { angle_deg: angle, .. } => *angle,
            };
            let mut enq = |glyph_brush: &mut GlyphBrush<()>, _angle: f32, bounds: glyph_brush_layout::ab_glyph::Rect, _pos: (f32, f32)| {
                let angle = -90.;
                let angle = angle / 180. * f32::PI();
                let translation = Vec2::new(
//                    0.,
//                    bounds.max.x,
//                    bounds.max.y,
                    (bounds.max.x + bounds.min.x) / 2.,
                    (bounds.max.y + bounds.min.y) / 2.
                );
//                translation.y -= pos.0 * angle.sin();
//                translation.x += pos.1 * angle.sin();
//                let translation = Vec2::new(
//                    pos.0, pos.1 - pos.0 * angle.sin(),
//                );
//                let translation = pixel_length_to_screen_space(pixel_middle, render_target_size.0);
                let transform = wgpu_glyph::orthographic_projection(render_target_size.x, render_target_size.y);
                let transform = Mat4::from_cols_array(&transform);
//                dbg!(transform);
//                let transform =  Mat4::orthographic_rh_gl(0., render_target_size.x as f32, render_target_size.y as f32, 0., 1., 0.);
                let transform = transform * Mat4::from_translation(Vec3::new(translation.x, translation.y, 0.));
                let transform = transform * Mat4::from_axis_angle(Vec3::Z, angle);
                let transform = transform * Mat4::from_translation(Vec3::new(-translation.x, -translation.y, 0.));
//                dbg!(transform);
//                dbg!(transform);
//                dbg!(transform);
//                let transform = transform * glam::Mat4::from_scale(Vec3::new(1.33 + 0.3, 1., 1.));
//                dbg!(transform);
                let transform = transform.to_cols_array();
                glyph_brush.draw_queued_with_transform(
                    &device,
                    staging_belt,
                    encoder,
                    view,
                    transform,
                ).unwrap();
//                glyph_brush.draw_queued_with_transform_and_scissoring(
//                    &device,
//                    staging_belt,
//                    encoder,
//                    view,
//                    transform,
//                    Region {
//                        x: key.rect.x,
//                        y: key.rect.y,
//                        width: key.rect.w,
//                        height: key.rect.h,
//                    },
//                ).unwrap();
            };
            for command in commands.text_commands.drain(..) {
                match command {
                    RenderTextCommand::Section { angle_deg: angle, section } => {
//                        if angle == angle {
//                            if let Ok(section) = sections.get(section) {
//                                glyph_brush.queue(&section.section);
//                            }
//                        } else {
                            current_angle = angle;

                            if let Ok(section) = sections.get(section) {
                                if let Some(bounds) = glyph_brush.glyph_bounds(&section.section) {
                                    glyph_brush.queue(&section.section);
                                    enq(glyph_brush, current_angle, bounds, section.section.screen_position);
                                }
                            }
//                        }
                    }
                    RenderTextCommand::Batch { angle_deg: _angle, batch } => {
//                        if current_angle != angle {
//                            enq(glyph_brush, current_angle);
//                            current_angle = angle;
//                        }
                        if let Ok(sections) = batch_sections.get(batch) {
                            for section in &sections.0 {
                                if let Some(bounds) = glyph_brush.glyph_bounds(section) {
                                    glyph_brush.queue(section);
                                    enq(glyph_brush, current_angle, bounds, section.screen_position);
                                }
                            }
                        }
                    }
                }
            }
            // TOOD: sometimes we get invalid scissorRect paramters when we drop render pass,
            //  this might due to bug in winit not sychronizing events with window size
        }
    }
}

fn draw(
    mut writer: UniqueEventWriter<RedrawWindow>,
    mut drawn_render_targets: Local<Vec<Entity>>,
    mut reader: UniqueEventReader<RedrawRenderTarget>,
    queue: Res<Queue>,
    device: Res<Device>,
    mut arena: ResMut<BindGroupArena>,
    render_targets: Query<(
        Entity,
        &RenderTarget,
        Option<&RenderTextureId>,
        Option<&RenderSurface>,
        Option<&ParentRenderTarget>,
        &RenderTargetSize,
    )>,
    pipelines: Res<RenderPipelines>,
    buffers: Res<Buffers>,
    mut glyph_brushes: ResMut<GlyphBrushes>,
    mut commands: ResMut<RenderCommands>,
    sections: Query<&Text>,
    batch_sections: Query<&TextBatch>,
    mut ui_data: ResMut<ImguiDrawDatas>,
    mut ui_renderer: ResMut<imgui_wgpu::Renderer>,
) {
    let now = Instant::now();
    // any buffer writes that happened should first be enqueued before we use them
    queue.submit(std::iter::empty());
    arena.written = false;
    let mut iter = reader.iter().peekable();
    if iter.peek().is_none() {
        return;
    }
    let mut staging_belt = StagingBelt::new(1024);
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("render_system"),
    });
    let mut frames = Vec::new();
    for e in iter {
        let (render_target_id, render_target, texture_id, surface, _, render_target_size) =
            ok_loop!(render_targets.get(e.id));
        drawn_render_targets.push(render_target_id);
        if let Some(texture_id) = texture_id {
            let view = ui_renderer.textures.get(texture_id.0).unwrap().view();
            process_commands(
                view,
                &render_target,
                render_target_size,
                render_target_id,
                &buffers,
                &arena,
                &device,
                &sections,
                &batch_sections,
                &pipelines,
                &mut encoder,
                &mut staging_belt,
                &mut glyph_brushes,
                &mut commands,
            );
        }
        if let Some(surface) = surface {
            let surface: &RenderSurface = surface;
            let frame = surface.0.get_current_texture().unwrap();
            let view = frame.texture.create_view(&TextureViewDescriptor::default());
            frames.push(frame);
            if let Some(data) = ui_data.0.get(&render_target_id) {
                let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 0.0,
                            }),
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
                ui_renderer
                    .render(data.0, &queue, &device, &mut pass)
                    .unwrap();
            }
        }
    }
    // release ui references for current frame
    ui_data.0.clear();
    // must finish before submiting
    staging_belt.finish();
    queue.submit([encoder.finish()]);
    let fut = queue.on_submitted_work_done();
    async move {
        fut.await;
        debug!("draw took: {} ms", now.elapsed().as_millis());
    }
    .spawn();
    for frame in frames {
        frame.present();
    }
    staging_belt.recall().spawn();
    for i in 0..drawn_render_targets.len() {
        let (_, _, _, _, parent_render_target, _) =
            render_targets.get(drawn_render_targets[i]).unwrap();
        let parent = some_loop!(parent_render_target).0;
        if drawn_render_targets
            .iter()
            .find(|x| **x == parent)
            .is_none()
        {
            // NOTE: only works for render targets that are directly under window
            writer.send(RedrawWindow { id: parent });
        }
    }
    drawn_render_targets.clear();
}

fn startup_renderer(
    windows: Query<(Entity, &Window)>,
    mut brushes: ResMut<GlyphBrushes>,
    mut render_commands: ResMut<RenderCommands>,
    mut commands: Commands,
) {
    info!("Starting up renderer");
    let (id, window) = windows.iter().next().unwrap();

    let instance = Instance::new(Backends::VULKAN);
    let surface = unsafe { instance.create_surface(&**window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .block()
        .unwrap();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
            //            Some(path_buf.as_path()),
        )
        .block()
        .unwrap();
    let format = surface.get_preferred_format(&adapter).unwrap();
    create_render_target(
        surface,
        format,
        &device,
        id,
        window,
        &mut *brushes,
        &mut *render_commands,
        &mut commands,
    );
    //    let path_buf = PathBuf::from("trace");
    debug!("{:#?}", adapter.get_info());

    commands.insert_resource_safe(PreferredSurfaceFormat(format));
    commands.insert_resource_safe(device);
    commands.insert_resource_safe(queue);
    commands.insert_resource_safe(adapter);
    commands.insert_resource_safe(instance);
    commands.insert_resource_safe(BindGroupArena {
        arenas: vec![],
        non_data_bind_groups: vec![],
        written: false,
    });
}

pub struct TextPipeline;

#[derive(Default, Debug)]
pub struct RenderCommands {
    commands: HashMap<Entity, BTreeMap<OrderedFloat<f32>, HashMap<RenderKey, CommandBuilder>>>,
}

impl RenderCommands {
    pub fn render_target(&mut self, id: Entity) -> RenderTargetCommandsBuilder {
        let commands = self.commands.entry(id).or_default();
        RenderTargetCommandsBuilder { commands }
    }
}

pub struct RenderTargetCommandsBuilder<'a> {
    commands: &'a mut BTreeMap<OrderedFloat<f32>, HashMap<RenderKey, CommandBuilder>>,
}

impl<'a> RenderTargetCommandsBuilder<'a> {
    pub fn drawer<Pipeline: ?Sized + 'static>(
        &mut self,
        z: Option<&ZComponent>,
        rect: ScissorRect,
    ) -> &mut CommandBuilder {
        let key = RenderKey {
            rect,
            pipeline: TypeId::of::<Pipeline>(),
        };
        self.commands
            .entry(z.map(|x| x.0).unwrap_or_default().into())
            .or_default()
            .entry(key)
            .or_default()
    }
}

#[derive(Clone, Debug)]
pub enum RenderCommand {
    SetVertexBuffer {
        slot: u8,
        id: BufferId,
        range: Range<BufferAddress>,
    },
    SetIndexBuffer {
        format: IndexFormat,
        id: BufferId,
        range: Range<BufferAddress>,
    },
    SetBindGroup {
        slot: u8,
        id: BindGroupId,
    },
    DrawIndexed {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    },
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
}

#[derive(Debug)]
pub enum RenderTextCommand {
    Section { angle_deg: f32, section: Entity },
    Batch { angle_deg: f32, batch: Entity },
}

struct Arena {
    buffer: Buffer,
    free_spaces: ArrayVec<FreeSpace, 255>,
    bind_groups: ArrayVec<Option<BindGroup>, 255>,
}

pub struct BindGroupArena {
    arenas: Vec<Arena>,
    non_data_bind_groups: Vec<BindGroup>,
    written: bool,
}

impl BindGroupArena {
    pub fn get_mut(&mut self, id: BindGroupId) -> &mut BindGroup {
        if id.references_data() {
            self.arenas[id.buffer as usize].bind_groups[id.id as usize]
                .as_mut()
                .unwrap()
        } else {
            &mut self.non_data_bind_groups[id.buffer as usize]
        }
    }

    pub fn get(&self, id: BindGroupId) -> &BindGroup {
        if id.references_data() {
            self.arenas[id.buffer as usize].bind_groups[id.id as usize]
                .as_ref()
                .unwrap()
        } else {
            &self.non_data_bind_groups[id.buffer as usize]
        }
    }

    pub fn write_material<T: AsBytes>(&mut self, material: &Material<T>, queue: &Queue) {
        self.written = true;
        //        debug!(
        //            "write material {:?}, {:?}",
        //            bytemuck::cast_slice::<_, f32>(material.data.as_bytes().as_ref()),
        //            material.id
        //        );
        let buffer = &self.arenas[material.id.buffer as usize].buffer;
        let _bytes = material.data.as_bytes().as_ref();
        if material.realloc_needed() {
            panic!("material length has been changed");
        }
        queue.write_buffer(
            buffer,
            (material.id.start_block as BufferAddress) * 256,
            material.data.as_bytes().as_ref(),
        );
    }

    pub fn store_non_data_bind_group(&mut self, group: BindGroup) -> BindGroupId {
        self.non_data_bind_groups.push(group);
        BindGroupId {
            buffer: (self.non_data_bind_groups.len() - 1) as u32,
            start_block: u8::MAX,
            len_blocks: u8::MAX,
            id: u8::MAX,
        }
    }

    /// Data doesn't have to be 256 byte aligned
    pub fn allocate_bind_group<T: MaterialLayout>(
        &mut self,
        device: &Device,
        queue: &Queue,
        data: &T,
    ) -> BindGroupId {
        let actual_len =
            u16::try_from(data.as_bytes().len()).expect("max bind group size is 64 KiB - 1");
        // align data
        let bytes;
        let mut copy_data;
        let aligned_len = if actual_len % 256 != 0 {
            copy_data = Vec::from(data.as_bytes());
            let aditional = 256 - actual_len % 256;
            copy_data.extend((0..aditional).into_iter().map(|_| 0));
            bytes = copy_data.into();
            actual_len + aditional
        } else {
            bytes = data.as_bytes();
            actual_len
        };

        let create_bind_group = |buffer, offset| {
            let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[data.aligned_layout()],
            });
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: T::slot(),
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer,
                        offset,
                        size: Some(NonZeroU64::new(aligned_len as u64).unwrap()),
                    }),
                }],
            })
        };

        match self.find_free_space((aligned_len / 256) as u8) {
            None => {
                let buffer_size = aligned_len.max(1 << 14);
                let buffer = device.create_buffer(&BufferDescriptor {
                    label: None,
                    size: buffer_size as u64,
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    mapped_at_creation: true,
                });
                buffer
                    .slice(..aligned_len as BufferAddress)
                    .get_mapped_range_mut()
                    .copy_from_slice(bytes.as_ref());
                buffer.unmap();
                let bind_group = Some(create_bind_group(&buffer, 0));
                self.arenas.push(Arena {
                    free_spaces: ArrayVec::new(),
                    bind_groups: ArrayVec::new(),
                    buffer,
                });
                let arena = self.arenas.last_mut().unwrap();
                arena.free_spaces.push(FreeSpace {
                    start: (aligned_len / 256) as u8,
                    len: ((buffer_size - aligned_len) / 256) as u8,
                });
                arena.bind_groups.push(bind_group);
                BindGroupId {
                    id: 0,
                    buffer: self.arenas.len() as u32 - 1,
                    start_block: 0,
                    len_blocks: (aligned_len / 256) as u8,
                }
            }
            Some((buffer_id, start_block)) => {
                let start = start_block as u64 * 256;
                let arena = &mut self.arenas[buffer_id as usize];
                queue.write_buffer(&arena.buffer, start, bytes.as_ref());
                let group = create_bind_group(&arena.buffer, start);
                let id = match arena.bind_groups.iter().position(|x| x.is_none()) {
                    None => {
                        arena.bind_groups.push(Some(group));
                        arena.bind_groups.len() - 1
                    }
                    Some(pos) => {
                        arena.bind_groups[pos] = Some(group);
                        pos
                    }
                } as u8;
                BindGroupId {
                    id,
                    buffer: buffer_id,
                    start_block,
                    len_blocks: (aligned_len / 256) as u8,
                }
            }
        }
    }

    pub fn free_bind_group(&mut self, id: BindGroupId) {
        let arena = &mut self.arenas[id.buffer as usize];
        let index = arena
            .free_spaces
            .binary_search_by_key(&id.start_block, |x| x.start as _)
            .unwrap_err();
        if let Some(space) = arena.free_spaces.get_mut(index.wrapping_sub(1)) {
            if space.start + space.len == id.start_block {
                space.len += id.len_blocks;
                return;
            }
        }
        arena.bind_groups.remove(id.id as usize);
        arena.free_spaces.insert(
            index,
            FreeSpace {
                start: id.start_block,
                len: id.len_blocks,
            },
        );
    }

    fn find_free_space(&mut self, block_len: u8) -> Option<(u32, u8)> {
        for (i, arena) in self.arenas.iter_mut().enumerate() {
            for space in &mut arena.free_spaces {
                if space.len >= block_len {
                    let start = space.start;
                    space.len -= block_len;
                    space.start += block_len;
                    return Some((i as u32, start));
                }
            }
        }
        None
    }
}

fn startup_ui_renderer(
    atlas: NonSend<SharedFontAtlas>,
    mut event: EventReader<WindowCreated>,
    windows: Query<&Window>,
    device: Res<Device>,
    format: Res<PreferredSurfaceFormat>,
    queue: Res<Queue>,
    mut commands: Commands,
) {
    //    let atlas = world.remove_non_send::<SharedFontAtlas>().unwrap();
    let ui_renderer = {
        let id = event.iter().next().unwrap().id;
        let window = windows.get(id).unwrap();
        // we don't have to store context because new one will be created when window is created
        let mut context = ImguiContext::new(window, &atlas);

        debug!("ui format {:#?}", format.0);
        let renderer_config = RendererConfig {
            texture_format: format.0,
            ..Default::default()
        };

        imgui_wgpu::Renderer::new(&mut context.context, &device, &queue, renderer_config)
    };

    commands.insert_resource_safe(ui_renderer);
    //    world.insert_non_send_resource_safe(atlas);
}

pub struct FreeSpace {
    start: u8,
    len: u8,
}

#[derive(Default)]
pub struct Buffers {
    buffers: Vec<Option<MyBuffer>>,
}

impl Buffers {
    pub fn get(&self, id: BufferId) -> &MyBuffer {
        self.buffers[id].as_ref().unwrap()
    }

    pub fn push_buffer(&mut self, buffer: MyBuffer) -> BufferId {
        let id = match self.buffers.iter().position(|x| x.is_none()) {
            None => {
                let len = self.buffers.len();
                self.buffers.push(None);
                len
            }
            Some(id) => id,
        };
        self.buffers[id] = Some(buffer);
        id
    }
}

pub type PipelineId = std::any::TypeId;
pub type BufferId = usize;
pub type RenderTargetId = usize;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct BindGroupId {
    pub(super) buffer: u32,
    pub(super) start_block: u8,
    pub(super) len_blocks: u8,
    pub(super) id: u8,
}

impl BindGroupId {
    pub fn references_data(&self) -> bool {
        self.start_block != u8::MAX
    }

    pub fn invalid() -> BindGroupId {
        BindGroupId {
            buffer: u32::MAX,
            start_block: u8::MAX,
            len_blocks: u8::MAX,
            id: u8::MAX,
        }
    }

    pub fn is_invalid(&self) -> bool {
        *self == Self::invalid()
    }
}

#[derive(Default)]
pub struct Shaders {
    pub shaders: Vec<ShaderModule>,
}

#[derive(Debug)]
pub struct MyBuffer {
    size: usize,
    buffer: Buffer,
    capacity: usize,
}

impl MyBuffer {
    pub fn new<T: Pod>(data: &[T], device: &Device) -> Self {
        let size = data.len() * T::size();
        MyBuffer {
            size,
            buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&data),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            }),
            capacity: size,
        }
    }

    pub fn with_capacity(capacity: usize, device: &Device) -> Self {
        MyBuffer::with_capacity_and_usage(
            capacity,
            device,
            BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        )
    }

    pub fn with_capacity_and_usage(capacity: usize, device: &Device, usage: BufferUsages) -> Self {
        MyBuffer {
            size: 0,
            buffer: device.create_buffer(&BufferDescriptor {
                label: None,
                size: (capacity) as _,
                usage,
                mapped_at_creation: false,
            }),
            capacity,
        }
    }

    pub fn update<T: Pod>(&mut self, data: &[T], device: &Device, queue: &Queue) {
        self.size = data.len() * T::size();
        if self.capacity < self.size {
            self.buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            });
            self.capacity = self.size;
        } else {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
}

#[derive(Deref, DerefMut, Default)]
pub struct GlyphBrushes(pub HashMap<TextureFormat, GlyphBrush<()>>);

#[derive(Debug)]
pub struct CommandBuilder {
    render_commands: Vec<RenderCommand>,
    text_commands: Vec<RenderTextCommand>,
    bind_groups: [BindGroupId; 4],
    vertex_buffers: [BufferId; 8],
}

impl CommandBuilder {
    fn clear_render_commands(&mut self) {
        self.render_commands.clear();
        for group in &mut self.bind_groups {
            *group = BindGroupId::invalid();
        }
        for vbo in &mut self.vertex_buffers {
            *vbo = BufferId::MAX;
        }
    }

    #[inline]
    pub fn set_bind_group(&mut self, slot: usize, id: BindGroupId) {
        if self.bind_groups[slot] == id {
            return;
        }
        self.render_commands.push(RenderCommand::SetBindGroup {
            slot: slot as u8,
            id,
        });
        self.bind_groups[slot] = id;
    }

    pub fn set_vertex_buffer(&mut self, slot: usize, id: BufferId, range: Option<Range<usize>>) {
        if self.vertex_buffers[slot] == id {
            return;
        }
        //        let start = match range.start_bound() {
        //            Bound::Included(start) => start,
        //            Bound::Excluded(_) => unreachable!(),
        //            Bound::Unbounded => 0,
        //        };
        //        let end = match range.end_bound() {
        //            Bound::Included(end) => end,
        //            Bound::Excluded(_) => unreachable!(),
        //            Bound::Unbounded => 0,
        //        };
        let range = range.unwrap_or(0..0);
        self.render_commands.push(RenderCommand::SetVertexBuffer {
            slot: slot as u8,
            id,
            range: range.start as u64..range.end as u64,
        });
        self.vertex_buffers[slot] = id;
    }

    pub fn draw(&mut self, vertices: Range<usize>, instances: Range<usize>) {
        self.render_commands.push(RenderCommand::Draw {
            vertices: vertices.start as u32..vertices.end as u32,
            instances: instances.start as u32..instances.end as u32,
        })
    }

    pub fn text(&mut self, entity: Entity, angle: f32) {
        self.text_commands.push(RenderTextCommand::Section {
            angle_deg: angle,
            section: entity,
        });
    }

    pub fn text_batch(&mut self, entity: Entity, angle: f32) {
        self.text_commands.push(RenderTextCommand::Batch {
            angle_deg: angle,
            batch: entity,
        });
    }
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self {
            render_commands: vec![],
            text_commands: vec![],
            bind_groups: [BindGroupId::invalid(); 4],
            vertex_buffers: [BufferId::MAX; 8],
        }
    }
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct UploadMeshLabel;

pub fn upload_mesh<T: Pod + Send + Sync>(
    queue: Res<Queue>,
    device: Res<Device>,
    mut buffers: ResMut<Buffers>,
    query: Query<(&Mesh<T>, &GpuMesh), Changed<Mesh<T>>>,
) {
    for (mesh, buffer) in query.iter() {
        let buffer = buffers.buffers[buffer.buffer_id].as_mut().unwrap();
        buffer.update(&mesh.data, &device, &queue);
        let _data: &[f32] = bytemuck::cast_slice(&mesh.data);
        //        debug!("uploading mesh {:?}", data);
        let data = bytemuck::cast_slice(&mesh.data);
        queue.write_buffer(buffer.buffer(), 0, data);
    }
}

fn remove_render_target_for_window(
    mut event: EventReader<WindowClosing>,
    mut render_commands: ResMut<RenderCommands>,
    _commands: Commands,
) {
    for e in event.iter() {
        render_commands.commands.remove(&e.id);
    }
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
struct StartupCreateRenderTargetForWindowLabel;

fn create_render_target_for_window(
    adapter: Res<Adapter>,
    instance: Res<Instance>,
    device: Res<Device>,
    mut event: EventReader<WindowCreated>,
    windows: Query<(&Window, Option<&RenderTarget>)>,
    mut glyph_brushes: ResMut<GlyphBrushes>,
    mut render_commands: ResMut<RenderCommands>,
    mut commands: Commands,
) {
    for e in event.iter() {
        let (window, rt) = windows.get(e.id).unwrap();
        // render target already created at startup
        if rt.is_some() {
            continue;
        }
        let surface = unsafe { instance.create_surface(&**window) };
        let format = surface.get_preferred_format(&adapter).unwrap();
        create_render_target(
            surface,
            format,
            &device,
            e.id,
            window,
            &mut *glyph_brushes,
            &mut *render_commands,
            &mut commands,
        );
    }
}

fn create_render_target(
    surface: Surface,
    format: TextureFormat,
    device: &Device,
    id: Entity,
    window: &Window,
    brushes: &mut GlyphBrushes,
    render_commands: &mut RenderCommands,
    commands: &mut Commands,
) {
    info!("Creating render target for window");
    let size = window.inner_size();
    let size = UVec2::new(size.width, size.height);
    surface.configure(&device, &create_surface_configuration(format, size));
    commands
        .entity(id)
        .insert_bundle(RenderTargetBundle::new(
            format,
            Operations {
                load: LoadOp::Clear(wgpu::Color {
                    r: 0.952,
                    g: 0.,
                    b: 0.952,
                    a: 1.0,
                }),
                store: true,
            },
            size,
        ))
        .insert(RenderSurface(surface));
    render_commands.commands.insert(id, Default::default());
    brushes.0.entry(format).or_insert_with(|| {
        GlyphBrushBuilder::using_font(
            ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf")).unwrap(),
        )
        .build(&device, format)
    });
    info!("render target created");
}
