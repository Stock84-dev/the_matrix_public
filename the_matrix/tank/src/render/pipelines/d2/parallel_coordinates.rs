use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Instant;

use bevy::ecs::change_detection::Mut;
use bevy::prelude::*;
use glyph_brush::{OwnedSection, OwnedText};
use merovingian::structs::RangeInclusive;
use mouse::mem::{Arena, Const};
use mouse::minipre::PreprocessorContext;
use rgb::RGBA;
use stretch::node::Node;
use stretch::number::Number;
use stretch::style::{AlignItems, Dimension, Style};
use stretch::Stretch;
use wgpu::{
    BlendState, BufferAddress, CommandEncoderDescriptor, Device, LoadOp, Operations,
    PrimitiveTopology, Queue, RenderPassColorAttachment, RenderPassDescriptor, TextureFormat,
    VertexFormat,
};
use wgpu_glyph::ab_glyph::PxScale;
use wgpu_glyph::{GlyphCruncher, HorizontalAlign, VerticalAlign};
use winit::event::{ElementState, MouseButton};

use crate::flex::{debug_layout_for, Flex, FlexNode, FlexStyle};
use crate::niobe::{Interactive, Text, View, ZComponent};
use crate::render::pipelines::d2::line::{LineMaterialData, LinePlugin, SampledLineMaterial};
use crate::render::pipelines::d2::line_strip::LineStripMaterial;
use crate::render::shaders::fragment::ColorForwardFrag;
use crate::render::shaders::vertex::ParallelCoordinatesVert;
use crate::render::shaders::Shader;
use crate::render::utils::{GroupMaterial, PipelineBuilder, VertBufferLayout};
use crate::render::{
    material_added, pixel_to_screen_space, pos_in_rect, Angle, AsBytes, BindGroupArena, Buffers,
    GlyphBrushes, GpuMesh, Material, MaterialLayout, Mesh, MeshBundle, MyBuffer,
    ParentRenderTarget, PreferredSurfaceFormat, RedrawRenderTarget, RenderPipelines, RenderTexture,
    ResizeRenderTarget, SampledMaterial,
};
use crate::scale::{Direction, Scale, ScaleKind, ScalePlugin};
use crate::winit_plugin::{CursorMoved, MouseInput, MouseState};
use crate::{
    colors, LineMaterialInPixelSpace, MutExt, RenderTargetPos, RenderTargetSize, TypeIdLabel,
};

pub struct ParallelCoordinatesPlugin;

impl Plug for ParallelCoordinatesPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
            .load(PipelinePlugin)
            .load(LinePlugin)
            .load(ScalePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.add_pipeline::<FilterParallelCoordinates>()
            .add_pipeline::<DrawParallelCoordinates>()
            .added_system(
                material_added::<ParallelCoordinates>
                    .label(TypeIdLabel::new::<ParallelCoordinates>()),
            )
            .added_system(
                parallel_coordinates_material_changed
                    .after(TypeIdLabel::new::<ParallelCoordinates>()),
            )
            .added_system(parallel_coordinates_change_mesh)
            .post_system(parallel_coordinates_configure_texture)
            // must happen after configure
            .render_system(parallel_coordinates_clear)
            //            .pre_system(sync_render_target_size_a.label(SyncRenderTargetSizeLabel))
            //            .pre_system(sync_render_target_size_b.after(SyncRenderTargetSizeLabel))
            .add_system_set(
                SystemSet::new()
                    .label(ParallelCoordinatesLabel)
                    .with_system(parallel_coordinates_update_scale_layouts.after(ReorderLabel))
                    .with_system(parallel_coordinates_update_groups.after(ReorderLabel))
                    .with_system(parallel_coordinates_update_material)
                    .with_system(parallel_coordinates_filter)
                    .with_system(parallel_coordinates_handle_reorder.label(ReorderLabel))
                    .with_system(parallel_coordinates_filter_handle_input)
                    // on update to get more parallelism + it needs to run before main draw to draw
                    // updated texture
                    .with_system(parallel_coordinates_draw),
            )
    }
}

impl ParallelCoordinatesPlugin {
    pub fn spawn(
        scale_names: &[String],
        buffers: &mut Buffers,
        device: &Device,
        format: TextureFormat,
        render_target: Entity,
        parent_node: Node,
        stretch: &mut Stretch,
        commands: &mut Commands,
    ) -> Entity {
        let n_props = scale_names.len();
        let mut scales = Vec::new();
        let mut parallel_coordinates = Entity::new(u32::MAX);
        let mut view = Entity::new(u32::MAX);
        let pc_style = Style {
            //            margin: stretch::geometry::Rect {
            //                start: Dimension::Points(10.),
            //                end: Dimension::Points(10.),
            //                top: Dimension::Points(10.),
            //                bottom: Dimension::Points(10.),
            //            },
            align_items: AlignItems::Stretch,
            size: stretch::geometry::Size {
                width: Dimension::Percent(1.),
                height: Dimension::Auto,
            },
            ..Default::default()
        };
        let pc_node = stretch.new_node(pc_style, vec![]).unwrap();
        stretch.add_child(parent_node, pc_node).unwrap();
        dbg!("pc", render_target);
        commands.entity(render_target).with_children(|parent| {
            // background
            view = parent
                .spawn()
                .insert(View)
                .insert(FlexNode(pc_node))
                .with_children(|parent| {
                    parent
                        .spawn()
                        .insert(Material::new(GroupMaterial::new(Vec2::ONE, Vec2::ZERO)))
                        .with_children(|parent| {
                            parallel_coordinates = parent.spawn().id();
                            let mesh = vec![Vec2::new(-1., 0.), Vec2::new(1., 0.)];
                            parent
                                .spawn()
                                .insert(Material::new(LineMaterialData {
                                    color: RGBA::new(1., 0., 0., 1.),
                                    width: Vec2::splat(2.),
                                }))
                                .insert(ZComponent(-1.))
                                .insert(SampledMaterial::new(parallel_coordinates))
                                .insert(SampledLineMaterial)
                                .insert_bundle(MeshBundle::new(mesh, buffers, device));
                        });
                })
                .id();
        });
        let style = Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            ..Default::default()
        };
        let first_style = Style {
            size: stretch::geometry::Size {
                width: Dimension::Percent(1.),
                height: Dimension::Percent(1.),
            },
            ..Default::default()
        };
        let first_container = stretch.new_node(style, vec![]).unwrap();
        for i in 0..n_props {
            let container;
            //            let layout =
            // glyph_brush::Layout::default_single_line().v_align(VerticalAlign::Bottom);
            //            let layout = if i == 0 {
            //                layout.h_align(HorizontalAlign::Left)
            //            } else {
            //                layout.h_align(HorizontalAlign::Left)
            //            };
            let layout = glyph_brush::Layout::default_single_line()
                .v_align(VerticalAlign::Bottom)
                .h_align(HorizontalAlign::Center);

            if i == 0 || i == 1 {
                container = first_container;
            } else {
                container = stretch.new_node(style, vec![]).unwrap();
            }
            if i > 0 {
                stretch.add_child(pc_node, container).unwrap();
            }
            let group_id = commands
                .spawn()
                .insert(Interactive(true))
                .insert(Material::new(GroupMaterial::default()))
                .insert(Text {
                    section: OwnedSection {
                        screen_position: (0.0, 0.0),
                        layout,
                        text: vec![OwnedText::new(scale_names[i].clone())
                            .with_scale(PxScale { x: 17.0, y: 17.0 })
                            .with_color(colors::BLACK)],
                        ..Default::default()
                    },
                })
                // actually needed
                .insert(FlexStyle(style))
                .insert(FlexNode(container))
                .insert(Angle(-90.))
                .id();
            commands.entity(view).push_children(&[group_id]);
            let scale_result = Scale::spawn(
                commands,
                first_style,
                //                if i == 0 { first_style } else { style },
                stretch,
                ScaleKind::Y,
                group_id,
                buffers,
                device,
                render_target,
                container,
                group_id,
                false,
                if i == 0 {
                    Direction::Reversed
                } else {
                    Direction::Normal
                },
                90.,
            );
            let data = vec![
                Vec2::new(-1., f32::MAX),
                Vec2::new(1., f32::MAX),
                Vec2::new(1., f32::MAX),
                Vec2::new(-1., f32::MAX),
                Vec2::new(-1., f32::MAX),
            ];
            let mut filter_marker_id = Entity::new(u32::MAX);
            commands
                .entity(scale_result.marker_parent_id)
                .with_children(|parent| {
                    filter_marker_id = parent
                        .spawn()
                        .insert_bundle(MeshBundle::new(data, &mut *buffers, &device))
                        .insert(Material::new(LineMaterialData {
                            color: RGBA::new(0., 1., 0., 1.),
                            width: Vec2::new(1., 1.),
                        }))
                        .insert(LineStripMaterial)
                        .insert(LineMaterialInPixelSpace(false))
                        .id();
                });
            scales.push(ScaleLink {
                id: i,
                group_id,
                scale_view_id: scale_result.scale_view_id,
                filter_marker_id,
                container_node: container,
            });
        }
        //        let first_node = stretch.remove_child_at_index(pc_node, 0).unwrap();
        //        let new_first_node = stretch.children(pc_node).unwrap()[0];
        //        stretch.add_child(new_first_node, first_node).unwrap();

        commands
            .entity(parallel_coordinates)
            .insert(Material::new(ParallelCoordinates::new(
                n_props,
                RGBA::new(1., 0., 0., 0.1),
            )))
            .insert(ParallelCoordinatesConfig {
                actual_len: n_props,
                // synced
                max_bounds: vec![],
                bounds: vec![],
                scales,
                orders: (0..n_props).into_iter().collect(),
                plot_every_nth_item: 1000,
            })
            .insert(GpuMesh::with_capacity(
                4 * 1024 * 1024,
                &mut *buffers,
                &device,
            ))
            .insert(RenderTexture::new(1600, 1000, format, &device))
            .insert(ParentRenderTarget(render_target))
            .insert(Cursor::default())
            .insert(LineMaterialInPixelSpace(false))
            .insert(ParallelCoordinatesClearColor(Operations {
                load: LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: true,
            }));

        stretch
            .compute_layout(
                parent_node,
                stretch::geometry::Size {
                    width: Number::Defined(1600.),
                    height: Number::Defined(1000.),
                },
            )
            .unwrap();
        debug_layout_for(stretch, parent_node);
        parallel_coordinates
    }
}

fn parallel_coordinates_update_scale_layouts(
    mut reader: EventReader<ResizeRenderTarget>,
    query: Query<(&ParentRenderTarget, &ParallelCoordinatesConfig)>,
    mut texts: Query<&mut Text>,
    stretch: NonSend<Stretch>,
    mut glyph_brushes: ResMut<GlyphBrushes>,
    format: Res<PreferredSurfaceFormat>,
) {
    let brush = glyph_brushes.0.get_mut(&format.0).unwrap();
    for e in reader.iter() {
        let (_, config) = some_loop!(query.iter().find(|x| x.0 .0 == e.id));
        let config: &ParallelCoordinatesConfig = config;
        let item_width = e.size.x as f32 / (config.scales.len() as f32 - 1.);
        for i in 0..config.scales.len() {
            let mut text = texts.get_mut(config.scales[i].group_id).unwrap();
            let layout = stretch.layout(config.scales[i].container_node).unwrap();
            if let Some(bounds) = brush.glyph_bounds(&text.section) {
                // text is rotated by -90
                let width = bounds.max.x - bounds.min.x;
                // allign text to bottom because rotation is at center so it would get clipped
                let offset = if i == 0 {
                    10. + text.section.text[0].scale.y
                } else {
                    item_width * (config.scales.len() as f32 - 1.) / config.scales.len() as f32
                        - text.section.text[0].scale.y
                    // 
                    //                        - width / 2.
                };
                text.section.screen_position = (
                    layout.location.x + offset,
                    layout.location.y + layout.size.height - width / 2.,
                );
            }
        }
    }
}

fn parallel_coordinates_update_groups(
    query: Query<
        (&Material<ParallelCoordinates>, &ParallelCoordinatesConfig),
        Changed<Material<ParallelCoordinates>>,
    >,
    mut groups: Query<&mut Material<GroupMaterial>>,
) {
    for (material, config) in query.iter() {
        let material: &Material<ParallelCoordinates> = material;
        let config: &ParallelCoordinatesConfig = config;
        //        dbg!(&material);
        for link in &config.scales {
            let mut group = groups.get_mut(link.group_id).unwrap();
            let group = group.deref_mut_sneak();
            let scale = material.data.scales[link.id];
            //            dbg!(material.data.translations[link.id].y - config.bounds[link.id].start
            // * scale);
            group.data.translate = Vec2::new(
                material.data.translations[link.id].x,
                material.data.translations[link.id].y,
            );
            group.data.set_scale(Vec2::new(1., scale));
            //            dbg!("updating", &group.data);
        }
    }
}

fn parallel_coordinates_update_material(
    mut writer: EventWriter<FlushPipeline>,
    mut query: Query<(
        &mut Material<ParallelCoordinates>,
        &ParallelCoordinatesConfig,
        Entity,
    )>,
    groups: Query<&Material<GroupMaterial>, Changed<Material<GroupMaterial>>>,
) {
    let mut updated = false;
    for (mut material, config, id) in query.iter_mut() {
        let config: &ParallelCoordinatesConfig = config;
        for link in &config.scales {
            let group = ok_loop!(groups.get(link.group_id));
            // must not create an alias to not trigger change detection
            material.data.scales[link.id] = group.data.scale().y;
            material.data.translations[link.id].y = group.data.translate.y;
            updated = true;
            break;
        }
        if updated {
            writer.send(FlushPipeline::new(id));
        }
    }
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
struct ReorderLabel;

fn parallel_coordinates_handle_reorder(
    mut writer: EventWriter<FlushPipeline>,
    mut grabbed: Local<Option<(usize, Entity, Vec2)>>,
    flex: Flex,
    mut cursor_moved: EventReader<CursorMoved>,
    mut mouse_input: EventReader<MouseInput>,
    mouse_state: Res<MouseState>,
    mut styles: Query<&mut FlexStyle>,
    _scale_views: Query<&Children>,
    _scales: Query<&mut Scale>,
    mut query: Query<(
        &mut Material<ParallelCoordinates>,
        &mut ParallelCoordinatesConfig,
        Entity,
    )>,
    mut texts: Query<&mut Text>,
    _commands: Commands,
    mut parents: Query<&mut Parent>,
) {
    for e in mouse_input
        .iter()
        .filter(|e| e.button == MouseButton::Middle)
    {
        match e.state {
            ElementState::Pressed => {
                error!("pressed");
                'outer: for (_material, config, id) in query.iter_mut() {
                    let config: &ParallelCoordinatesConfig = &config;
                    for i in 0..config.scales.len() {
                        let layout = some_loop!(flex.layout(config.scales[i].scale_view_id));
                        if !layout.contains(mouse_state.render_target_pixel_pos) {
                            continue;
                        }
                        let _style = styles.get_mut(config.scales[i].group_id).unwrap();
                        //                        style.0.position_type = PositionType::Absolute;
                        //                        style.0.position = stretch::geometry::Rect {
                        //                            start: Dimension::Points(layout.location.x),
                        //                            end: Dimension::Undefined,
                        //                            top: Dimension::Undefined,
                        //                            bottom: Dimension::Undefined,
                        //                        };
                        *grabbed = Some((i, id, layout.location));
                        break 'outer;
                    }
                }
            }
            ElementState::Released => {
                let grabbed = some_loop!(grabbed.take());
                error!("reoder");
                let (mut material, mut config, _) = query.get_mut(grabbed.1).unwrap();
                dbg!(&material);
                let config: &mut ParallelCoordinatesConfig = &mut *config;
                let _style = styles
                    .get_mut(config.scales[grabbed.0].scale_view_id)
                    .unwrap();
                let mouse_pos = mouse_state.render_target_pixel_pos;
                let mut new_i = config.scales.len() - 1;
                for i in 0..config.scales.len() {
                    let layout = flex.layout(config.scales[i].scale_view_id).unwrap();
                    if layout.location.x > mouse_pos.x {
                        new_i = i - 1;
                        break;
                    }
                }
                let old_group_id = config.scales[grabbed.0].group_id;
                let new_group_id = config.scales[new_i].group_id;
                let original_order = config.orders.iter().position(|x| *x == grabbed.0).unwrap();
                let new_order = config.orders.iter().position(|x| *x == new_i).unwrap();
                config.orders.swap(original_order, new_order);
                let material: &mut Material<ParallelCoordinates> = &mut *material;
                debug!("swap {} {}", grabbed.0, new_i);
                material.data.scales.swap(new_i, grabbed.0);
                let old_y = material.data.translations[grabbed.0].y;
                let new_y = material.data.translations[new_i].y;
                material.data.translations[grabbed.0].y = new_y;
                material.data.translations[new_i].y = old_y;

                // swap scale groups
                //                let old_scale_view = scale_views
                //                    .get(config.scales[grabbed.0].scale_view_id)
                //                    .unwrap();
                //                let new_scale_view =
                // scale_views.get(config.scales[new_i].scale_view_id).unwrap();
                //                let mut old_scale_id = None;
                //                let mut new_scale_id = None;
                //                for child in old_scale_view.iter() {
                //                    ok_loop!(scales.get_mut(*child));
                //                    old_scale_id = Some(*child);
                //                    break;
                //                }
                //                for child in new_scale_view.iter() {
                //                    ok_loop!(scales.get_mut(*child));
                //                    new_scale_id = Some(*child);
                //                    break;
                //                }
                //                dbg!(old_scale_id, new_scale_id, old_group_id, new_group_id);
                //                scales.get_mut(old_scale_id.unwrap()).unwrap().group =
                // new_group_id;
                // scales.get_mut(new_scale_id.unwrap()).unwrap().group = old_group_id;

                // swap parallel coordinates groups
                let original_parent = parents.get_mut(old_group_id).unwrap().0;
                let new_parent = parents.get_mut(new_group_id).unwrap().0;

                parents.get_mut(old_group_id).unwrap().0 = new_parent;
                parents.get_mut(new_group_id).unwrap().0 = original_parent;

                let old_text =
                    std::mem::take(&mut texts.get_mut(old_group_id).unwrap().section.text[0].text);
                let new_text =
                    std::mem::take(&mut texts.get_mut(new_group_id).unwrap().section.text[0].text);
                texts.get_mut(old_group_id).unwrap().section.text[0].text = new_text;
                texts.get_mut(new_group_id).unwrap().section.text[0].text = old_text;

                //                style.0.position_type = PositionType::Relative;
                //                style.0.position = Default::default();
                writer.send(FlushPipeline::new(grabbed.1));
                dbg!(&config.orders, material);
            }
        }
    }
    if !mouse_state.middle_hold {
        return;
    }
    let mut delta_px = Vec2::ZERO;
    for e in cursor_moved.iter() {
        delta_px += e.delta;
    }
    grabbed.unwrap().2 += delta_px.x;
    //    let (_, config, _) = query.get_mut(grabbed.unwrap().1).unwrap();
    //    let mut style = styles.get_mut(config.scales[grabbed.unwrap().0].group_id).unwrap();
    //    style.0.position = stretch::geometry::Rect {
    //        start: Dimension::Points(pos + delta_px.x),
    //        end: Dimension::Undefined,
    //        top: Dimension::Undefined,
    //        bottom: Dimension::Undefined,
    //    };
}

#[derive(Component)]
struct Scales(Entity);

fn parallel_coordinates_material_changed(
    mut pipelines: ResMut<RenderPipelines>,
    mut arena: ResMut<BindGroupArena>,
    query: Query<&Material<ParallelCoordinates>, Changed<Material<ParallelCoordinates>>>,
    queue: Res<Queue>,
    device: Res<Device>,
    format: Res<PreferredSurfaceFormat>,
    frag: Res<Shader<ColorForwardFrag>>,
) {
    for material in query.iter() {
        let mut context = PreprocessorContext::new();
        context.define("N_PARAMS", material.data.scales.len().to_string());
        let vert = ParallelCoordinatesVert::new(&mut context, &device);
        let pipeline = PipelineBuilder::default()
            .vert_buffer_layout(VertBufferLayout::new().attribute(VertexFormat::Float32))
            .material(material.data.aligned_layout())
            .topology(PrimitiveTopology::LineStrip)
            .blend(BlendState::ALPHA_BLENDING)
            .build_with(&vert.0, &*frag, &device, &format);
        arena.write_material(&material, &queue);
        dbg!(material);
        pipelines.pipelines.insert(
            std::any::TypeId::of::<ParallelCoordinatesPipeline>(),
            pipeline,
        );
    }
}

fn parallel_coordinates_change_mesh(
    query: Query<
        (
            &Scales,
            &ParallelCoordinatesConfig,
            &Material<ParallelCoordinates>,
        ),
        Changed<ParallelCoordinatesConfig>,
    >,
    mut meshes: Query<&mut Mesh<Vec2>>,
) {
    for q in query.iter() {
        let (scales, config, _material): (
            &Scales,
            &ParallelCoordinatesConfig,
            &Material<ParallelCoordinates>,
        ) = q;
        let mut mesh = meshes.get_mut(scales.0).unwrap();
        mesh.data.clear();
        for i in 0..config.actual_len {
            let x = (i as f32 / (config.actual_len - 1) as f32) * 2. - 1.;
            mesh.data.push(Vec2::new(x, -1.));
            mesh.data.push(Vec2::new(x, 1.));
        }
    }
}

#[derive(Component)]
struct ParallelCoordinatesClearColor(Operations<wgpu::Color>);

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ParallelCoordinatesLabel;

pub struct FilterParallelCoordinates(pub Const<Vec<f32>>);
// struct ReorderParallelCoordinates(pub Const<Vec<f32>>);
struct DrawParallelCoordinates(pub Const<Vec<f32>>);

#[derive(Debug)]
pub struct ParallelCoordinates {
    scales: Vec<f32>,
    translations: Vec<Vec2>,
    opacities: Vec<f32>,
    color: RGBA<f32>,
}

impl ParallelCoordinates {
    pub fn new(n_props: usize, color: RGBA<f32>) -> Self {
        let required_len = required_len(n_props);
        let mut opacities = vec![1.; required_len];
        let mut translations: Vec<_> = (0..required_len)
            .into_iter()
            .map(|x| Vec2::new((x as f32 / (n_props - 1) as f32 - 0.5) * 2., -1.))
            .collect();
        translations[n_props].x = translations[n_props - 1].x;
        translations[n_props + 1].x = translations[0].x;
        //        translations[n_props + 2].x = translations[0].x;
        for i in n_props..required_len {
            //            translations[i].x = translations[0].x;
            opacities[i] = 0.;
        }
        dbg!(&translations);

        Self {
            scales: vec![1.; required_len],
            translations,
            opacities,
            color,
        }
    }
}

/// for some unknown reason plotting only works on even number of paramaters
fn required_len(actual_len: usize) -> usize {
    let additional = 2;
    actual_len + additional + (actual_len + additional) % 2
}

#[derive(Component)]
pub struct ParallelCoordinatesConfig {
    actual_len: usize,
    max_bounds: Vec<RangeInclusive<f32>>,
    bounds: Vec<RangeInclusive<f32>>,
    scales: Vec<ScaleLink>,
    orders: Vec<usize>,
    plot_every_nth_item: usize,
}

impl ParallelCoordinatesConfig {
    pub fn set_plot_every_nth_item(&mut self, nth_item: usize) {
        self.plot_every_nth_item = nth_item;
    }

    pub fn max_bounds(&self) -> &[RangeInclusive<f32>] {
        if self.actual_len <= self.max_bounds.len() {
            &self.max_bounds[..self.actual_len]
        } else {
            &self.max_bounds
        }
    }

    pub fn set_max_bounds(
        &mut self,
        material: &mut Material<ParallelCoordinates>,
        bounds: &[RangeInclusive<f32>],
    ) {
        error!("bounds set");
        self.max_bounds.clear();
        self.max_bounds.extend_from_slice(bounds);
        material.data.scales.clear();

        for i in 0..bounds.len() {
            let range = bounds[i].end - bounds[i].start;
            let scale = 2. / range;
            material.data.scales.push(scale);
            material.data.translations[i].y = -1. - bounds[i].start * scale;
        }
        for _ in self.actual_len..required_len(self.actual_len) {
            self.max_bounds
                .push(RangeInclusive::new(f32::MIN, f32::MAX));
            material.data.scales.push(1.);
        }
        self.bounds.clear();
        self.bounds.extend_from_slice(&self.max_bounds);
    }
}

impl AsBytes for ParallelCoordinates {
    fn n_bytes(&self) -> usize {
        self.scales.len() * f32::size()
            + self.translations.len() * Vec2::size()
            + self.opacities.len() * f32::size()
            + RGBA::<f32>::size()
    }

    fn as_bytes<'a>(&'a self) -> Cow<'a, [u8]> {
        let mut bytes = Vec::with_capacity(self.n_bytes());
        unsafe {
            bytes.extend_from_slice(self.scales.transmute_slice());
            bytes.extend_from_slice(self.translations.transmute_slice());
            bytes.extend_from_slice(self.opacities.transmute_slice());
            bytes.extend_from_slice(self.color.as_u8_slice());
        }
        bytes.into()
    }
}

impl MaterialLayout for ParallelCoordinates {}

pub struct ParallelCoordinatesPipeline;

#[derive(Component, Default)]
struct Cursor(usize);

fn parallel_coordinates_filter_handle_input(
    flex: Flex,
    mut cursor_moved: EventReader<CursorMoved>,
    views: Query<(Entity, &Children), With<View>>,
    mut configs: Query<(&mut ParallelCoordinatesConfig, &mut Pipeline)>,
    mut flusher: PipelineFlusher,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mouse_state: Res<MouseState>,
    render_targets: Query<(
        &ParentRenderTarget,
        &RenderTargetPos,
        &RenderTargetSize,
        &Children,
    )>,
    mut meshes: Query<&mut Mesh<Vec2>>,
    children_query: Query<&Children>,
    groups: Query<&Material<GroupMaterial>>,
) {
    if !mouse_state.right_hold {
        return;
    }
    some!(cursor_moved.iter().last());
    let pixel_pos = mouse_state.render_target_pixel_pos;
    let id = some!(mouse_state.window_id);
    let (_, _, size, children) = some!(render_targets
        .iter()
        .find(|x| x.0 .0 == id && pos_in_rect(pixel_pos, x.1 .0.as_vec2(), x.2 .0.as_vec2())));
    for child in children.iter() {
        let (id, children) = ok_loop!(views.get(*child));
        let layout = some_loop!(flex.layout(id));
        if !layout.contains(pixel_pos) {
            continue;
        }
        for group_id in children.iter() {
            for config_id in ok_loop!(children_query.get(*group_id)).iter() {
                let (config, mut pipeline) = ok_loop!(configs.get_mut(*config_id));
                let mut config: Mut<ParallelCoordinatesConfig> = config;
                for scale_id in 0..config.scales.len() {
                    let scale = &config.scales[scale_id];
                    let layout = flex.layout(scale.scale_view_id).unwrap();
                    if !layout.contains(pixel_pos) {
                        continue;
                    }
                    let mut mesh = meshes.get_mut(scale.filter_marker_id).unwrap();
                    let group = groups.get(scale.group_id).unwrap();
                    let screen = pixel_to_screen_space(pixel_pos, size.0.as_vec2());
                    let world = group.data.screen_to_world(screen);
                    if mouse_state.right_clicked {
                        error!("clicked {}", world.y);
                        mesh.data[0].y = world.y;
                        mesh.data[1].y = world.y;
                        config.bounds[scale_id].start = world.y;
                    }
                    error!("expanding {}", world.y);
                    mesh.data[2].y = world.y;
                    mesh.data[3].y = world.y;
                    mesh.data[4] = mesh.data[0];
                    config.bounds[scale_id].end = world.y;
                    dbg!(&config.bounds);

                    writer.send(RedrawRenderTarget::new(id));
                    flusher.flush(&mut *pipeline, *config_id);
                    break;
                }
            }
        }
    }
}

fn parallel_coordinates_filter(
    arena: Res<Arena<Vec<f32>>>,
    writer: PipelinedWriter<DrawParallelCoordinates>,
    reader: PipelinedReader<FilterParallelCoordinates>,
    mut query: Query<(&ParallelCoordinatesConfig, &mut Cursor)>,
) {
    for e in reader.iter() {
        let e: &Pipelined<FilterParallelCoordinates> = e;
        let (config, mut cursor) = ok_loop!(query.get_mut(e.id));
        let mut output = arena.alloc();
        let additional = config.bounds.len() - config.actual_len;
        'element: for element_start in (0..e.0.len()).step_by(config.actual_len) {
            // data is aligned to bounds.len()
            for i in 0..config.actual_len {
                if !config.bounds[i].contains(&e.0[element_start + i]) {
                    continue 'element;
                }
            }
            cursor.0 = cursor.0.wrapping_add(1);
            if cursor.0 % config.plot_every_nth_item != 0 {
                continue;
            }
            let output_start = output.len();
            output.reserve(config.bounds.len());
            unsafe {
                output.set_len(output_start + config.actual_len);
            }
            for i in 0..config.actual_len {
                output[output_start + config.orders[i]] = e.0[element_start + i];
            }
            //            dbg!(cursor.0);
            //            output.extend_from_slice(&e.0[element_start..element_start +
            // config.actual_len]);
            output.extend((0..additional).into_iter().map(|_| 0.));
        }
        dbg!(&config.orders);
        writer.send(DrawParallelCoordinates(output.into()), e.id);
    }
}

fn parallel_coordinates_clear(
    device: Res<Device>,
    queue: Res<Queue>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mut reader: EventReader<FlushPipeline>,
    mut query: Query<(
        &mut Cursor,
        &ParallelCoordinatesClearColor,
        &RenderTexture,
        &ParentRenderTarget,
    )>,
) {
    let mut command_buffers = Vec::new();
    for e in reader.iter() {
        let (mut cursor, color, texture, parent_rt) = ok_loop!(query.get_mut(e.id()));
        *cursor = Cursor::default();
        writer.send(RedrawRenderTarget { id: parent_rt.0 });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("clear pc"),
        });
        let pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[RenderPassColorAttachment {
                view: &texture.view,
                resolve_target: None,
                ops: color.0,
            }],
            depth_stencil_attachment: None,
        });
        drop(pass);
        command_buffers.push(encoder.finish());
    }
    queue.submit(command_buffers);
}

fn parallel_coordinates_configure_texture(
    mut flusher: PipelineFlusher,
    mut reader: EventReader<ResizeRenderTarget>,
    format: Res<PreferredSurfaceFormat>,
    mut configs: Query<
        (
            &mut RenderTexture,
            &ParentRenderTarget,
            &mut Pipeline,
            Entity,
        ),
        With<ParallelCoordinatesConfig>,
    >,
    device: Res<Device>,
    _commands: Commands,
) {
    for e in reader.iter() {
        let (mut texture, _, mut pipeline, id) =
            some_loop!(configs.iter_mut().find(|x| x.1 .0 == e.id));
        flusher.flush(&mut *pipeline, id);
        *texture = RenderTexture::new(e.size.x, e.size.y, format.0, &device);
    }
}

struct ScaleLink {
    id: usize,
    /// container node is also on group
    group_id: Entity,
    scale_view_id: Entity,
    filter_marker_id: Entity,
    container_node: Node,
}

// 150k draw calls ~ 3s, 1.6s in release
// 1 draw call: 1.82s in release
fn parallel_coordinates_draw(
    arena: Res<BindGroupArena>,
    device: Res<Device>,
    queue: Res<Queue>,
    buffers: Res<Buffers>,
    pipelines: Res<RenderPipelines>,
    reader: PipelinedReader<DrawParallelCoordinates>,
    mut query: Query<(
        &ParallelCoordinatesConfig,
        &ParentRenderTarget,
        &Material<ParallelCoordinates>,
        &GpuMesh,
        &RenderTexture,
        &mut ParallelCoordinatesClearColor,
    )>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
) {
    let now = Instant::now();
    let mut iter = reader.iter().peekable();
    if iter.peek().is_none() {
        return;
    }

    struct Job {
        event_pos: usize,
        inner_pos: usize,
        stride: usize,
        data: Vec<Const<Vec<f32>>>,
    }

    impl Job {
        fn write(&mut self, buffer: &MyBuffer, queue: &Queue) -> usize {
            let remaining = self.data[self.event_pos].size() - self.inner_pos
                + self
                    .data
                    .iter()
                    .skip(self.event_pos + 1)
                    .map(|x| x.size())
                    .sum::<usize>();
            let remaining = remaining - remaining % self.stride;
            let to_write = buffer.capacity().min(remaining);
            let data_len = to_write;
            let mut buffer_offset = 0;

            while self.event_pos < self.data.len() {
                while self.inner_pos < self.data[self.event_pos].size() {
                    let to_write = (self.data[self.event_pos].size() - self.inner_pos)
                        .min(data_len - buffer_offset as usize);
                    let source = unsafe {
                        self.data[self.event_pos][self.inner_pos / f32::size()
                            ..(self.inner_pos + to_write) / f32::size()]
                            .transmute_slice()
                    };
                    queue.write_buffer(buffer.buffer(), buffer_offset, source);
                    buffer_offset += source.len() as BufferAddress;
                    self.inner_pos += to_write;
                }
                if self.inner_pos == self.data[self.event_pos].size() {
                    self.inner_pos = 0;
                    self.event_pos += 1;
                }
            }
            to_write
        }

        fn is_finished(&self) -> bool {
            self.event_pos >= self.data.len()
        }
    }

    let mut map: HashMap<Entity, Job> = Default::default();

    for e in iter {
        match map.get_mut(&e.id) {
            None => {
                let (config, parent_render_target, ..) = &ok_loop!(query.get_mut(e.id));
                // set parent render target dirty
                writer.send(RedrawRenderTarget::new(parent_render_target.0));
                map.insert(
                    e.id,
                    Job {
                        event_pos: 0,
                        inner_pos: 0,
                        stride: required_len(config.actual_len) * f32::size(),
                        data: vec![e.0.clone()],
                    },
                );
            }
            Some(v) => v.data.push(e.0.clone()),
        }
    }
    while !map.is_empty() {
        let mut command_buffers = Vec::new();
        map.retain(|k, v| {
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
                label: Some("output plot"),
            });
            let (config, _, material, mesh, texture, _color) = query.get_mut(*k).unwrap();
            let written = v.write(&buffers.get(mesh.buffer_id), &queue);
            if written == 0 {
                return false;
            }
            let n_vertices = written / f32::size();
            let _n_instances = written / (config.bounds.len() * f32::size());
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[RenderPassColorAttachment {
                    view: &texture.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            pass.set_pipeline(&pipelines.pipelines[&ParallelCoordinatesPipeline::id()]);

            pass.set_vertex_buffer(
                0,
                buffers
                    .get(mesh.buffer_id)
                    .buffer()
                    .slice(..written as BufferAddress),
            );
            pass.set_bind_group(0, arena.get(material.id), &[]);
            //            for i in 0..n_instances {
            //                let offset = config.bounds.len() as u32 * i as u32;
            //                pass.draw(offset..config.bounds.len() as u32 + offset, 0..1);
            //            }
            pass.draw(0..n_vertices as u32, 0..1);
            drop(pass);
            command_buffers.push(encoder.finish());
            !v.is_finished()
        });
        queue.submit(command_buffers);
        let fut = queue.on_submitted_work_done();
        async move {
            fut.await;
            debug!("draw pc took: {} ms", now.elapsed().as_millis());
        }
        .spawn();
        //        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
