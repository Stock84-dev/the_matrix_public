use std::any::TypeId;
use std::collections::{HashMap};

use std::num::{NonZeroU32, NonZeroU64};
use std::ops::{Range};
use std::sync::{Arc};


use bytemuck::Pod;
use glyph_brush::OwnedSection;

use mouse::ext::{IdExt, PodExt, StaticSize, VecExt};
use mouse::log::{trace};
use mouse::num::NumExt;



use wgpu::{
    BindGroupLayout, BindGroupLayoutEntry, BindingType, BufferBindingType, Color, Device, Extent3d, Operations, PresentMode, Queue, RenderPipeline, ShaderStages, Surface, SurfaceConfiguration, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};







use crate::niobe::{Text, View, ZComponent};
use crate::winit_plugin::{
    RedrawWindow, WindowResized, WinitPlugin,
};
use crate::{
    find_root_parent, MutExt, RenderTargetPos, RenderTargetSize, Stages, TypeIdLabel,
};

mod drawer;
use std::borrow::Cow;
use std::fmt::Debug;


use bevy::prelude::*;
pub use drawer::*;
use imgui::TextureId;


use mouse::serde::Serialize;
use stretch::Stretch;

use crate::flex::FlexNode;
use crate::render::utils::GroupMaterial;

pub mod shaders;
pub mod utils;
pub mod pipelines {
    pub mod d1 {}
    pub mod d2 {
        pub mod line;
        pub mod line_strip;
        pub mod parallel_coordinates;
    }
}

pub struct RenderPlugin;

impl Plug for RenderPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(WinitPlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.init_resource::<Shaders>()
            .init_resource::<Buffers>()
            .init_resource::<RenderCommands>()
            .init_resource::<RenderPipelines>()
            .init_resource::<UniqueEvents<RedrawRenderTarget>>()
            .add_system_to_stage(
                // updating events at this stage so that systems on later stages would pick up
                // events sooner
                CoreStage::PreUpdate,
                UniqueEvents::<RedrawRenderTarget>::update_system,
            )
            .add_event::<ResizeRenderTarget>()
            .added_system(update_sampled_material)
            .pre_pre_system(redraw_render_target_on_window_redraw)
            .pre_pre_system(configure_surfaces)
            .post_system(mesh_pixel_to_screen_space.before(RescalePixelSpaceMeshLabel))
            .post_system(rescale_pixel_space_mesh.label(RescalePixelSpaceMeshLabel))
            .render_system(text_system);
        add_material_systems::<GroupMaterial>(app);
        drawer::build(app);
        app
    }
}

#[derive(Eq, PartialEq, Hash, Debug, new)]
pub struct RedrawRenderTarget {
    pub id: Entity,
}

pub struct ResizeRenderTarget {
    pub id: Entity,
    pub size: UVec2,
}

fn redraw_render_target_on_window_redraw(
    mut reader: UniqueEventReader<RedrawWindow>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
) {
    for e in reader.iter() {
        writer.send(RedrawRenderTarget { id: e.id });
    }
}
pub struct RenderTargetResized {
    pub id: Entity,
}

#[derive(Component, Deref, DerefMut)]
pub struct RenderTextureId(pub TextureId);

#[derive(Component)]
pub struct RenderTarget {
    pub ops: Operations<Color>,
    pub format: TextureFormat,
}

#[derive(Component)]
struct RenderSurface(Surface);

#[derive(Component)]
pub struct RenderTexture {
    pub texture: Texture,
    pub view: TextureView,
}

#[derive(Bundle)]
pub struct RenderTargetBundle {
    rt: RenderTarget,
    size: RenderTargetSize,
    prev_size: PrevRenderTargetSize,
}

impl RenderTargetBundle {
    pub fn new(format: TextureFormat, ops: Operations<Color>, size: UVec2) -> Self {
        Self {
            rt: RenderTarget { ops, format },
            size: RenderTargetSize(size),
            prev_size: PrevRenderTargetSize(UVec2::ZERO),
        }
    }
}

#[derive(Bundle)]
pub struct ChildRenderTargetBundle {
    #[bundle]
    rt: RenderTargetBundle,
    pos: RenderTargetPos,
    texture_id: RenderTextureId,
    parent_rt: ParentRenderTarget,
}

impl ChildRenderTargetBundle {
    pub fn new(
        rt: RenderTargetBundle,
        pos: UVec2,
        window_entity: Entity,
        device: &Device,
        ui_renderer: &mut imgui_wgpu::Renderer,
    ) -> Self {
        let texture_config = imgui_wgpu::TextureConfig {
            size: Extent3d {
                width: rt.size.0.x,
                height: rt.size.0.y,
                ..Default::default()
            },
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            format: Some(rt.rt.format),
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(&device, &ui_renderer, texture_config);
        let texture_id = ui_renderer.textures.insert(texture);
        Self {
            rt,
            pos: RenderTargetPos(pos),
            texture_id: RenderTextureId(texture_id),
            parent_rt: ParentRenderTarget(window_entity),
        }
    }
}

impl RenderTexture {
    pub fn new(width: u32, height: u32, format: TextureFormat, device: &Device) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        });

        let view = texture.create_view(&TextureViewDescriptor::default());
        Self { texture, view }
    }
}

// impl RenderTarget {
//    fn new(view: Option<TextureId>, width: u32, height: u32) -> Self {
//        RenderTarget {
//            ops: Operations {
//                load: LoadOp::Clear(wgpu::Color {
//                    r: 0.0,
//                    g: 0.0,
//                    b: 0.0,
//                    a: 0.0,
//                }),
//                store: true,
//            },
//            texture_id: view,
//            size: UVec2::new(width, height),
//        }
//    }
//
//    pub fn pixel_scale(&self) -> Vec2 {
//        Vec2::new(1. / self.size.x as f32, 1. / self.size.y as f32)
//    }
//}

#[derive(Component)]
pub struct MeshClip {
    pub visible_range: Range<usize>,
}

#[derive(Component)]
pub struct GpuMesh {
    pub buffer_id: BufferId,
}

//#[derive(Component)]
// pub struct Redraw(pub bool);

#[derive(Component)]
pub struct TextBatch(pub Vec<OwnedSection>);

#[derive(Component)]
/// In degrees
pub struct Angle(pub f32);

impl GpuMesh {
    pub fn invalid() -> Self {
        Self {
            buffer_id: BufferId::MAX,
        }
    }
    pub fn new<T: Pod>(data: &[T], buffers: &mut Buffers, device: &Device) -> Self {
        Self {
            buffer_id: buffers.push_buffer(MyBuffer::new(data, device)),
        }
    }

    pub fn with_capacity(capacity: usize, buffers: &mut Buffers, device: &Device) -> Self {
        let buffer = MyBuffer::with_capacity(capacity, device);
        Self {
            buffer_id: buffers.push_buffer(buffer),
        }
    }
}
#[derive(Bundle)]
pub struct MeshBundle<T: AsBytes + Send + Sync> {
    mesh: Mesh<T>,
    gpu_mesh: GpuMesh,
}

impl<T: AsBytes + Send + Sync> MeshBundle<T> {
    pub fn new(data: Vec<T>, buffers: &mut Buffers, device: &Device) -> Self {
        Self {
            gpu_mesh: GpuMesh::with_capacity(data.size(), buffers, device),
            mesh: Mesh::new(data),
        }
    }

    pub fn empty(buffers: &mut Buffers, device: &Device) -> Self {
        Self {
            gpu_mesh: GpuMesh::with_capacity(256, buffers, device),
            mesh: Mesh::new(Vec::new()),
        }
    }
}

#[derive(Component)]
pub struct PrevRenderTargetSize(pub UVec2);

#[derive(Component, Deref, DerefMut)]
pub struct Mesh<T: AsBytes> {
    pub data: Vec<T>,
}

impl<T: AsBytes> Mesh<T> {
    pub fn new(data: Vec<T>) -> Self {
        Self { data }
    }
}

pub trait MaterialLayout: AsBytes {
    fn slot() -> u32 {
        0
    }

    fn aligned_layout(&self) -> BindGroupLayoutEntry {
        let mut entry = self.layout();
        match &mut entry.ty {
            BindingType::Buffer {
                min_binding_size, ..
            } => {
                if let Some(min_binding_size) = min_binding_size {
                    let actual_len: u64 = (*min_binding_size).into();
                    let aligned_len = if actual_len % 256 != 0 {
                        let aditional = 256 - actual_len % 256;
                        actual_len + aditional
                    } else {
                        actual_len
                    };
                    *min_binding_size = NonZeroU64::new(aligned_len).unwrap();
                }
            }
            _ => {}
        }
        entry.into()
    }

    fn layout(&self) -> NonAlignedBindGroupLayoutEntry {
        NonAlignedBindGroupLayoutEntry {
            binding: Self::slot(),
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(NonZeroU64::new(self.n_bytes() as u64).unwrap()),
            },
            count: None,
        }
    }
}

pub trait DynMaterial: Serialize {
    fn template(&self) -> &str;
}

pub struct NonAlignedBindGroupLayoutEntry {
    pub binding: u32,
    /// Which shader stages can see this binding.
    pub visibility: ShaderStages,
    /// The type of the binding
    pub ty: BindingType,
    /// If this value is Some, indicates this entry is an array. Array size must be 1 or greater.
    ///
    /// If this value is Some and `ty` is `BindingType::Texture`,
    /// [`Features::TEXTURE_BINDING_ARRAY`] must be supported.
    ///
    /// If this value is Some and `ty` is any other variant, bind group creation will fail.
    pub count: Option<NonZeroU32>,
}

impl Into<BindGroupLayoutEntry> for NonAlignedBindGroupLayoutEntry {
    fn into(self) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding: self.binding,
            visibility: self.visibility,
            ty: self.ty,
            count: self.count,
        }
    }
}

#[derive(Component, Debug)]
// system will automatically update uniform buffers whenever material changes
pub struct Material<T> {
    pub id: BindGroupId,
    pub data: T,
}

impl<T: AsBytes> Material<T> {
    pub fn new(data: T) -> Self {
        Self {
            id: BindGroupId::invalid(),
            data,
        }
    }

    pub fn realloc_needed(&self) -> bool {
        self.id.len_blocks as usize != self.data.as_bytes().len().div_ceil(256)
    }
}

#[derive(Component)]
pub struct SampledMaterialConfig {
    pub view: Arc<TextureView>,
}

fn update_sampled_material(
    device: Res<Device>,
    mut arena: ResMut<BindGroupArena>,
    query: Query<(Entity, &RenderTexture), Changed<RenderTexture>>,
    mut materials: Query<&mut SampledMaterial>,
) {
    for mut material in materials.iter_mut() {
        let texture = some_loop!(query.iter().find(|x| x.0 == material.texture));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &SampledMaterial::layout(&device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.1.view),
            }],
            label: Some("sampled_bind_group"),
        });
        if material.id.is_invalid() {
            material.id = arena.store_non_data_bind_group(group);
        } else {
            *arena.get_mut(material.id) = group;
        }
    }
}

#[derive(Component, Debug)]
// system will automatically update uniform buffers whenever material changes
pub struct SampledMaterial {
    pub texture: Entity,
    pub id: BindGroupId,
}

impl SampledMaterial {
    pub fn new<'a>(texture: Entity) -> Self {
        Self {
            texture,
            id: BindGroupId::invalid(),
        }
    }

    pub fn layout(device: &Device) -> BindGroupLayout {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                }],
                label: Some("texture_bind_group_layout"),
            });
        texture_bind_group_layout
    }
}

pub trait AsBytes: 'static {
    fn n_bytes(&self) -> usize;
    fn as_bytes<'a>(&'a self) -> Cow<'a, [u8]>;
}

impl<T: Pod> AsBytes for T {
    fn n_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn as_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
        PodExt::as_bytes(self).into()
    }
}
pub trait CompileMaterial: AsBytes {
    type Pipeline;
    fn build(&self, world: &World) -> RenderPipeline;
}

pub fn realloc_material<T: MaterialLayout + Send + Sync>(
    mut arena: ResMut<BindGroupArena>,
    device: Res<Device>,
    queue: Res<Queue>,
    mut query: Query<&mut Material<T>, Changed<Material<T>>>,
) {
    for mut material in query.iter_mut() {
        if material.realloc_needed() {
            arena.free_bind_group(material.id);
            let id = arena.allocate_bind_group(&device, &queue, &material.data);
            material.deref_mut_sneak().id = id;
        }
    }
}

pub fn add_compile_material_systems<T: CompileMaterial + MaterialLayout + Send + Sync + Debug>(
    app: &mut App,
) {
    add_material_systems::<T>(app);
    app.add_system_to_stage(
        Stages::Added,
        compile_material_added::<T>
            .exclusive_system()
            .label(TypeIdLabel::new::<T>()),
    );
    app.add_system_to_stage(
        Stages::Added,
        compile_material_changed::<T>
            .exclusive_system()
            .after(TypeIdLabel::new::<T>()),
    );
}

pub fn add_material_systems<T: MaterialLayout + Send + Sync + Debug>(app: &mut App) {
    app.add_system_to_stage(Stages::Added, material_added::<T>.system());
    app.add_system_to_stage(Stages::Render, material_changed::<T>.system());
}

fn compile_material_added<T: CompileMaterial + Send + Sync + Debug>(world: &mut World) {
    let mut pipelines = world.remove_resource::<RenderPipelines>().unwrap();
    let mut query = world.query_filtered::<&Material<T>, Added<Material<T>>>();
    for material in query.iter(world) {
        pipelines.pipelines.insert(
            std::any::TypeId::of::<T::Pipeline>(),
            material.data.build(world),
        );
    }
    world.insert_resource(pipelines);
}

fn compile_material_changed<T: CompileMaterial + Send + Sync>(world: &mut World) {
    let mut pipelines = world.remove_resource::<RenderPipelines>().unwrap();
    let mut query = world.query_filtered::<&Material<T>, Changed<Material<T>>>();
    for material in query.iter(world) {
        *pipelines
            .pipelines
            .get_mut(&std::any::TypeId::of::<T::Pipeline>())
            .unwrap() = material.data.build(world);
    }
    world.insert_resource(pipelines);
}

fn material_added<T: MaterialLayout + Send + Sync + Debug>(
    queue: Res<Queue>,
    device: Res<Device>,
    mut arena: ResMut<BindGroupArena>,
    mut query: Query<(&mut Material<T>, Entity), Added<Material<T>>>,
    _commands: Commands,
) {
    for (mut material, entity) in query.iter_mut() {
        let id = arena.allocate_bind_group(&device, &queue, &material.data);
        material.id = id;
        trace!("material added {:?} {:?}", entity, material);
    }
}

fn material_changed<T: AsBytes + Send + Sync + Debug>(
    queue: Res<Queue>,
    mut arena: ResMut<BindGroupArena>,
    query: Query<(&Material<T>, Entity), Changed<Material<T>>>,
) {
    for (material, entity) in query.iter() {
        trace!("material changed {:?} {:?}", entity, material);
        arena.write_material(material, &*queue);
    }
}

fn configure_surfaces(
    mut map: Local<HashMap<Entity, UVec2>>,
    mut event: EventReader<WindowResized>,
    mut writer: EventWriter<ResizeRenderTarget>,
    surfaces: Query<(&RenderSurface, &RenderTarget)>,
    device: Res<Device>,
) {
    for e in event.iter() {
        map.insert(e.id, UVec2::new(e.width, e.height));
    }
    for (id, size) in &*map {
        let (surface, rt) = ok_loop!(surfaces.get(*id));
        writer.send(ResizeRenderTarget {
            id: *id,
            size: *size,
        });
        surface
            .0
            .configure(&device, &create_surface_configuration(rt.format, *size));
    }
    map.clear();
}

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct RenderKey {
    pub rect: ScissorRect,
    pub pipeline: TypeId,
}

impl RenderKey {
    pub fn new<T: 'static>(rect: ScissorRect) -> Self {
        Self {
            rect,
            pipeline: std::any::TypeId::of::<T>(),
        }
    }
}

#[derive(Component, Default, Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct ScissorRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

//#[derive(Deref, DerefMut, Default, Debug)]
// struct Surfaces(HashMap<WindowId, RenderSurface>);

#[derive(Default)]
pub struct RenderPipelines {
    pub pipelines: HashMap<TypeId, RenderPipeline>,
}

impl RenderPipelines {
    fn add_static<T: 'static>(&mut self, pipeline: RenderPipeline) {
        self.pipelines.insert(T::id(), pipeline);
    }
}

fn create_surface_configuration(format: TextureFormat, size: UVec2) -> SurfaceConfiguration {
    SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.x,
        height: size.y,
        present_mode: PresentMode::Fifo,
    }
}

fn text_system(
    mut commands: ResMut<RenderCommands>,
    mut reader: UniqueEventReader<RedrawRenderTarget>,
    render_targets: Query<&Children>,
    views: Query<(&FlexNode, &Children), With<View>>,
    text_batches: Query<(Option<&ZComponent>, Option<&Angle>), With<TextBatch>>,
    texts: Query<(Option<&ZComponent>, Option<&Angle>), With<Text>>,
    stretch: NonSend<Stretch>,
) {
    for e in reader.iter() {
        let render_target_id = e.id;
        let children = ok_loop!(render_targets.get(render_target_id));
        let mut rt_drawer = commands.render_target(render_target_id);
        for child in children.iter() {
            let (flex_node, children) = ok_loop!(views.get(*child));
            let layout = stretch.layout(flex_node.0).unwrap();
            let scissor = ScissorRect {
                x: layout.location.x as u32,
                y: layout.location.y as u32,
                w: layout.size.width as u32,
                h: layout.size.height as u32,
            };
            for child in children.iter() {
                let mut process_text_batch = || {
                    let (z, angle) = ok!(text_batches.get(*child));
                    let builder = rt_drawer.drawer::<TextPipeline>(z, scissor);
                    builder.text_batch(*child, angle.map(|x| x.0).unwrap_or_default());
                };
                process_text_batch();
                let mut process_text = || {
                    let (z, angle) = ok!(texts.get(*child));
                    let builder = rt_drawer.drawer::<TextPipeline>(z, scissor);
                    builder.text(*child, angle.map(|x| x.0).unwrap_or_default());
                };
                process_text();
            }
        }
    }
}

pub struct PreferredSurfaceFormat(pub TextureFormat);

pub fn screen_to_pixel_space(screen: Vec2, screen_size_in_px: Vec2) -> Vec2 {
    let scale = Vec2::ONE / screen_size_in_px;
    // x (0.25 + 1) / 2 / 0.001 = 1.25 / 2 = 0.625
    // (screen + 1) / 2 / screen_pixel_size
    // y (1 - -0.25) / 2 / 0.001 = 1.25 / 2 = 0.625
    // (1 - screen) / 2 / screen_pixel_size
    Vec2::new(screen.x + 1., 1. - screen.y) / 2. / scale
}

pub fn screen_length_to_pixel_space(screen: Vec2, screen_size_in_px: UVec2) -> Vec2 {
    screen / 2. * screen_size_in_px.as_vec2()
}

pub fn pixel_to_screen_space(pixel: Vec2, screen_size_in_px: Vec2) -> Vec2 {
    let scale = Vec2::ONE / screen_size_in_px;
    // size = 1000, 1000
    // pos = 750, 750
    // x pos / scale * 2 - 1
    // y (1 - pos / scale) * 2 - 1
    let mut tmp = pixel * scale;
    tmp.y = 1. - tmp.y;
    tmp * Vec2::splat(2.) - Vec2::ONE
}

pub fn pixel_length_to_screen_space(pixel: Vec2, screen_size_in_px: UVec2) -> Vec2 {
    let screen_size_in_px = screen_size_in_px.as_vec2();
    pixel / screen_size_in_px * 2.
}

pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    pub fn from_min_max(min: Vec2, max: Vec2) -> Rect {
        Self { min, max }
    }

    pub fn to_mesh(&self, mesh: &mut impl Extend<Vec2>) {
        mesh.extend_reserve(4);
        mesh.extend_one(self.min);
        mesh.extend_one(Vec2::new(self.max.x, self.min.y));
        mesh.extend_one(Vec2::new(self.min.x, self.max.y));
        mesh.extend_one(self.max);
    }
}

#[derive(Component)]
pub struct PixelSpaceMesh {
    pub converted: bool,
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RescalePixelSpaceMeshLabel;

fn rescale_pixel_space_mesh(
    changed_render_targets: Query<
        (&RenderTargetSize, &PrevRenderTargetSize),
        Changed<RenderTargetSize>,
    >,
    parents: Query<&Parent>,
    mut meshes: Query<(&mut Mesh<Vec2>, &mut PixelSpaceMesh, &Parent)>,
) {
    for (mut mesh, mut space, parent) in meshes.iter_mut().filter(|x| !x.0.is_changed()) {
        let root = find_root_parent(&parents, **parent);
        let (size, prev_size) = match changed_render_targets.get(root) {
            Ok(x) => x,
            Err(_) => continue,
        };
        space.converted = true;
        trace!("rescaled mesh");

        for vertex in &mut mesh.deref_mut_sneak().data {
            *vertex = pixel_to_screen_space(
                screen_to_pixel_space(*vertex, prev_size.0.as_vec2()),
                size.0.as_vec2(),
            );
        }
    }
}

fn mesh_pixel_to_screen_space(
    parents: Query<&Parent>,
    mut changed_meshes: Query<
        (&mut Mesh<Vec2>, &mut PixelSpaceMesh, &Parent),
        Changed<Mesh<Vec2>>,
    >,
    render_targets_sizes: Query<&RenderTargetSize>,
) {
    for (mut mesh, mut space, parent) in changed_meshes.iter_mut() {
        if space.converted {
            space.converted = false;
            continue;
        }
        trace!("converted changed mesh");
        let root = find_root_parent(&parents, **parent);
        let size = render_targets_sizes.get(root).unwrap();
        for vertex in &mut mesh.deref_mut_sneak().data {
            *vertex = pixel_to_screen_space(*vertex, size.0.as_vec2());
        }
    }
}

#[test]
fn space_conversion() {
    let screen_size_in_px = Vec2::new(1000., 1000.);
    let point_px = Vec2::new(1., 1.);
    let point_sc = pixel_to_screen_space(point_px, screen_size_in_px);
    //    assert_eq!(point_sc.x, 0.5);
    assert_eq!(point_sc.y, -0.5);
    //    let p2 = screen_to_pixel_space(point_sc, screen_size_in_px);
    //    assert_eq!(p2.x, 750.);
    //    assert_eq!(p2.y, 750.);
}

#[derive(Component)]
pub struct ParentRenderTarget(pub Entity);

#[derive(Debug, Clone)]
pub struct RedrawWindowOnly {
    pub id: Entity,
}

pub fn pos_in_layout(pos: Vec2, layout: &stretch::result::Layout) -> bool {
    pos.x >= layout.location.x
        && pos.x < layout.location.x + layout.size.width
        && pos.y >= layout.location.y
        && pos.y < layout.location.y + layout.size.height
}

pub fn pos_in_rect(pos: Vec2, top_left: Vec2, size: Vec2) -> bool {
    pos.x >= top_left.x
        && pos.x < top_left.x + size.x
        && pos.y >= top_left.y
        && pos.y < top_left.y + size.y
}
