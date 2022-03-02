


use std::ops::Rem;

use bevy::prelude::*;
use float_pretty_print::PrettyPrintFloat;
use glyph_brush::{OwnedSection, OwnedText};
use rgb::RGBA;
use stretch::node::Node;
use stretch::style::Style;
use stretch::Stretch;
use wgpu::Device;
use wgpu_glyph::ab_glyph::PxScale;
use wgpu_glyph::{GlyphBrush, HorizontalAlign, VerticalAlign};
use winit::window::CursorIcon;

use crate::flex::{Flex, FlexNode, FlexStyle, FlexboxPlugin, Layout, ParentNode};
use crate::niobe::{Interactive, View};
use crate::render::pipelines::d2::line::{LineMaterial, LineMaterialData, LinePlugin};
use crate::render::utils::GroupMaterial;
use crate::render::{
    pixel_length_to_screen_space, pos_in_rect, Angle, Buffers, GlyphBrushes,
    Material, Mesh, MeshBundle, ParentRenderTarget, PixelSpaceMesh, RedrawRenderTarget,
    RenderPlugin, ScissorRect, TextBatch,
};
use crate::winit_plugin::{CursorKind, CursorMoved, MouseState, MouseWheel};
use crate::{colors, LineMaterialInPixelSpace, RenderTargetPos, RenderTargetSize};

pub struct ScalePlugin;

impl Plug for ScalePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
            .load(FlexboxPlugin)
            .load(RenderPlugin)
            .load(LinePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.add_system(scale_system.label(ScaleLabel))
            .added_system(scale_sync_world_groups)
            .add_system(scale_pan_system)
            .add_system(scale_zoom_system)
    }
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ScaleLabel;

#[derive(Component)]
struct InnerScaleMarkers;

#[derive(Component)]
pub struct ScaleMesh;

#[derive(Clone, Copy, Debug)]
pub enum ScaleKind {
    X,
    Y,
}

//#[derive(Component, Default)]
// pub struct ScaleMarkers(pub HashMap<TypeId, ScaleMarker>);

#[derive(Component)]
pub struct Scale {
    pub group: Entity,
    pub show_grid: bool,
    pub direction: Direction,
    pub kind: ScaleKind,
    pub marker_parent: Entity,
}

//#[derive(Component)]
// pub enum MarkerPosition {
//    WorldSpace(f32),
//    PixelSpace(f32),
//}
// pub struct RangeMarkerBundle<T> {
//    pos: MarkerPosition,
//    #[bundle]
//    mesh_bundle: MeshBundle<Vec2>,
//    material: Material<T>,
//}

fn scale_sync_world_groups(
    mut changed_scales: Local<Vec<Entity>>,
    scales: Query<(&Scale, Entity)>,
    mut groups: Query<(Entity, &mut Material<GroupMaterial>)>,
) {
    for (id, _) in groups.iter_mut() {
        // parallel coordinates changes group material without change detection
        // so we must copy when they are different manually
        //        if !material.is_changed() {
        //            continue;
        //        }

        let id = some_loop!(scales.iter().find(|x| x.0.group == id).map(|x| x.1));
        changed_scales.push(id);
    }
    for scale_id in changed_scales.drain(..) {
        let (scale, _) = scales.get(scale_id).unwrap();
        let new_data = groups.get_mut(scale.group).unwrap().1.data.clone();
        let (_, mut group) = groups.get_mut(scale.marker_parent).unwrap();
        match scale.kind {
            ScaleKind::X => {
                if group.data.translate.x != new_data.translate.x {
                    group.data.translate.x = new_data.translate.x;
                }
                if group.data.scale().x != new_data.scale().x {
                    let y_scale = group.data.scale().y;
                    group.data.set_scale(Vec2::new(new_data.scale().x, y_scale));
                }
            }
            ScaleKind::Y => {
                if group.data.translate.y != new_data.translate.y {
                    group.data.translate.y = new_data.translate.y;
                }
                if group.data.scale().y != new_data.scale().y {
                    let x_scale = group.data.scale().x;
                    group.data.set_scale(Vec2::new(x_scale, new_data.scale().y));
                }
            }
        }
    }
}

pub struct ScaleResult {
    pub scale_view_id: Entity,
    pub marker_parent_id: Entity,
}

impl Scale {
    pub fn spawn(
        commands: &mut Commands,
        style: Style,
        stretch: &mut Stretch,
        kind: ScaleKind,
        group: Entity,
        buffers: &mut Buffers,
        device: &Device,
        render_target: Entity,
        parent_node: Node,
        parent_node_id: Entity,
        show_grid: bool,
        direction: Direction,
        angle: f32,
    ) -> ScaleResult {
        let node = stretch.new_node(style, vec![]).unwrap();
        let mut view_id = Entity::new(u32::MAX);
        let mut marker_parent = Entity::new(u32::MAX);
        commands.entity(render_target).with_children(|parent| {
            view_id = parent
                .spawn()
                .insert(CursorKind(CursorIcon::Move))
                .insert(View)
                .insert(FlexStyle(style))
                .insert(FlexNode(node))
                .insert(ParentNode(parent_node_id))
                .with_children(|parent| {
                    marker_parent = parent
                        .spawn()
                        .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                        .id();
                    parent
                        .spawn()
                        .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                        .with_children(|parent| {
                            parent
                                .spawn()
                                .insert(ScaleMesh)
                                .insert_bundle(MeshBundle::<Vec2>::empty(buffers, &device))
                                .insert(PixelSpaceMesh { converted: false })
                                .insert(LineMaterial)
                                .insert(LineMaterialInPixelSpace(false))
                                .insert(Material::new(LineMaterialData {
                                    color: RGBA::new(0., 0., 0., 1.),
                                    width: Vec2::splat(2.),
                                }));
                            //            parent
                            //                .spawn()
                            //                .insert(InnerScaleMarkers)
                            //                .insert(TextBatch(Vec::new()))
                            //                .insert(PixelSpaceMesh { converted: false })
                            //                .insert(Ignore(false))
                            //                .insert_bundle(MeshBundle::<Vec2>::empty(&mut buffers,
                            // &device))
                            // .insert(Material::new(QuadMaterialData {
                            // color: RGBA::new(1., 0., 0., 1.),                }));
                        })
                        //                        .insert(ScaleMarkers::default())
                        .insert(Scale {
                            group,
                            show_grid,
                            direction,
                            kind,
                            marker_parent,
                        })
                        .insert(TextBatch(Vec::new()));
                })
                .id();
        });
        stretch.add_child(parent_node, node).unwrap();
        if angle != 0. {
            commands.entity(view_id).insert(Angle(angle));
        }
        ScaleResult {
            scale_view_id: view_id,
            marker_parent_id: marker_parent,
        }
    }
}

fn scale_system(
    flex: Flex,
    mut reader: UniqueEventReader<RedrawRenderTarget>,
    children_query: Query<&Children>,
    mut views: Query<(Entity, &Children, Option<&mut ScissorRect>), With<View>>,
    mut meshes: Query<&mut Mesh<Vec2>>,
    mut scales: Query<(Entity, &Scale, &mut TextBatch)>,
    _marker_sections: Query<&mut TextBatch, Without<Scale>>,
    render_targets: Query<&RenderTargetSize>,
    groups: Query<(&Material<GroupMaterial>, &Parent)>,
    mut glyph_brushes: ResMut<GlyphBrushes>,
) {
    let glyph_brush = glyph_brushes.0.values_mut().next().unwrap();
    for e in reader.iter() {
        let children = ok_loop!(children_query.get(e.id));
        for child in children.iter() {
            let group_views = unsafe {
                &mut *(&views as *const _
                    as *mut Query<
                        '_,
                        '_,
                        (Entity, &Children, Option<&mut ScissorRect>),
                        With<View>,
                    >)
            };
            let (id, children, mut scissor) = ok_loop!(views.get_mut(*child));
            for child in children.iter() {
                let (entity, scale, mut sections) = ok_loop!(scales.get_mut(*child));
                let index = scale.kind as usize;
                let layout = flex.layout(id).unwrap();
                let (group, parent) = groups.get(scale.group).unwrap();
                if group.data.scale()[index].is_nan() || group.data.scale()[index] == 0. {
                    warn!("Invalid scale");
                    continue;
                }

                let group_view_id = group_views.get_mut(**parent).unwrap().0;
                if let Some(scissor) = scissor.as_mut() {
                    let group_layout = flex.layout(group_view_id).unwrap();
                    scissor.x = group_layout.location.x.min(layout.location.x) as u32;
                    scissor.y = group_layout.location.y.min(layout.location.y) as u32;
                    let layout_bot_right = Vec2::new(
                        layout.location.x + layout.size.x,
                        layout.location.y + layout.size.y,
                    );
                    let group_bot_right = Vec2::new(
                        group_layout.location.x + group_layout.size.x,
                        group_layout.location.y + group_layout.size.y,
                    );
                    scissor.w = group_bot_right.x.max(layout_bot_right.x) as u32 - scissor.x;
                    scissor.h = group_bot_right.y.max(layout_bot_right.y) as u32 - scissor.y;
                }
                let render_target_size = render_targets.get(e.id).unwrap().0;
                let pixel_scale = pixel_length_to_screen_space(Vec2::ONE, render_target_size);
                match scale.kind {
                    ScaleKind::X => {
                        build_scale_x(
                            scale,
                            entity,
                            layout,
                            pixel_scale,
                            &group.data,
                            &children_query,
                            &mut meshes,
                            &mut sections.0,
                        );
                    }
                    ScaleKind::Y => {
                        build_scale_y(
                            scale,
                            render_target_size,
                            entity,
                            layout,
                            pixel_scale,
                            &group.data,
                            &children_query,
                            &mut meshes,
                            &mut sections.0,
                            glyph_brush,
                        );
                    }
                }
            }
        }
    }
}

fn build_scale_x(
    scale: &Scale,
    scale_entity: Entity,
    layout: Layout,
    pixel_scale: Vec2,
    group: &GroupMaterial,
    children_query: &Query<&Children>,
    meshes: &mut Query<&mut Mesh<Vec2>>,
    sections: &mut Vec<OwnedSection>,
) {
    let index = 0;
    let offset = layout.location.x * pixel_scale[index];
    let start = (-1. - group.translate[index] + offset) / group.scale()[index];
    let log = 10.0f32.powf(-group.scale()[index].log10().floor()) / 10.;
    let tick_spacing = log * group.scale()[index];
    let mut x_value = if start > 0. {
        start - start.rem(log) + log
    } else {
        start - start.rem(log)
    };
    let mut text_screen_pos = x_value * group.scale()[index] + group.translate[index] + 0.;

    for child in children_query.get(scale_entity).unwrap().iter() {
        if let Ok(mut mesh) = meshes.get_mut(*child) {
            let mut n_sections = 0;
            mesh.clear();
            mesh.push(Vec2::new(layout.location.x, layout.location.y));
            mesh.push(Vec2::new(
                layout.location.x + layout.size.x,
                layout.location.y,
            ));
            loop {
                if n_sections >= sections.len() {
                    sections.push(OwnedSection {
                        screen_position: (0.0, 0.0),
                        layout: glyph_brush::Layout::default_single_line()
                            .h_align(HorizontalAlign::Center),
                        text: vec![OwnedText::new("".to_string())
                            .with_scale(PxScale { x: 17.0, y: 17.0 })
                            .with_color(colors::BLACK)],
                        ..Default::default()
                    });
                }
                let mut section = &mut sections[n_sections];
                section.screen_position =
                    ((text_screen_pos + 1.) / pixel_scale.x, layout.location.y);
                let end = if scale.show_grid {
                    0.
                } else {
                    layout.location.y
                };
                mesh.push(Vec2::new(
                    section.screen_position.0 as f32,
                    layout.location.y + 5.,
                ));
                mesh.push(Vec2::new(section.screen_position.0 as f32, end));
                sections[n_sections].text[0].text.clear();
                use std::fmt::Write;
                write!(
                    &mut sections[n_sections].text[0].text,
                    "{:5.5}",
                    PrettyPrintFloat(x_value as f64)
                )
                .unwrap();
                n_sections += 1;
                x_value += log;
                text_screen_pos += tick_spacing;
                if text_screen_pos > 1. {
                    break;
                }
            }
            sections.truncate(n_sections);
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Normal,
    Reversed,
}

impl Direction {
    pub fn is_reversed(&self) -> bool {
        match self {
            Direction::Normal => false,
            Direction::Reversed => true,
        }
    }
}

fn build_scale_y(
    scale: &Scale,
    render_target_size: UVec2,
    scale_entity: Entity,
    layout: Layout,
    pixel_scale: Vec2,
    group: &GroupMaterial,
    children_query: &Query<&Children>,
    meshes: &mut Query<&mut Mesh<Vec2>>,
    sections: &mut Vec<OwnedSection>,
    _glyph_brush: &mut GlyphBrush<()>,
) {
    let mut direction_offset = layout.size.x;
    let mut tick_size = 5.;
    if scale.direction.is_reversed() {
        direction_offset = 0.;
        tick_size = -tick_size;
    }
    let index = 1;
    let offset =
        (render_target_size.y as f32 - layout.location.y - layout.size.y) * pixel_scale[index];
    let build_section = |sections: &mut Vec<OwnedSection>, i: usize, screen_pos: f32| {
        if i >= sections.len() {
            let layout = glyph_brush::Layout::default_single_line().v_align(VerticalAlign::Center);
            let layout = if scale.direction.is_reversed() {
                layout.h_align(HorizontalAlign::Left)
            } else {
                layout.h_align(HorizontalAlign::Right)
            };
            sections.push(OwnedSection {
                screen_position: (0.0, 0.0),
                layout,
                text: vec![OwnedText::new("".to_string())
                    .with_scale(PxScale { x: 14.0, y: 14.0 })
                    .with_color(colors::BLACK)],
                ..Default::default()
            });
        }
        let section = &mut sections[i];
        section.screen_position = (
            if scale.direction.is_reversed() {
                layout.location.x + tick_size
            } else {
                layout.location.x + layout.size.x - tick_size
            },
            layout.size.y - (screen_pos + 1.) / pixel_scale.y,
        );
    };
    let value_to_screen_pos =
        |value: f32| -> f32 { value * group.scale()[index] + group.translate[index] - offset };
    let start = (-1. - group.translate[index] + offset) / group.scale()[index];
    let log = 10.0f32.powf(-group.scale()[index].log10().floor()) / 10.;
    let tick_spacing = log * group.scale()[index];
    let mut y_value = if start > 0. {
        start - start.rem(log) + log
    } else {
        start - start.rem(log)
    };
    let mut text_screen_pos = value_to_screen_pos(y_value);
    for child in children_query.get(scale_entity).unwrap().iter() {
        //        if let Ok(sections) = marker_sections.get_mut(*child) {
        //            let mut mesh = meshes.get_mut(*child).unwrap();
        //            mesh.clear();
        //            let sections: &mut Vec<OwnedSection> = &mut sections.0;
        //            for (i, marker) in markers.values().enumerate() {
        //                let mut text_screen_pos = match marker.value {
        //                    MarkerValue::WorldSpace(x) => value_to_screen_pos(x),
        //                    MarkerValue::PixelSpace(x) => x,
        //                };
        //                build_section(sections, n_sections, text_screen_pos);
        //                sections[i].text[0].text = marker.text.clone();
        //                if marker.background_color.a != 0. {
        //                    let bounds = glyph_brush.glyph_bounds(&sections[i]).unwrap();
        //                    Rect::from_min_max(
        //                        Vec2::new(bounds.min.x, bounds.min.y),
        //                        Vec2::new(bounds.max.x, bounds.max.y),
        //                    )
        //                    .to_mesh(&mut *mesh);
        //                }
        //            }
        //            sections.truncate(markers.len());
        //        } else if
        if let Ok(mut mesh) = meshes.get_mut(*child) {
            let mut n_sections = 0;
            mesh.clear();

            mesh.push(Vec2::new(
                layout.location.x + direction_offset,
                layout.location.y,
            ));
            mesh.push(Vec2::new(
                layout.location.x + direction_offset,
                layout.location.y + layout.size.y,
            ));
            loop {
                build_section(sections, n_sections, text_screen_pos);
                mesh.push(Vec2::new(
                    layout.location.x + direction_offset - tick_size,
                    sections[n_sections].screen_position.1,
                ));
                let end = if scale.show_grid {
                    render_target_size.x as f32
                } else {
                    layout.location.x + direction_offset
                };

                mesh.push(Vec2::new(end, sections[n_sections].screen_position.1));
                sections[n_sections].text[0].text.clear();
                use std::fmt::Write;
                write!(
                    &mut sections[n_sections].text[0].text,
                    "{:5.5}",
                    PrettyPrintFloat(y_value as f64)
                )
                .unwrap();
                n_sections += 1;
                y_value += log;
                text_screen_pos += tick_spacing;
                if text_screen_pos > 1. {
                    break;
                }
            }
            sections.truncate(n_sections);
        }
    }
}

fn scale_pan_system(
    flex: Flex,
    mut events: Local<Vec<CursorMoved>>,
    mut cursor_moved: EventReader<CursorMoved>,
    views: Query<(Entity, &Children), With<View>>,
    scales: Query<&Scale>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mouse_state: Res<MouseState>,
    render_targets: Query<(
        &ParentRenderTarget,
        &RenderTargetPos,
        &RenderTargetSize,
        &Children,
    )>,
    mut groups: Query<(&Interactive, &mut Material<GroupMaterial>)>,
) {
    if !mouse_state.left_hold {
        return;
    }
    events.clear();
    events.extend(cursor_moved.iter().cloned());
    if events.is_empty() {
        return;
    }
    let mut delta_px = Vec2::ZERO;
    let e = events.get(0).unwrap();
    let id = e.id;
    let (_, _, size, children) = some!(render_targets
        .iter()
        .find(|x| x.0 .0 == e.id && pos_in_rect(e.position, x.1 .0.as_vec2(), x.2 .0.as_vec2())));
    for child in children.iter() {
        let (id, children) = ok_loop!(views.get(*child));
        let layout = some_loop!(flex.layout(id));
        if !layout.contains(e.position) {
            continue;
        }
        for e in &*events {
            if layout.contains(e.position) {
                delta_px += e.delta;
            }
        }
        let delta = pixel_length_to_screen_space(delta_px, size.0);
        for child in children.iter() {
            let scale = ok_loop!(scales.get(*child));
            let mut group = ok_loop!(groups.get_mut(scale.group));
            if !group.0 .0 {
                break;
            }
            // don't know why we need to divide by 2
            match scale.kind {
                ScaleKind::X => {
                    group.1.data.translate.x += delta.x / 2.;
                }
                ScaleKind::Y => {
                    group.1.data.translate.y -= delta.y / 2.;
                }
            }
            break;
        }
    }
    writer.send(RedrawRenderTarget::new(id));
}

fn scale_zoom_system(
    flex: Flex,
    mut mouse_wheel: EventReader<MouseWheel>,
    views: Query<(Entity, &Children), With<View>>,
    mouse_state: Res<MouseState>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    render_targets: Query<(
        &ParentRenderTarget,
        &RenderTargetPos,
        &RenderTargetSize,
        &Children,
    )>,
    mut groups: Query<(&Interactive, &mut Material<GroupMaterial>)>,
    scales: Query<&Scale>,
) {
    let id = some!(mouse_state.window_id);
    let mut delta = Vec2::ZERO;
    for e in mouse_wheel.iter() {
        delta += e.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    let (_, _, _, children) = some!(render_targets.iter().find(|x| x.0 .0 == id
        && pos_in_rect(
            mouse_state.render_target_pixel_pos,
            x.1 .0.as_vec2(),
            x.2 .0.as_vec2(),
        )));
    let mut scale_changed = false;
    for child in children.iter() {
        let (view_id, children) = ok_loop!(views.get(*child));
        let layout = some_loop!(flex.layout(view_id));
        if !layout.contains(mouse_state.render_target_pixel_pos) {
            continue;
        }
        // if events would be stockpiled we could flip axis
        let mult = (1. + delta * 0.1).abs();
        for child in children.iter() {
            let scale = ok_loop!(scales.get(*child));
            let mut group = match groups.get_mut(scale.group) {
                Ok(x) if x.0 .0 => x.1,
                _ => continue,
            };
            let scale = group.data.scale() * mult;
            group.data.set_scale(scale);
            scale_changed = true;
        }
    }
    if scale_changed {
        writer.send(RedrawRenderTarget::new(id));
    }
}
