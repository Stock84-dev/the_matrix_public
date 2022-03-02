use std::ops::RangeBounds;
use std::path::PathBuf;
use std::pin::Pin;

use bevy::prelude::*;
use bevy::task::{BevyTryFut, Carc, Task};
use itertools::Itertools;
use merovingian::compression::{CompressionPlugin, Decompress, Decompressed, DecompressionMethod};
use merovingian::output_reader::{OutputReadError, OutputReader};
use merovingian::structs::RangeInclusive;
use merovingian::variable::{Variable, Variables};
use mouse::mem::{Arena, Const};
use mouse::num::{f16, NumExt};
use mouse::traits::AsyncReadSeek;
use opencl::construct::{BacktestResult, SeekResult, ToF32Array};
use opencl::{size, Precision};
use stretch::style::{Dimension, FlexDirection, Style};
use stretch::Stretch;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use wgpu::{Device, LoadOp, Operations};

use crate::flex::FlexboxPlugin;
use crate::imgui_plugin::ImguiPlugin;
use crate::niobe::{FlexNode, FlexStyle};
use crate::render::pipelines::d2::line::LinePlugin;
use crate::render::pipelines::d2::parallel_coordinates::{
    FilterParallelCoordinates, ParallelCoordinates, ParallelCoordinatesConfig,
    ParallelCoordinatesLabel, ParallelCoordinatesPlugin,
};
use crate::render::{
    upload_mesh, Buffers, ChildRenderTargetBundle, Material, RenderTarget, RenderTargetBundle,
};

pub type AsyncRwLock<T> = tokio::sync::RwLock<T>;

pub struct OutputPlotPlugin;

impl Plug for OutputPlotPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
            .load(CompressionPlugin)
            .load(ParallelCoordinatesPlugin)
            .load(LinePlugin)
            .load(ImguiPlugin)
            .load(FlexboxPlugin)
            .load(PipelinePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.init_resource::<Arena<Vec<u8>>>()
            .init_resource::<Arena<Vec<f32>>>()
            .add_pipeline::<OutputFiltered>()
            .add_pipelined_portal::<OutputLoaded>()
            .add_startup_system(startup_big_data)
            .render_system(upload_mesh::<Vec2>)
            .add_system(sync_config.before(ParallelCoordinatesLabel))
            .add_system_set(
                SystemSet::new()
                    .with_system(start_load)
                    .with_system(decompress_output)
                    .with_system(deserialize_output),
            )
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum OutputSourceKind {
    File(PathBuf),
}
// fn sync_config(
//    mut query: Query<(
//        &Carc<AsyncRwLock<OutputPlotConfig>>,
//        &mut Material<ParallelCoordinates>,
//        &mut ParallelCoordinatesConfig,
//    )>,
//) {
//    for (output, mut material, mut pp) in query.iter_mut() {
//        let guard = output.read().block();
//        let additional = 2;
//        if pp.bounds.is_empty()
//            || pp.bounds.len() - additional != guard.bounds.len()
//            || guard.bounds != pp.bounds[0..guard.bounds.len()]
//        {
//            trace!("sync config");
//            if material.data.opacities.len() != pp.bounds.len() {
//                pp.bounds.extend_from_slice(&guard.bounds);
//                for _ in 0..additional {
//                    material.data.opacities.push(0.);
//                    let t = material.data.translations[0];
//                    material.data.translations.push(Vec2::new(t.x, 0.));
//                    material.data.scales.push(1.);
//                    pp.bounds.push(RangeInclusive::new(0., 1.));
//                }
//            }
//            //            pp.bounds.clear();
//            pp.bounds[0..guard.bounds.len()].copy_from_slice(&guard.bounds);
//            //            pp.bounds.push(RangeInclusive::new(0., 1.));
//            for i in 0..guard.bounds.len() {
//                let range = guard.bounds[i].end - guard.bounds[i].start;
//                material.data.scales[i] = 2. / range / 4.;
//            }
//        }
//    }
//}

fn sync_config(
    mut query: Query<(
        &Carc<AsyncRwLock<OutputPlotConfig>>,
        &mut Material<ParallelCoordinates>,
        &mut ParallelCoordinatesConfig,
    )>,
) {
    for (output, mut material, mut pp) in query.iter_mut() {
        let guard = output.read().block();
        if pp.max_bounds() != guard.bounds {
            let _bounds = guard.bounds.clone();
            pp.set_max_bounds(&mut *material, &guard.bounds);
        }
    }
}
// fn sync_config(
//    mut query: Query<(
//        &Carc<AsyncRwLock<OutputPlotConfig>>,
//        &mut Material<ParallelCoordinates>,
//        &mut ParallelCoordinatesConfig,
//    )>,
//) {
//    for (output, mut material, mut pp) in query.iter_mut() {
//        let guard = output.read().block();
//        if guard.bounds != pp.bounds {
//            trace!("sync config");
//            pp.bounds.clear();
//            pp.bounds.extend_from_slice(&guard.bounds);
//            for i in 0..guard.bounds.len() {
//                let range = guard.bounds[i].end - guard.bounds[i].start;
//                material.data.scales[i] = 2. / range / 4.;
//            }
//        }
//    }
//}

fn start_load(
    mut commands: Commands,
    portal: Res<PipelinedPortal<OutputLoaded>>,
    arena: Res<Arena<Vec<u8>>>,
    query: Query<(Entity, &Carc<AsyncRwLock<OutputPlotConfig>>), Without<Task<()>>>,
) {
    for (id, config) in query.iter() {
        info!("spawn load");
        let config: &Carc<AsyncRwLock<OutputPlotConfig>> = config;
        let _guard = config.write().block();
        let watch = AsyncPipeline::spawn(id, &mut commands);
        //        guard.load_sender = Some(txc);
        //        guard.load_receiver = Some(rxc);
        let config = config.clone();
        let context = config.clone();

        commands.entity(id).insert(
            load_output(id, config, watch, portal.clone(), arena.clone())
                .spawn_handled_with_context(async move {
                    format!("{:?}", context.read().await.output_source_kind)
                }),
        );
    }
}

async fn find_max_bounds(
    reader: &mut OutputReader<Pin<Box<dyn AsyncReadSeek>>>,
    min_variables: &mut Variables,
    max_variables: &mut Variables,
) -> Result<Vec<RangeInclusive<f32>>> {
    let header = reader.read_header().await?;
    reader.skip(&header).await?;
    let mut max_bounds = Vec::with_capacity(min_variables.len() + header.ranges.len());
    let n_variables = min_variables.len();
    min_variables.set_combination(header.start_combination);
    max_variables.set_combination(header.end_combination_inclusive);
    for i in 0..min_variables.len() {
        max_bounds.push(RangeInclusive::new(
            min_variables[i].value,
            max_variables[i].value,
        ));
    }
    for i in 0..header.ranges.len() {
        max_bounds.push(RangeInclusive::new(
            header.ranges[i].start,
            header.ranges[i].end,
        ));
    }
    loop {
        let header = match reader.read_header().await {
            Ok(h) => h,
            Err(OutputReadError::AllRead) => break,
            Err(e) => return Err(e.into()),
        };
        min_variables.set_combination(header.start_combination);
        max_variables.set_combination(header.end_combination_inclusive);
        for i in 0..min_variables.len() {
            max_bounds[i].start.min_mut(min_variables[i].value);
            max_bounds[i].end.max_mut(max_variables[i].value);
        }
        for i in 0..header.ranges.len() {
            max_bounds[n_variables + i]
                .start
                .min_mut(header.ranges[i].start);
            max_bounds[n_variables + i]
                .end
                .max_mut(header.ranges[i].end);
        }
        reader.skip(&header).await?;
    }
    reader.seek_start().await?;
    for bound in &mut max_bounds {
        if bound.start == bound.end {
            bound.end = bound.start + 1.;
        }
    }
    dbg!(&max_bounds);
    Ok(max_bounds)
}

async fn read(
    id: &Entity,
    config: &Carc<AsyncRwLock<OutputPlotConfig>>,
    reader: &mut OutputReader<Pin<Box<dyn AsyncReadSeek>>>,
    arena: &Arena<Vec<u8>>,
    min_variables: &mut Variables,
    max_variables: &mut Variables,
    portal: &PipelinedPortal<OutputLoaded>,
) -> Result<(), OutputReadError> {
    let mut dest = arena.alloc();
    let mut header;
    loop {
        let mut skip = true;
        header = reader.read_header().await?;
        let config = config.read().await;
        min_variables.set_combination(header.start_combination);
        max_variables.set_combination(header.end_combination_inclusive);
        for i in 0..min_variables.len() {
            if config.bounds[i].contains(&min_variables[i].value)
                || config.bounds[i].contains(&max_variables[i].value)
            {
                skip = false;
                break;
            }
        }
        if skip {
            for i in min_variables.len()..config.bounds.len() {
                if config.bounds[i].contains(&header.ranges[i].start)
                    || config.bounds[i].contains(&header.ranges[i].end)
                {
                    skip = false;
                    break;
                }
            }
        }
        drop(config);
        if skip {
            reader.skip(&header).await?;
        } else {
            break;
        }
    }
    reader.read_block(&header, &mut *dest).await?;
    portal.send(OutputLoaded { data: dest.into() }, *id).await;
    Ok(())
}

async fn load_output(
    id: Entity,
    config: Carc<AsyncRwLock<OutputPlotConfig>>,
    mut watch: AsyncPipelineWatch,
    portal: PipelinedPortal<OutputLoaded>,
    arena: Arena<Vec<u8>>,
) -> Result<()> {
    let guard = config.read().await;
    let _element_size = guard.element_size();
    let mut min_variables: Variables = guard.variables.clone();
    let mut max_variables: Variables = guard.variables.clone();
    let should_find_max_bounds = guard.max_bounds.is_none();
    let mut reader: OutputReader<Pin<Box<dyn AsyncReadSeek>>> = OutputReader::new(
        Box::pin(match &guard.output_source_kind {
            OutputSourceKind::File(path) => File::open(path).await?,
        }),
        guard.output_kind.n_output_paramaters(),
    );
    drop(guard);

    if should_find_max_bounds {
        info!("finding max bounds");
        let bounds = find_max_bounds(&mut reader, &mut min_variables, &mut max_variables).await?;
        let mut guard = config.write().await;
        if guard
            .bounds
            .iter()
            .all(|x| x.start == f32::MIN && x.end == f32::MAX)
        {
            guard.bounds = bounds.clone();
        }
        guard.max_bounds = Some(bounds);
    }
    info!("reading...");
    loop {
        tokio::select! {
            result = watch.on_flushing() => {
                warn!("flushing");
                ok_if_err!(result);
                reader.seek_start().await?;
                ok_if_err!(watch.on_flushed().await);
                warn!("flushed");
            }
            result = read(&id, &config, &mut reader, &arena, &mut min_variables, &mut max_variables, &portal) => {
                match result {
                    Ok(_) => {},
                    Err(OutputReadError::AllRead) => {
                        ok_if_err!(watch.on_flushing().await);
                        ok_if_err!(result);
                        reader.seek_start().await?;
                        ok_if_err!(watch.on_flushed().await);
                    },
                    Err(e) => return Err(e.into()),
                };
            }
        }
    }
    Ok(())
}

fn decompress_output(
    writer: PipelinedWriter<Decompress>,
    reader: PipelinedPortalReader<OutputLoaded>,
    query: Query<&Carc<AsyncRwLock<OutputPlotConfig>>>,
) {
    for e in reader.read().iter() {
        let config = query.get(e.id).unwrap();
        writer.send(
            Decompress {
                method: config.read().block().decompression_method,
                data: e.data.clone(),
            },
            e.id,
        );
    }
}

fn deserialize_output(
    arena: Res<Arena<Vec<f32>>>,
    writer: PipelinedWriter<FilterParallelCoordinates>,
    reader: PipelinedReader<Decompressed>,
    query: Query<&Carc<AsyncRwLock<OutputPlotConfig>>>,
) {
    for e in reader.iter() {
        let config = ok_loop!(query.get(e.id)).read().block();
        let kind = config.output_kind;
        let precision = config.precision;
        let mut variables: Variables = config.variables.clone();
        let element_size = config.element_size();
        let n_elements = e.0.len() / element_size;
        let mut output = arena.alloc();
        output.reserve(n_elements * config.bounds.len());
        drop(config);
        let mut reader = &**e.0;
        // consume number of items
        bincode::deserialize_from::<_, u64>(&mut reader)
            .map_err(|_| OutputReadError::CorruptedSource)
            .log();
        let mut values = Vec::new();
        loop {
            let combination: u64 = match bincode::deserialize_from(&mut reader) {
                Ok(c) => c,
                Err(_) => break,
            };
            // error is NOT here
            variables.set_combination(combination);
            for v in variables.variables() {
                output.push(v.value);
                values.push(v.value);
            }
            match kind {
                OutputKind::Combinations => {}
                OutputKind::SeekResults => match precision {
                    Precision::F16 => {
                        let result: SeekResult<f16> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                    Precision::F32 => {
                        let result: SeekResult<f32> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                    Precision::F64 => {
                        let result: SeekResult<f64> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                },
                OutputKind::BacktestResults => match precision {
                    Precision::F16 => {
                        let result: BacktestResult<f16> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                    Precision::F32 => {
                        let result: BacktestResult<f32> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                    Precision::F64 => {
                        let result: BacktestResult<f64> =
                            some_loop!(bincode::deserialize_from(&mut reader)
                                .log_context("corrupted data source"));
                        result.to_f32_extend(&mut *output);
                    }
                },
            }
        }
        writer.send(FilterParallelCoordinates(output.into()), e.id);
    }
}

pub struct OutputFiltered {
    pub id: Entity,
    pub data: Const<Vec<f32>>,
}

pub struct OutputLoaded {
    pub data: Const<Vec<u8>>,
}

#[derive(Component)]
pub struct OutputPlotConfig {
    max_bounds: Option<Vec<RangeInclusive<f32>>>,
    bounds: Vec<RangeInclusive<f32>>,
    decompression_method: DecompressionMethod,
    variables: Variables,
    precision: Precision,
    output_kind: OutputKind,
    output_source_kind: OutputSourceKind,
}

impl OutputPlotConfig {
    fn element_size(&self) -> usize {
        (match self.output_kind {
            OutputKind::Combinations => 0,
            OutputKind::SeekResults => size!(SeekResult, self.precision),
            OutputKind::BacktestResults => size!(BacktestResult, self.precision),
        }) + u64::size()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum OutputKind {
    Combinations,
    SeekResults,
    BacktestResults,
}

impl OutputKind {
    pub fn n_output_paramaters(&self) -> usize {
        match self {
            OutputKind::Combinations => 1,
            OutputKind::SeekResults => SeekResult::<f32>::FIELD_NAMES_AS_ARRAY.len(),
            OutputKind::BacktestResults => BacktestResult::<f32>::FIELD_NAMES_AS_ARRAY.len(),
        }
    }
}

fn startup_big_data(
    device: Res<Device>,
    mut buffers: ResMut<Buffers>,
    mut ui_renderer: ResMut<imgui_wgpu::Renderer>,
    mut commands: Commands,
    mut stretch: NonSendMut<Stretch>,
    windows: Query<(&RenderTarget, Entity)>,
) {
    let (rt, window_entity) = windows.iter().next().unwrap();
    let _mesh = vec![Vec2::new(-1., 0.), Vec2::new(1., 0.)];
    let variables = Variables::new(vec![
        Variable::new(1.0, 14400.0, 1.0, 1.0),
        Variable::new(0.0, 1.0, 0.0, 1.0),
        Variable::new(0.0, 3.0, 0.0, 1.0),
        Variable::new(2.0, 300.0, 2.0, 1.0),
        Variable::new(0.0, 100.0, 0.0, 1.0),
        Variable::new(0.0, 100.0, 0.0, 1.0),
    ]);
    let n_props = variables.len() + 4;
    let render_target_style = Style {
        flex_direction: FlexDirection::Row,
        size: stretch::geometry::Size {
            width: Dimension::Percent(1.),
            height: Dimension::Percent(1.),
        },
        ..Default::default()
    };
    let render_target_node = stretch.new_node(render_target_style, vec![]).unwrap();
    debug!("{:#?}", render_target_node);
    let parent_render_target = commands.spawn().id();

    let render_target = commands
        .entity(parent_render_target)
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
        .id();
    let scale_names: Vec<String> = vec![
        "timeframe".into(),
        "input offset".into(),
        "input kind".into(),
        "lenght".into(),
        "lline".into(),
        "hline".into(),
        "balance".into(),
        "max_balance".into(),
        "max_drawdown".into(),
        "n_trades".into(),
    ];
    let parallel_coordinates = ParallelCoordinatesPlugin::spawn(
        &scale_names,
        &mut *buffers,
        &device,
        rt.format,
        render_target,
        render_target_node,
        &mut *stretch,
        &mut commands,
    );

    commands
        .entity(parallel_coordinates)
        .insert(Carc::new(AsyncRwLock::new(OutputPlotConfig {
            // auto initialized
            max_bounds: None,
            bounds: vec![RangeInclusive::new(f32::MIN, f32::MAX); n_props],
            decompression_method: DecompressionMethod::Zstd,
            variables,
            precision: Precision::F32,
            output_kind: OutputKind::SeekResults,
            //            output_source_kind:
            // OutputSourceKind::File("/home/stock/cache/rsi/run.seek_f32".into()),
            output_source_kind: OutputSourceKind::File("/home/stock/data/tmp/seek_f32.zst".into()),
            /* auto initialized
             *            load_sender: None,
             *            load_receiver: None, */
        })));
    //    stretch
    //        .compute_layout(
    //            render_target_node,
    //            stretch::geometry::Size {
    //                width: Number::Defined(1600.),
    //                height: Number::Defined(1000.),
    //            },
    //        )
    //        .unwrap();
}
