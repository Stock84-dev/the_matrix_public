pub mod big_data;

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use glyph_brush::OwnedSection;
use imgui::TextureId;
use rgb::RGBA;
use stretch::style::{Dimension, FlexDirection, FlexWrap, Style};
use stretch::Stretch;
use wgpu::{Device, LoadOp, Operations, Texture};

use crate::flex::{FlexNode, FlexStyle, FlexboxPlugin};
use crate::imgui_plugin::ImguiPlugin;
use crate::render::pipelines::d2::line::{LineMaterial, LineMaterialData, LinePlugin};
use crate::render::pipelines::d2::line_strip::LineStripPlugin;
use crate::render::utils::GroupMaterial;
use crate::render::{
    pixel_length_to_screen_space, pos_in_layout, upload_mesh, BufferId, Buffers,
    ChildRenderTargetBundle, Material, MaterialLayout, Mesh, MeshBundle, PixelSpaceMesh,
    RedrawRenderTarget, RenderTarget, RenderTargetBundle, ScissorRect, TextBatch,
};
use crate::scale::{Direction, Scale, ScaleKind, ScaleLabel, ScaleMesh};
use crate::winit_plugin::{CursorMoved, MouseState, MouseWheel, Window};
use crate::{LineMaterialInPixelSpace, RenderTargetPos, RenderTargetSize};

pub struct NiobePlugin;

impl Plug for NiobePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(LinePlugin);
        loader.load(FlexboxPlugin);
        loader.load(LineStripPlugin).load(ImguiPlugin) //.load(OutputPlotConfig)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        //        big_data::startup(app);
        //        app.add_plugin(LineStripPlugin)
        app
            //            .add_system(redraw)
            //            .add_system_to_stage(CoreStage::PreUpdate, add_flex.system())
            .add_startup_system(startup_niobe)
            .render_system(upload_mesh::<Vec2>)
            .input_system(pan_system.before(ScaleLabel))
            .input_system(crosshair_system.before(ScaleLabel))
            .input_system(zoom_system.before(ScaleLabel))
        // TODO: if another plugin loads this it will clear previous value
    }
}

#[derive(Component)]
pub struct TextureIdComponent(pub TextureId);

fn startup_niobe(
    device: Res<Device>,
    mut buffers: ResMut<Buffers>,
    mut ui_renderer: ResMut<imgui_wgpu::Renderer>,
    mut commands: Commands,
    mut stretch: NonSendMut<Stretch>,
    windows: Query<(&Window, Entity, &RenderTarget)>,
) {
    let (_window, window_entity, rt) = windows.iter().next().unwrap();
    let max_x = 7.;
    let n_points = 64;
    let points: Vec<_> = (0..n_points)
        .into_iter()
        .map(|x| {
            let sinx = x as f32 / n_points as f32 * max_x;
            let y = (sinx).sin();
            let x = (x as f32 / n_points as f32 - 0.5) * 2.;
            Vec2::new(x, y)
        })
        .collect();
    // y scale -> group
    let render_target_style = Style {
        flex_direction: FlexDirection::Column,
        size: stretch::geometry::Size {
            width: Dimension::Percent(1.),
            height: Dimension::Percent(1.),
        },
        flex_wrap: FlexWrap::Wrap,
        ..Default::default()
    };
    let main_row_style = Style {
        flex_direction: FlexDirection::Row,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        ..Default::default()
    };
    let view_style = Style {
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::Points(0.),
        margin: stretch::geometry::Rect {
            start: Dimension::Points(5.),
            end: Dimension::Points(5.),
            top: Dimension::Points(5.),
            bottom: Dimension::Points(5.),
        },
        size: stretch::geometry::Size {
            width: Dimension::Auto,
            height: Dimension::Auto,
        },
        ..Default::default()
    };
    let x_scale_height = 15.;
    let y_scale_width = 50.;
    let x_scale_style = Style {
        //        flex_grow: 1.0,
        //        flex_shrink: 1.0,
        //        flex_basis: Dimension::Auto,
        size: stretch::geometry::Size {
            width: Dimension::Auto,
            height: Dimension::Points(x_scale_height),
        },
        margin: stretch::geometry::Rect {
            start: Dimension::Points(y_scale_width),
            end: Dimension::Points(0.),
            top: Dimension::Undefined,
            bottom: Dimension::Undefined,
        },
        ..Default::default()
    };
    let y_scale_style = Style {
        size: stretch::geometry::Size {
            width: Dimension::Points(y_scale_width),
            height: Dimension::Auto,
        },
        ..Default::default()
    };
    let view_node = stretch.new_node(view_style, vec![]).unwrap();
    let y_scale_view_node = stretch.new_node(y_scale_style, vec![]).unwrap();
    let x_scale_view_node = stretch.new_node(x_scale_style, vec![]).unwrap();
    let main_row_node = stretch
        .new_node(main_row_style, vec![y_scale_view_node, view_node])
        .unwrap();
    let mut group_id = Entity::new(u32::MAX);
    let render_target_node = stretch
        .new_node(render_target_style, vec![main_row_node, x_scale_view_node])
        .unwrap();
    let view_entity = commands
        .spawn()
        .insert(View)
        .insert(FlexStyle(view_style))
        .insert(FlexNode(view_node))
        .with_children(|parent| {
            group_id = parent
                .spawn()
                .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                .insert(Interactive(true))
                .with_children(|parent| {
                    parent
                        .spawn()
                        .insert(Material::new(LineMaterialData {
                            color: RGBA::new(1., 0., 0., 1.),
                            width: Vec2::splat(1.),
                        }))
                        .insert(LineMaterialInPixelSpace(false))
                        .insert(LineMaterial)
                        .insert_bundle(MeshBundle::new(points, &mut buffers, &device));
                })
                .id();
            debug!("{:#?}", group_id);
            let id = parent
                .spawn()
                .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                .with_children(|parent| {
                    parent
                        .spawn()
                        .insert(Crosshair)
                        .insert(PixelSpaceMesh { converted: false })
                        .insert(LineMaterialInPixelSpace(false))
                        .insert(LineMaterial)
                        .insert(Material::new(LineMaterialData {
                            color: RGBA::new(0., 1., 0., 1.),
                            width: Vec2::splat(1.),
                        }))
                        .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers, &device));
                })
                .id();
            debug!("{:#?}", id);
        })
        .id();
    let x_scale = commands
        .spawn()
        .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
        .with_children(|parent| {
            parent
                .spawn()
                .insert(ScaleMesh)
                .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers, &device))
                .insert(PixelSpaceMesh { converted: false })
                .insert(LineMaterial)
                .insert(LineMaterialInPixelSpace(false))
                .insert(Material::new(LineMaterialData {
                    color: RGBA::new(1., 0., 0., 1.),
                    width: Vec2::splat(1.),
                }));
            //            parent
            //                .spawn()
            //                .insert(InnerScaleMarkers)
            //                .insert(TextBatch(Vec::new()))
            //                .insert(PixelSpaceMesh { converted: false })
            //                .insert(Ignore(false))
            //                .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers, &device))
            //                .insert(Material::new(QuadMaterialData {
            //                    color: RGBA::new(1., 0., 0., 1.),
            //                }));
        })
        //        .insert(ScaleMarkers::default())
        .insert(Scale {
            group: group_id,
            show_grid: true,
            direction: Direction::Normal,
            kind: ScaleKind::X,
            marker_parent: Entity::new(u32::MAX),
        })
        .insert(TextBatch(Vec::new()))
        .id();
    let y_scale = commands
        .spawn()
        .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
        .with_children(|parent| {
            parent
                .spawn()
                .insert(ScaleMesh)
                .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers, &device))
                .insert(PixelSpaceMesh { converted: false })
                .insert(LineMaterial)
                .insert(LineMaterialInPixelSpace(false))
                .insert(Material::new(LineMaterialData {
                    color: RGBA::new(1., 0., 0., 1.),
                    width: Vec2::splat(1.),
                }));
        })
        //        .insert(ScaleMarkers::default())
        .insert(Scale {
            group: group_id,
            show_grid: true,
            direction: Direction::Normal,
            kind: ScaleKind::Y,
            marker_parent: Entity::new(u32::MAX),
        })
        .insert(TextBatch(Vec::new()))
        .id();
    let x_scale_view = commands
        .spawn()
        .insert(View)
        .insert(ScissorRect::default())
        .insert(FlexStyle(x_scale_style))
        .insert(FlexNode(x_scale_view_node))
        .push_children(&[x_scale])
        .id();
    let y_scale_view = commands
        .spawn()
        .insert(View)
        .insert(ScissorRect::default())
        .insert(FlexStyle(y_scale_style))
        .insert(FlexNode(y_scale_view_node))
        .push_children(&[y_scale])
        .id();
    let render_target_id = commands
        .spawn()
        .insert_bundle(ChildRenderTargetBundle::new(
            RenderTargetBundle::new(
                rt.format,
                Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.3,
                        g: 0.2,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: true,
                },
                UVec2::new(800, 600),
            ),
            UVec2::ZERO,
            window_entity,
            &device,
            &mut *ui_renderer,
        ))
        .insert(FlexStyle(render_target_style))
        .insert(FlexNode(render_target_node))
        .push_children(&[view_entity, x_scale_view, y_scale_view])
        //        .push_children(&[view_entity])
        .id();
    debug!("redraw {:#?}", render_target_id);
    commands.insert_resource(render_target_id);
}

// fn first_child_in<Q: WorldQuery>(
//    children_query: Query<&Children>,
//    query: &Q,
//) -> Option<<Q::Fetch as Fetch>::Item>{
//    for child in children_query.get(entity).unwrap().iter() {
//        if let Ok(mut mesh) = query.get(*child) {}
//    }
//}
fn zoom_system(
    mut mouse_wheel: EventReader<MouseWheel>,
    views: Query<(&FlexNode, &Children), With<View>>,
    mouse_state: Res<MouseState>,
    stretch: NonSendMut<Stretch>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    render_targets: Query<(&RenderTargetSize, &RenderTargetPos, &Children)>,
    mut groups: Query<(&Interactive, &mut Material<GroupMaterial>)>,
) {
    let id = some!(mouse_state.window_id);
    let mut delta = Vec2::ZERO;
    let (_size, _pos, children) = ok!(render_targets.get(id));
    trace!("executed");
    for child in children.iter() {
        let (node, children) = ok_loop!(views.get(*child));
        let layout = stretch.layout(node.0).unwrap();
        if !pos_in_layout(mouse_state.window_pixel_pos, layout) {
            continue;
        }
        for e in mouse_wheel.iter() {
            delta += e.delta;
        }
        // if events would be stockpiled we could flip axis
        let mult = (1. + delta * 0.1).abs();
        debug!("multiplier {:#?}", mult);
        for child in children.iter() {
            let mut group = match groups.get_mut(*child) {
                Ok(x) if x.0 .0 => x.1,
                _ => continue,
            };
            let scale = group.data.scale() * mult;
            group.data.set_scale(scale);
        }
        writer.send(RedrawRenderTarget::new(id));
        break;
    }
}

fn crosshair_system(
    mut cursor_moved: EventReader<CursorMoved>,
    views: Query<(&FlexNode, &Children), With<View>>,
    mouse_state: Res<MouseState>,
    stretch: NonSendMut<Stretch>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mut crosshairs: Query<&mut Mesh<Vec2>, With<Crosshair>>,
    render_targets: Query<(&RenderTargetSize, &RenderTargetPos, &Children)>,
    groups: Query<&Children, With<Material<GroupMaterial>>>,
) {
    let e = some!(cursor_moved.iter().next());
    let start_pos = e.position;
    let id = e.id;
    let (size, _pos, child_views) = ok!(render_targets.get(e.id));
    for child in child_views.iter() {
        let (node, child_groups) = ok_loop!(views.get(*child));
        let layout = stretch.layout(node.0).unwrap();
        if !pos_in_layout(start_pos, layout) {
            continue;
        }
        for child in child_groups.iter() {
            let children = ok_loop!(groups.get(*child));
            for child in children.iter() {
                debug!("{:#?}", child);
                let mut mesh = ok_loop!(crosshairs.get_mut(*child));
                error!("crosshair");
                mesh.clear();
                mesh.push(Vec2::new(mouse_state.render_target_pixel_pos.x, 0.));
                mesh.push(Vec2::new(
                    mouse_state.render_target_pixel_pos.x,
                    size.y as f32,
                ));
                mesh.push(Vec2::new(0., mouse_state.render_target_pixel_pos.y));
                mesh.push(Vec2::new(
                    size.x as f32,
                    mouse_state.render_target_pixel_pos.y,
                ));
                writer.send(RedrawRenderTarget::new(id));
                return;
            }
        }
    }
}

fn pan_system(
    mut cursor_moved: EventReader<CursorMoved>,
    views: Query<(&FlexNode, &Children), With<View>>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mouse_state: Res<MouseState>,
    stretch: NonSendMut<Stretch>,
    render_targets: Query<(&RenderTargetSize, &RenderTargetPos, &Children)>,
    mut groups: Query<(&Interactive, &mut Material<GroupMaterial>)>,
) {
    if !mouse_state.left_hold {
        return;
    }
    let mut delta_px = Vec2::ZERO;
    let mut iter = cursor_moved.iter().peekable();
    let e = some!(iter.peek());
    let id = e.id;
    let (size, _pos, children) = ok!(render_targets.get(e.id));
    for child in children.iter() {
        let (node, children) = ok_loop!(views.get(*child));
        let layout = stretch.layout(node.0).unwrap();
        if !pos_in_layout(e.position, layout) {
            continue;
        }
        for e in iter {
            if pos_in_layout(e.position, layout) {
                delta_px += e.delta;
            }
        }
        let delta = pixel_length_to_screen_space(delta_px, size.0);
        for child in children.iter() {
            let mut group = match groups.get_mut(*child) {
                Ok(x) if x.0 .0 => x.1,
                _ => continue,
            };
            group.data.translate.x += delta.x;
            group.data.translate.y -= delta.y;
        }
        writer.send(RedrawRenderTarget::new(id));
        break;
    }
}

// pub struct ScaleBundle {
//    view: Parent,
//    flex: Flex,
//    children: Children,
//}

#[derive(Component)]
pub struct OwnedRenderTarget {
    texture: Texture,
}

#[derive(Component, Clone)]
pub struct Text {
    pub section: OwnedSection,
}

#[derive(Component)]
pub struct RenderData {
    pub buffer: BufferId,
    pub data: Arc<Mutex<Vec<f32>>>,
}

#[derive(Component)]
pub struct ZComponent(pub f32);

#[derive(Component)]
pub struct View;

#[derive(Component)]
pub struct Interactive(pub bool);

impl MaterialLayout for GroupMaterial {}

fn pixel_scale(size: UVec2) -> Vec2 {
    Vec2::ONE / size.as_vec2()
}

#[derive(Component)]
pub struct Crosshair;
