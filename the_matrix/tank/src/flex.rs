use bevy::prelude::*;
use bevy::utils::HashSet;
use rgb::RGBA;
use stretch::geometry::Point;
use stretch::node::Node;
use stretch::number::Number;
use stretch::style::Style;
use stretch::Stretch;
use wgpu::Device;

use crate::imgui_plugin::{ImguiPlugin, RenderTargetSizeAppliedLabel};
use crate::niobe::{View, ZComponent};
use crate::render::pipelines::d2::line::{LineMaterialData};
use crate::render::pipelines::d2::line_strip::{LineStripMaterial, LineStripPlugin};
use crate::render::utils::GroupMaterial;
use crate::render::{
    pos_in_layout, Buffers, Material, Mesh, MeshBundle, ParentRenderTarget, PixelSpaceMesh,
    RenderPlugin, RenderTarget, ResizeRenderTarget,
};
use crate::winit_plugin::{CursorKind, CursorMoved, MouseState, Window};
use crate::{find_root_parent, LineMaterialInPixelSpace, RenderTargetSize};

pub struct FlexboxPlugin;

impl Plug for FlexboxPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
            .load(RenderPlugin)
            .load(ImguiPlugin)
            .load(LineStripPlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.insert_non_send_resource(Stretch::new())
            //            .init_resource::<DebugLayout>()
            //            .add_event::<LayoutChanged>()
            // must be run after sizes have updated
            .pre_pre_system(compute_layout.after(RenderTargetSizeAppliedLabel))
            .pre_pre_system(add_debug_layout)
            .pre_system(debug_layout_system)
            .add_system(remove_debug_layout)
            .pre_system(cursor_system)
    }
}

#[derive(Default)]
pub struct DebugLayout(bool);

fn cursor_system(
    flex: NonSend<Stretch>,
    mut reader: EventReader<CursorMoved>,
    windows: Query<&Window>,
    render_targets: Query<(&ParentRenderTarget, &Children)>,
    nodes: Query<(&FlexNode, &CursorKind)>,
    mouse_state: Res<MouseState>,
) {
    let id = some!(reader.iter().last()).id;
    let window = windows.get(id).unwrap();
    let (_rt, children) = some!(render_targets.iter().find(|x| x.0 .0 == id));
    for child in children.iter() {
        let (node, kind) = ok_loop!(nodes.get(*child));
        let layout = flex.layout(node.0).unwrap();
        if pos_in_layout(mouse_state.render_target_pixel_pos, layout) {
            window.set_cursor_icon(kind.0);
        }
    }
}

#[derive(Component)]
struct DebugMesh(Entity);

fn add_debug_layout(
    res: Option<ResMut<DebugLayout>>,
    device: Res<Device>,
    mut buffers: ResMut<Buffers>,
    mut commands: Commands,
    rts: Query<(Entity, &FlexNode), With<RenderTarget>>,
) {
    let mut res = some!(res);
    if res.0 {
        return;
    }

    for (id, _node) in rts.iter() {
        let mut mesh_id = Entity::new(u32::MAX);
        commands
            .entity(id)
            .with_children(|parent| {
                parent
                    .spawn()
                    .with_children(|parent| {
                        parent
                            .spawn()
                            .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                            .with_children(|parent| {
                                mesh_id = parent
                                    .spawn()
                                    .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers, &device))
                                    .insert(PixelSpaceMesh { converted: false })
                                    .insert(LineStripMaterial)
                                    .insert(LineMaterialInPixelSpace(false))
                                    .insert(ZComponent(f32::MAX))
                                    .insert(Material::new(LineMaterialData {
                                        color: RGBA::new(1., 0., 1., 1.),
                                        width: Vec2::splat(2.),
                                    }))
                                    .id();
                            });
                    })
                    .insert(View);
            })
            .insert(DebugMesh(mesh_id));
    }
    res.0 = true;
}

fn debug_layout_system(
    res: Option<ResMut<DebugLayout>>,
    flex: NonSend<Stretch>,
    mut meshes: Query<&mut Mesh<Vec2>>,
    query: Query<(&FlexNode, &DebugMesh)>,
) {
    if res.is_none() {
        return;
    }
    for (node, mesh_id) in query.iter() {
        warn!("debug");
        let mut mesh = meshes.get_mut(mesh_id.0).unwrap();
        mesh.data.clear();
        debug(&flex, node.0, &mut mesh.data);
    }
}

fn remove_debug_layout(
    mut existed: Local<bool>,
    res: Option<ResMut<DebugLayout>>,
    mut commands: Commands,
    _flex: NonSend<Stretch>,
    _meshes: Query<&mut Mesh<Vec2>>,
    query: Query<(Entity, &DebugMesh)>,
    parents: Query<&Parent>,
) {
    match res {
        None => {
            if *existed {
                for (id, mesh_id) in query.iter() {
                    let group = parents.get(mesh_id.0).unwrap();
                    let view = parents.get(**group).unwrap();
                    commands.entity(**view).despawn_recursive();
                    commands.entity(id).remove::<DebugMesh>();
                }
                *existed = false;
            }
        }
        Some(_) => *existed = true,
    }
}

fn debug(flex: &Stretch, node: Node, mesh: &mut Vec<Vec2>) {
    let layout = flex.layout(node).unwrap();
    mesh.push(Vec2::new(layout.location.x, layout.location.y));
    mesh.push(Vec2::new(
        layout.location.x + layout.size.width,
        layout.location.y,
    ));
    mesh.push(Vec2::new(
        layout.location.x + layout.size.width,
        layout.location.y + layout.size.height,
    ));
    mesh.push(Vec2::new(
        layout.location.x,
        layout.location.y + layout.size.height,
    ));
    mesh.push(Vec2::new(layout.location.x, layout.location.y));
    for child in flex.children(node).unwrap() {
        debug(flex, child, mesh);
    }
}

pub trait PointIntoVec2 {
    fn into(&self) -> Vec2;
}

impl PointIntoVec2 for Point<f32> {
    fn into(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

#[derive(Component, Default)]
pub struct FlexStyle(pub Style);

#[derive(Component, Deref, DerefMut)]
pub struct FlexNode(pub Node);

#[derive(Component, Deref, DerefMut)]
pub struct ParentNode(pub Entity);

#[derive(SystemParam)]
pub struct Flex<'w, 's> {
    stretch: NonSend<'w, Stretch>,
    nodes: Query<'w, 's, &'static FlexNode>,
    parent_nodes: Query<'w, 's, &'static ParentNode>,
}

impl<'w, 's> Flex<'w, 's> {
    pub fn layout(&self, mut id: Entity) -> Option<self::Layout> {
        let node = self.nodes.get(id).ok()?;
        let layout = self.stretch.layout(node.0).unwrap();
        let mut result = self::Layout {
            location: Vec2::new(layout.location.x, layout.location.y),
            size: Vec2::new(layout.size.width, layout.size.height),
        };
        loop {
            id = ok_break!(self.parent_nodes.get(id)).0;
            let node = ok_break!(self.nodes.get(id));
            let layout = self.stretch.layout(node.0).unwrap();
            result.location.x += layout.location.x;
            result.location.y += layout.location.y;
        }
        Some(result)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub location: Vec2,
    pub size: Vec2,
}

impl Layout {
    pub fn contains(&self, pos: Vec2) -> bool {
        pos.x >= self.location.x
            && pos.x < self.location.x + self.size.x
            && pos.y >= self.location.y
            && pos.y < self.location.y + self.size.y
    }
}

//
//#[derive(Component)]
// pub struct LayoutInvalidated(pub bool);

pub struct LayoutChanged {
    id: Entity,
}

fn compute_layout(
    mut to_compute: Local<HashSet<Entity>>,
    mut reader: EventReader<ResizeRenderTarget>,
    parents: Query<&Parent>,
    //    mut writer: EventWriter<LayoutChanged>,
    nodes: Query<&FlexNode>,
    mut stretch: NonSendMut<Stretch>,
    changed: Query<(&FlexNode, &FlexStyle, Entity), Changed<FlexStyle>>,
    sizes: Query<&RenderTargetSize>,
) {
    for e in reader.iter() {
        to_compute.insert(e.id);
    }
    for (flex, style, id) in changed.iter() {
        stretch.set_style(flex.0, style.0.clone()).unwrap();
        let parent = find_root_parent(&parents, id);
        to_compute.insert(parent);
    }
    for id in to_compute.drain() {
        // window render target doesn't have flexbox
        let node = ok_loop!(nodes.get(id));
        let size = sizes.get(id).unwrap();
        stretch
            .compute_layout(
                node.0,
                stretch::geometry::Size {
                    width: Number::Defined(size.0.x as f32),
                    height: Number::Defined(size.0.y as f32),
                },
            )
            .unwrap();
        //        writer.send(LayoutChanged {
        //            id: e.id,
        //        });
        debug_layout_for(&stretch, node.0);
    }
}

pub fn debug_layout_for(stretch: &Stretch, node: Node) {
    debug_layout(stretch, node, 0);
}

fn debug_layout(stretch: &Stretch, node: Node, depth: usize) {
    debug!(
        "{:?}, depth: {}, node: {:?}",
        stretch.layout(node),
        depth,
        node
    );
    for child in stretch.children(node).unwrap() {
        debug_layout(stretch, child, depth + 1);
    }
}
