#![feature(map_first_last)]
#![feature(negative_impls)]
#![feature(specialization)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(trace_macros)]
#![feature(try_blocks)]
#![feature(auto_traits)]
//#![feature(const_generics)]
#![feature(generic_const_exprs)]
#![feature(core_intrinsics)]
#![feature(const_type_id)]
#![feature(extend_one)]
#![deny(unreachable_patterns)]
#![deny(unused_must_use)]

#[macro_use]
extern crate bytemuck;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate derive_setters;
#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate futures_util;
#[macro_use]
extern crate pin_project;
#[macro_use]
extern crate mouse;

use std::any::TypeId;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::time::Instant;

use bevy::ecs::change_detection::Mut;
use bevy::ecs::schedule::ReportExecutionOrderAmbiguities;
use bevy::log::{Level, LogSettings};
use bevy::math::UVec2;
use bevy::prelude::*;
use bevy_mod_debugdump::schedule_graph::{schedule_graph_dot_styled, ScheduleGraphStyle};
use bytemuck::Pod;
use dozer::provider::load_hlcv;
use ieee754::Ieee754;
use merovingian::hlcv::Hlcv;
use mouse::num::NumExt;
use rand::Rng;
use zigzag::ZigZag;

// mod my_node_pass;
pub mod app;
pub mod colors;
pub mod flex;
pub mod imgui_plugin;
pub mod niobe;
pub mod render;
pub mod scale;
pub mod winit_plugin;
// mod vertex_buffer_layout_builder;

pub struct CorePlugin;

impl Plug for CorePlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.add_default_stages()
            .insert_resource(ReportExecutionOrderAmbiguities)
            .add_startup_stage_before(
                StartupStage::PreStartup,
                InitStages::Window,
                SystemStage::parallel(),
            )
            .add_stage_before(
                CoreStage::PreUpdate,
                Stages::PrePreUpdate,
                SystemStage::parallel(),
            )
            .add_stage_before(Stages::PrePreUpdate, Stages::Input, SystemStage::parallel())
            .add_stage_after(CoreStage::Update, Stages::Added, SystemStage::parallel())
            .add_stage_after(
                CoreStage::PostUpdate,
                Stages::Render,
                SystemStage::parallel(),
            )
            .add_stage_after(Stages::Render, Stages::Draw, SystemStage::parallel())
    }
}

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
}

fn main2() -> Result<()> {
    //    use tracing_subscriber::layer::SubscriberExt;
    //    let layer = tracing_subscriber::fmt::layer()
    //        .with_span_events(FmtSpan::CLOSE)
    //        .finish();
    //    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
    //        .expect("set up the subscriber");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(set_tokio_handle());
    unsafe {
        config::load("config.yaml")?;
    }
    let mut app = App::empty();
    app.insert_resource(LogSettings {
        filter: "wgpu=trace".to_string(),
        level: Level::TRACE,
    });
    //    app.add_plugin(bevy::log::LogPlugin);
    let mut loader = PluginLoader::new(&mut app);
    loader
        .load(CorePlugin)
        // must run before
        .load(render::RenderPlugin)
        .load(winit_plugin::WinitPlugin)
        .load(imgui_plugin::ImguiPlugin)
        .load(niobe::big_data::OutputPlotPlugin);
    let mut file = File::create("/tmp/schedule.dot")?;
    write!(
        &mut file,
        "{}",
        schedule_graph_dot_styled(
            &app.schedule,
            &ScheduleGraphStyle {
                hide_startup_schedule: false,
                ..ScheduleGraphStyle::dark()
            }
        )
    )
    .unwrap();

    app.run();
    // NOTE: if winit runner is being used, when there are no windows it will call process::exit()
    // instead of returning here
    unreachable!();
}
pub trait ResourceInsertNonSendSafe {
    fn insert_non_send_resource_safe<R: 'static>(&mut self, resource: R)
    where
        If<{ std::mem::size_of::<R>() != 0 }>: True;
}

pub trait ResourceInsertSafe {
    fn insert_resource_safe<R: Resource>(&mut self, resource: R)
    where
        If<{ std::mem::size_of::<R>() != 0 }>: True;
}

pub trait WorldExt {
    fn get_resource_init<'a, R: Resource + FromWorld>(&'a mut self) -> &'a R;
    fn get_resource_init_mut<'a, R: Resource + FromWorld>(&'a mut self) -> Mut<'a, R>;
}

impl WorldExt for World {
    fn get_resource_init<'a, R: Resource + FromWorld>(&'a mut self) -> &'a R {
        if !self.contains_resource::<R>() {
            let resource = R::from_world(self);
            self.insert_resource(resource);
        }
        &*self.get_resource::<R>().unwrap()
    }

    fn get_resource_init_mut<'a, R: Resource + FromWorld>(&'a mut self) -> Mut<'a, R> {
        if !self.contains_resource::<R>() {
            let resource = R::from_world(self);
            self.insert_resource(resource);
        }
        self.get_resource_mut::<R>().unwrap()
    }
}

impl ResourceInsertSafe for World {
    fn insert_resource_safe<R: Resource>(&mut self, resource: R)
    where
        If<{ std::mem::size_of::<R>() != 0 }>: True,
    {
        self.insert_resource(resource);
    }
}

impl ResourceInsertNonSendSafe for World {
    fn insert_non_send_resource_safe<R: 'static>(&mut self, resource: R)
    where
        If<{ std::mem::size_of::<R>() != 0 }>: True,
    {
        self.insert_non_send(resource);
    }
}

pub struct If<const B: bool>;
pub trait True {}
impl True for If<true> {}

impl<'a, 'b> ResourceInsertSafe for Commands<'a, 'b> {
    fn insert_resource_safe<R: Resource>(&mut self, resource: R)
    where
        If<{ std::mem::size_of::<R>() != 0 }>: True,
    {
        self.insert_resource(resource);
    }
}

#[derive(SystemLabel, Clone, Copy, Hash, Eq, PartialEq, Debug)]
pub struct TypeIdLabel {
    data: TypeId,
}

impl TypeIdLabel {
    pub fn new<T: 'static>() -> Self {
        Self { data: T::id() }
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct RenderTargetSize(pub UVec2);

#[derive(Component)]
pub struct RenderTargetPos(pub UVec2);

#[derive(Component)]
pub struct LineMaterialInPixelSpace(pub bool);

pub fn find_root_parent(parents: &Query<&Parent>, mut entity: Entity) -> Entity {
    while let Ok(parent) = parents.get(entity) {
        entity = parent.0;
    }
    return entity;
}
fn dema() {
    let mut prev_exp = 0;
    let mut prev_mant = 0;
    let mut prev_mant_delta = 0;
    use rand;
    let neg = ZigZag::encode(-0838861i32);
    let pos = ZigZag::encode(0838861i32);
    println!("{:#034b}", unsafe { *(&neg as *const _ as *const u32) });
    println!("{:#034b}", unsafe { *(&pos as *const _ as *const u32) });

    let mut val = 2000000.0f32;
    let mut rng = rand::thread_rng();
    for i in 0..100 {
        let prev_val = val;
        let mut sign = 0.;
        if rng.gen::<f32>() < 0.2 {
            sign = -1.;
        }
        if rng.gen::<f32>() > 0.8 {
            sign = 1.;
        }
        let mult = rng.gen_range(1..5);
        val += 0.25 * sign * mult as f32;
        let xor = unsafe { *(&val as *const _ as *const i32) }
            - unsafe { *(&prev_val as *const _ as *const i32) };
        let xor_f32 = unsafe { *(&xor as *const _ as *const f32) };
        print!("{:#034b} ", xor);
        let mant_delta = val.decompose_raw().2 as i32 - prev_mant as i32;
        print!("{:#034b}", unsafe { *(&val as *const _ as *const u32) });
        print!(
            " {:#08} {:#08} {} {:#010b} {:#025b} {:#03} {:#07}, {:#08} {:#08} {:#034b} {:#034b}",
            xor_f32.decompose_raw().1,
            xor_f32.decompose_raw().2,
            val.decompose_raw().0 as u8,
            val.decompose_raw().1,
            val.decompose_raw().2,
            val.decompose_raw().1,
            val.decompose_raw().2,
            val.decompose_raw().1 as i32 - prev_exp as i32,
            mant_delta,
            mant_delta,
            ZigZag::encode(mant_delta),
        );
        println!(" {}", val);
        prev_mant_delta = mant_delta;
        prev_exp = val.decompose_raw().1;
        prev_mant = val.decompose_raw().2;
    }
}

fn main() {
    dema();
    panic!();
    //    //    wtf();
    //    unsafe {
    //        config::load("/home/stock/ssd/projects/the_matrix/the_matrix/config.yaml").unwrap();
    //    }
    //    let rt = tokio::runtime::Runtime::new().unwrap();
    //    rt.block_on(set_tokio_handle());
    //    let mapped = rt
    //        .block_on(load_hlcv(
    //            "BitMEX",
    //            "XBTUSD",
    //            1443184465,
    //            1633869265 - 1443184465,
    //        ))
    //        .unwrap();
    //    let map: Vec<_> = mapped.as_ref().iter().map(|x| x.close).collect();
    //    let mut now = Instant::now();
    //    let b = normal(&map);
    //    let mut encoder = zstd::stream::read::Encoder::with_buffer(&b[..], 19).unwrap();
    //    let mut out = Vec::new();
    //    std::io::copy(&mut encoder, &mut out).unwrap();
    //    let mega = 1024 as f32 * 1024 as f32;
    //    let input = b.size() as f32 / mega;
    //    let output = out.size() as f32 / mega;
    //
    //    println!(
    //        "{} MiB -> {} MiB {} {} ms {} MiB/s",
    //        input,
    //        output,
    //        output / input,
    //        now.elapsed().as_millis(),
    //        input / now.elapsed().as_secs() as f32,
    //    );
    //
    //    z1(mapped.as_ref());
}

fn count_bits_le(mut n: u32) -> u32 {
    let mut count = 0;
    // While loop will run until we get n = 0
    while n != 0 {
        count += 1;
        n = n >> 1;
    }
    count
}
// TODO: better mantissa compression
//  70 000.5, 70 002.0 requires 17 bits of precision, but floating point has 23 bits
//  we could save 6 bits if we know desired precision, we could go through all mantissas and find
//  first lsb with 1
//

fn wtf() {
    fn p(val: i16) {
        println!("{}", val);
        println!("val: {:#018b}", val);
        println!("zig: {:#018b}", ZigZag::encode(val));
        println!("shf: {:#018b}", ZigZag::encode(val) >> 1);
        println!("bzi: {:#018b}", ZigZag::encode(val.to_be()));
        println!("bshf: {:#018b}", ZigZag::encode(val.to_be()) >> 1);
    }
    println!("1.23 = {:#034b}", unsafe {
        *(&1.23f32 as *const _ as *const u32)
    });
    println!("-1.23 = {:#034b}", unsafe {
        *(&-1.23f32 as *const _ as *const u32)
    });
    //    p(3);
    //    p(3 >> 1);
    p(0);
    p(1);
    p(-1);
    p(-32768);
    p(-32767);
    p(32767);
    p(1337);
    p(-1337);
}

fn z1(hlcvs: &[Hlcv]) {
    use bitvec::bitvec;
    use bitvec::prelude::*;
    use ieee754::Ieee754;
    use zigzag::ZigZag;
    let mut exp = 0;
    let mut exp_i = 0;
    let mut encoded = Vec::<u8>::new();
    //    let mut max_mantissa = 0;
    //    let mut min_mantissa = u32::MAX;
    let mut prev_mantissa = 0;
    //    let mut avg_mantissa = 0.;
    let mut avg_delta = 0.;
    //    let mut min_delta = i32::MAX;
    let mut max_delta = 0;
    let mut bits = BitVec::<Msb0, u8>::new();
    for (i, hlcv) in hlcvs.iter().enumerate() {
        let current_exp = hlcv.close.decompose_raw().1;
        let mantissa = hlcv.close.decompose_raw().2;
        let delta = mantissa as i32 - prev_mantissa as i32;
        let delta = ZigZag::encode(delta);
        let d = delta;
        //        println!("{}", delta);
        //        avg_mantissa += mantissa as f32 / hlcvs.len() as f32;
        avg_delta += (delta) as f32 / hlcvs.len() as f32;
        //        let d: u32 = unsafe { std::mem::transmute(delta) };
        //        let max_d: u32 = unsafe { std::mem::transmute(max_delta) };
        //        let min_d: u32 = unsafe { std::mem::transmute(min_delta) };
        //        let range = max_d.max(min_d);
        let range = max_delta;
        let bits_required = count_bits_le(range);

        prev_mantissa = mantissa;
        if current_exp != exp {
            //            println!("{}%", i as f32 / hlcvs.len() as f32 * 100.);
            println!("{}", bits_required);
            encoded.extend_from_slice(prev_mantissa.as_u8_slice());
            encoded.push(exp);
            encoded.extend_from_slice(bits.as_raw_slice());
            bits.clear();
            exp = current_exp;
            exp_i = i;
            max_delta = 0;
            //            min_delta = i32::MAX;
            //            println!("{} {}", exp, exp_i);
            bits.extend_from_bitslice(&d.view_bits::<Msb0>()[..bits_required as usize]);
            continue;
        }
        bits.extend_from_bitslice(&d.view_bits::<Msb0>()[..bits_required as usize]);
        //        min_mantissa.min_mut(mantissa);
        //        max_mantissa.max_mut(mantissa);
        //        min_delta.min_mut(delta);
        max_delta.max_mut(delta);
    }
    let ratio = (hlcvs.len() * 4) as f32 / encoded.len() as f32;
    println!("ratio: {}", ratio);
    // bitvec mantissa of 23 bits: 1.509
    // bitvec mantissa varlen with u32: 1.08
    // bitvec mantissa varlen zigzag: 1.77
    // bitvec mantissa varlen zigzag + zstd: 45.89
    let mut file = std::fs::File::create("/tmp/a").unwrap();
    file.write_all(&encoded).unwrap();
    dbg!(
        //        min_mantissa,
        //        avg_mantissa,
        //        max_mantissa,
        //        min_delta,
        avg_delta, max_delta
    );
}

struct Pipeline {
    id: u64,
    outputs: Vec<Vec<u64>>,
}

struct Message {
    data: Vec<u8>,
    pipeline_id: u64,
    // we could have multiple mailboxes with same id
    node_id: u64,
}

struct LoadMessage {
    data: Vec<u8>,
    pipeline_id: u64,
}

struct DecompressMessage {
    data: Vec<u8>,
    pipeline_id: u64,
}

// struct DeserializedMessage {
//    data: Vec<MyStruct>,
//    other: Vec<u8>,
//    pipeline_id: u64,
//}
// struct Reader<T> {}

pub trait System {
    fn call(&mut self);
}

impl<F> System for F
where
    F: FnMut(),
{
    fn call(&mut self) {
        (self)()
    }
}

fn user<S: System>(mut system: S) {
    system.call();
}

#[test]
fn demo() {
    System::call(&mut load);
}

fn load() {
    println!("hello world");
    // file.read
    // send
}

// fn system(reader: Reader<String>) {}

// requirements:
// - multi producer multi observer messaging system
// send message to multiple destinations
// system requires multiple input/output mailboxes
// cyclic graph structure
// in memory and on disk states
// stages
// distributed architecture
// custom pipeline system to handle errors
// dynamic mods
// allow multiple pipelines of same layout to run

// mailbox:
// a container of messages
// can be specific to pipeline, node
// system uses multiple mailboxes and can send to multiple mailboxes
// semaphore: only passes message once it receives that a message down the line has been passed
