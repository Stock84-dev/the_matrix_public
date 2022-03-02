use std::collections::HashMap;
use std::ops::Deref;
use std::path::PathBuf;
use std::time::Instant;

use bevy::ecs::change_detection::Mut;
use bevy::prelude::*;
use winit::dpi::Pixel;
use winit::event::{
    ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget};
use winit::window::{WindowBuilder, WindowId};

use crate::imgui_plugin::ImguiContexts;
use crate::render::ParentRenderTarget;
use crate::{RenderTargetPos, ResourceInsertNonSendSafe, Stages};

pub struct WinitPlugin;

#[derive(StageLabel, Clone, Eq, PartialEq, Debug, Hash)]
pub enum WindowStartupStage {
    CreatingWindow,
    WindowCreated,
}

pub struct MyWindow {
    pub window: Window,
    pub render_target: Entity,
}

pub struct MouseScrollUnit(f32);

impl Default for MouseScrollUnit {
    fn default() -> Self {
        Self(1.)
    }
}

#[derive(Debug)]
pub enum WinitEvent {
    Update,
    EventLoop(EventLoopEvent),
}

async fn feed_winit(proxy: EventLoopProxy<WinitEvent>) {
    loop {
        EVENT_LOOP.wait_for_events().await;
        let mut guard = EVENT_LOOP.read();
        for e in guard.iter() {
            proxy.send_event(WinitEvent::EventLoop(e)).ignore();
        }
        if guard.update_needed() {
            proxy.send_event(WinitEvent::Update).ignore();
        }
    }
}

impl Plug for WinitPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.add_event::<WindowResized>()
            .add_event::<CreateWindow>()
            .add_event::<WindowCreated>()
            .add_event::<WindowClosing>()
            .add_event::<WindowMoved>()
            .add_event::<CloseWindow>()
            .add_event::<CursorMoved>()
            .add_event::<CursorEntered>()
            .add_event::<CursorLeft>()
            .add_event::<ReceivedCharacter>()
            .add_event::<WindowFocus>()
            .add_event::<WindowScaleFactorChanged>()
            .add_event::<FileDragAndDrop>()
            .add_event::<KeyboardInput>()
            .add_event::<MouseInput>()
            .add_event::<MouseWheel>()
            .add_unique_event::<RedrawWindow>()
            .init_resource::<MouseState>()
            .init_resource::<MouseScrollUnit>()
            .add_startup_system_to_stage(InitStages::Window, startup_winit.exclusive_system())
            .add_system_to_stage(Stages::PrePreUpdate, mouse_state)
            .add_system_to_stage(CoreStage::Last, mouse_state_last)
            .post_system(close_window)
            .set_runner(winit_runner)
    }
}

fn startup_winit(world: &mut World) {
    // main window must be created before startup
    let event_loop = EventLoop::<WinitEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new().build(&*event_loop).unwrap();
    let window_id = window.id();
    let id = world.spawn().insert(Window(window)).id();
    world.resource_scope(|_world, mut writer: Mut<Events<WindowCreated>>| {
        writer.send(WindowCreated { id });
    });
    let mut windows = HashMap::new();
    feed_winit(proxy).spawn();
    windows.insert(window_id, id);
    world.insert_resource(windows);
    world.insert_non_send_resource_safe(event_loop);
}

// use tracing_flame::{FlameLayer, FlushGuard};
// use tracing_subscriber::prelude::*;
// use tracing_subscriber::registry::Registry;
// pub struct FlameGuard(FlushGuard<std::io::BufWriter<std::fs::File>>);
//
// fn setup_global_subscriber() -> FlameGuard {
//    let fmt_layer = tracing_subscriber::fmt::Layer::default();
//
//    let (flame_layer, _guard) = FlameLayer::with_file("/tmp/flame.folded").unwrap();
//
//    tracing_subscriber::registry()
//        .with(fmt_layer)
//        .with(flame_layer)
//        .init();
//    FlameGuard(_guard)
//}
// static PATH: &str = "flame.folded";
//
// fn make_flamegraph(tmpdir: impl AsRef<Path>, out: impl AsRef<Path>) {
//    let out = out.as_ref();
//    let tmpdir = tmpdir.as_ref();
//    println!("outputting flamegraph to {}", out.display());
//    let inf = std::fs::File::open(tmpdir.join(PATH)).unwrap();
//    let reader = std::io::BufReader::new(inf);
//
//    let out = std::fs::File::create(out).unwrap();
//    let writer = std::io::BufWriter::new(out);
//
//    let mut opts = inferno::flamegraph::Options::default();
//    inferno::flamegraph::from_reader(&mut opts, reader, writer).unwrap();
//}

fn winit_runner(mut app: App) {
    // TODO: currently winit listens to all events even if window isn't focused
    //  https://github.com/rust-windowing/winit/issues/1634

    //    let guard = setup_global_subscriber();
    //    app.insert_resource(guard);

    // we must first startup all systems otherwise resize events and window size will become out of
    // sync
    info!("Starting up...");
    app.update();
    info!("Starting up...DONE");
    let mut mouse_pos = Vec2::new(0., 0.);
    let mut active = true;
    let event_loop = app
        .world
        .remove_non_send::<EventLoop<WinitEvent>>()
        .unwrap();
    let mut windows = app
        .world
        .remove_resource::<HashMap<WindowId, Entity>>()
        .unwrap();
    let mut update_count = 0u64;
    let _last_mouse_update = Instant::now();
    let mut last_frame = Instant::now();
    let mut window_event_ocurred = false;
    let mut last_redraw_requests = vec![];
    let mut last_event_time = Instant::now();
    let mut process_time = Instant::now();
    EVENT_LOOP.update();
    event_loop.run(move |event, event_loop, control_flow| {
        //        trace!("{:?}", event);
        *control_flow = ControlFlow::Wait;
        // satisfy borrow checker
        let mut contexts = app.world.remove_non_send::<ImguiContexts>().unwrap();
        for (id, context) in contexts.0.iter_mut() {
            let window = app.world.get::<Window>(*id).unwrap();
            context
                .platform
                .handle_event(context.context.io_mut(), window, &event);
        }
        app.world.insert_non_send_resource_safe(contexts);

        match event {
            Event::NewEvents(_) => {
                process_time = Instant::now();
                let now = Instant::now();
                let mut contexts = app
                    .world
                    .get_non_send_resource_mut::<ImguiContexts>()
                    .unwrap();
                for (_id, context) in contexts.0.iter_mut() {
                    context.context.io_mut().update_delta_time(now - last_frame);
                }
                last_frame = now;
            }
            Event::WindowEvent {
                event, window_id, ..
            } => {
                window_event_ocurred = true;
                let id = windows[&window_id];
                let window = app.world.get::<Window>(id).unwrap();
                window.request_redraw();
                let window_size = window.inner_size();

                match event {
                    WindowEvent::Resized(size) => {
                        debug!("{:?} {:?}", window_size, size);
                        println!("resized {:?}", window_id);
                        let mut resize_events = app
                            .world
                            .get_resource_mut::<Events<WindowResized>>()
                            .unwrap();
                        resize_events.send(WindowResized {
                            id: windows[&window_id],
                            width: window_size.width,
                            height: window_size.height,
                        });
                    }
                    WindowEvent::CloseRequested => {
                        let mut window_close_requested_events = app
                            .world
                            .get_resource_mut::<Events<WindowClosing>>()
                            .unwrap();
                        window_close_requested_events.send(WindowClosing {
                            id: windows[&window_id],
                        });
                        app.update();
                        let mut window_close_requested_events =
                            app.world.get_resource_mut::<Events<CloseWindow>>().unwrap();
                        window_close_requested_events.send(CloseWindow {
                            id: windows[&window_id],
                        });
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        let mut keyboard_input_events = app
                            .world
                            .get_resource_mut::<Events<KeyboardInput>>()
                            .unwrap();
                        keyboard_input_events.send(input);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        // TOOD: sum all cursor moved events into one
                        let mut mouse_state = app.world.get_resource_mut::<MouseState>().unwrap();
                        let position = Vec2::new(position.x as f32, position.y as f32);
                        let diff = position - mouse_pos;
                        mouse_pos = position;
                        mouse_state.window_pixel_pos = mouse_pos;
                        drop(mouse_state);
                        let delta = Vec2::new(diff.x, diff.y);

                        let mut cursor_moved_events =
                            app.world.get_resource_mut::<Events<CursorMoved>>().unwrap();
                        cursor_moved_events.send(CursorMoved {
                            id: windows[&window_id],
                            position: Vec2::new(position.x.cast(), position.y.cast()),
                            delta,
                        });
                    }
                    WindowEvent::CursorEntered { .. } => {
                        let mut mouse_state = app.world.get_resource_mut::<MouseState>().unwrap();
                        mouse_state.window_id = Some(windows[&window_id]);
                        drop(mouse_state);

                        let mut cursor_entered_events = app
                            .world
                            .get_resource_mut::<Events<CursorEntered>>()
                            .unwrap();
                        cursor_entered_events.send(CursorEntered {
                            id: windows[&window_id],
                        });
                    }
                    WindowEvent::CursorLeft { .. } => {
                        let mut mouse_state = app.world.get_resource_mut::<MouseState>().unwrap();
                        mouse_state.window_id = None;
                        drop(mouse_state);
                        let mut cursor_left_events =
                            app.world.get_resource_mut::<Events<CursorLeft>>().unwrap();
                        cursor_left_events.send(CursorLeft {
                            id: windows[&window_id],
                        });
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        let mut mouse_state = app.world.get_resource_mut::<MouseState>().unwrap();
                        let pressed = match state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                        match button {
                            MouseButton::Left => {
                                mouse_state.left_clicked = !mouse_state.left_hold & pressed;
                                mouse_state.left_hold = pressed;
                                mouse_state.left_click_time = Instant::now();
                            }
                            MouseButton::Right => {
                                mouse_state.right_clicked = !mouse_state.right_hold & pressed;
                                mouse_state.right_hold = pressed;
                                mouse_state.right_click_time = Instant::now();
                                error!("{}", mouse_state.right_clicked);
                                error!("{}", mouse_state.right_hold);
                            }
                            MouseButton::Middle => {
                                mouse_state.middle_clicked = !mouse_state.middle_hold & pressed;
                                mouse_state.middle_hold = pressed;
                                mouse_state.middle_click_time = Instant::now();
                            }
                            MouseButton::Other(_) => {}
                        }
                        drop(mouse_state);
                        let mut mouse_button_input_events =
                            app.world.get_resource_mut::<Events<MouseInput>>().unwrap();
                        mouse_button_input_events.send(MouseInput { button, state });
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_x, y),
                        ..
                    } => {
                        // delta is a vector of [0., +-1.]
                        let scroll_unit = app.world.get_resource::<MouseScrollUnit>().unwrap();
                        let delta = y * scroll_unit.0;
                        let mut mouse_wheel_input_events =
                            app.world.get_resource_mut::<Events<MouseWheel>>().unwrap();
                        mouse_wheel_input_events.send(MouseWheel { delta });
                    }
                    WindowEvent::Touch(_touch) => {}
                    WindowEvent::ReceivedCharacter(c) => {
                        let mut char_input_events = app
                            .world
                            .get_resource_mut::<Events<ReceivedCharacter>>()
                            .unwrap();

                        char_input_events.send(ReceivedCharacter {
                            id: windows[&window_id],
                            char: c,
                        });
                    }
                    WindowEvent::ScaleFactorChanged {
                        scale_factor: _,
                        new_inner_size,
                    } => {
                        let mut resize_events = app
                            .world
                            .get_resource_mut::<Events<WindowResized>>()
                            .unwrap();
                        resize_events.send(WindowResized {
                            id: windows[&window_id],
                            width: new_inner_size.width,
                            height: new_inner_size.height,
                        });
                    }
                    WindowEvent::Focused(focused) => {
                        let mut focused_events =
                            app.world.get_resource_mut::<Events<WindowFocus>>().unwrap();
                        focused_events.send(WindowFocus {
                            id: windows[&window_id],
                            focused,
                        });
                    }
                    WindowEvent::DroppedFile(path_buf) => {
                        let mut events = app
                            .world
                            .get_resource_mut::<Events<FileDragAndDrop>>()
                            .unwrap();
                        events.send(FileDragAndDrop::DroppedFile {
                            id: windows[&window_id],
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFile(path_buf) => {
                        let mut events = app
                            .world
                            .get_resource_mut::<Events<FileDragAndDrop>>()
                            .unwrap();
                        events.send(FileDragAndDrop::HoveredFile {
                            id: windows[&window_id],
                            path_buf,
                        });
                    }
                    WindowEvent::HoveredFileCancelled => {
                        let mut events = app
                            .world
                            .get_resource_mut::<Events<FileDragAndDrop>>()
                            .unwrap();
                        events.send(FileDragAndDrop::HoveredFileCancelled {
                            id: windows[&window_id],
                        });
                    }
                    WindowEvent::Moved(position) => {
                        let position = IVec2::new(position.x, position.y);
                        let mut events =
                            app.world.get_resource_mut::<Events<WindowMoved>>().unwrap();
                        events.send(WindowMoved {
                            id: windows[&window_id],
                            position,
                        });
                    }
                    _ => {}
                }
            }
            Event::Suspended => {
                active = false;
            }
            Event::Resumed => {
                active = true;
            }
            // if we resize window we will get RedrawEventsCleared but not MainEventsCleared
            Event::MainEventsCleared => {
                handle_create_window_events(&mut app.world, event_loop.deref(), &mut windows);
            }
            Event::RedrawEventsCleared => {
                // there are animations in imgui so we have to process multiple frames
                if window_event_ocurred {
                    window_event_ocurred = false;
                    for id in last_redraw_requests.drain(..) {
                        app.world.get::<Window>(id).unwrap().0.request_redraw();
                    }
                }
                update_count = update_count.wrapping_add(1);
                println!("start update #{}", update_count);
                let update_time = Instant::now();
                app.update();
                println!("end update #{}", update_count);
                println!(
                    "event time: {} ms, update_time: {} ms",
                    last_event_time.elapsed().as_millis(),
                    update_time.elapsed().as_millis()
                );
                last_event_time = Instant::now();
                let elapsed = process_time.elapsed().as_micros() as u64;
                println!("{} FPS", 1_000_000. / elapsed as f32);
                let min_frame_time = 16_666;
                if elapsed < min_frame_time {
                    std::thread::sleep(std::time::Duration::from_micros(min_frame_time - elapsed));
                }
            }
            Event::RedrawRequested(window_id) => {
                last_redraw_requests.push(windows[&window_id]);
                let mut events = app
                    .world
                    .get_resource_mut::<UniqueEvents<RedrawWindow>>()
                    .unwrap();
                events.send(RedrawWindow {
                    id: windows[&window_id],
                });
            }
            Event::UserEvent(WinitEvent::Update) => {
                // update is run after all events have been processed
                //                info!("startup redraw");
                //                windows
                //                    .windows
                //                    .iter()
                //                    .for_each(|(_, window)| window.request_redraw());
            }
            Event::UserEvent(WinitEvent::EventLoop(EventLoopEvent::Exit)) => {
                info!("exiting...");
                *control_flow = ControlFlow::Exit;
                //                drop(app.world.remove_resource::<FlameGuard>());
                //                make_flamegraph("/tmp", "flame.svg");
            }
            _ => (),
        }
    });
}

fn handle_create_window_events(
    world: &mut World,
    event_loop: &EventLoopWindowTarget<WinitEvent>,
    windows: &mut HashMap<WindowId, Entity>,
) {
    world.resource_scope(|world, reader: Mut<Events<CreateWindow>>| {
        world.resource_scope(|world, mut writer: Mut<Events<WindowCreated>>| {
            for _e in reader.get_reader().iter(&reader) {
                let window = WindowBuilder::new().build(event_loop).unwrap();
                let window_id = window.id();
                let id = world.spawn().insert(Window(window)).id();
                writer.send(WindowCreated { id });
                windows.insert(window_id, id);
            }
        });
    });
}

fn close_window(
    windows: Query<&Window>,
    mut reader: EventReader<CloseWindow>,
    mut commands: Commands,
) {
    let mut iter = reader.iter().peekable();
    let skip = iter.peek().map(|_| 1).unwrap_or(0);
    for e in iter {
        commands.entity(e.id).despawn_recursive();
    }
    // skip closing window
    if windows.iter().skip(skip).next().is_none() {
        EVENT_LOOP.send(EventLoopEvent::Exit);
    }
}

fn mouse_state(
    mut mouse_state: ResMut<MouseState>,
    mut cursor_moved: EventReader<CursorMoved>,
    render_targets: Query<(&ParentRenderTarget, &RenderTargetPos)>,
) {
    let event = match cursor_moved.iter().last() {
        Some(e) => e,
        None => return,
    };
    let pos = match render_targets.iter().find(|x| x.0 .0 == event.id) {
        Some(x) => x.1,
        None => return,
    };
    mouse_state.render_target_pixel_pos = mouse_state.window_pixel_pos - pos.0.as_vec2();
}

fn mouse_state_last(mut mouse_state: ResMut<MouseState>) {
    let delay = 100;
    if mouse_state.left_click_time.elapsed().as_millis() > delay {
        mouse_state.left_clicked = false;
    }
    if mouse_state.middle_click_time.elapsed().as_millis() > delay {
        mouse_state.middle_clicked = false;
    }
    if mouse_state.right_click_time.elapsed().as_millis() > delay {
        mouse_state.right_clicked = false;
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct Window(winit::window::Window);

#[derive(Component)]
pub struct CursorKind(pub winit::window::CursorIcon);

#[derive(Debug)]
pub struct MouseState {
    pub window_id: Option<Entity>,
    /// in pixel space
    pub window_pixel_pos: Vec2,
    pub render_target_pixel_pos: Vec2,
    pub left_hold: bool,
    pub middle_hold: bool,
    pub right_hold: bool,
    pub left_clicked: bool,
    pub middle_clicked: bool,
    pub right_clicked: bool,
    left_click_time: Instant,
    middle_click_time: Instant,
    right_click_time: Instant,
    /*    pub left_double_clicked: bool,
     *    pub middle_double_clicked: bool,
     *    pub right_double_clicked: bool, */
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            window_id: None,
            window_pixel_pos: Default::default(),
            render_target_pixel_pos: Default::default(),
            left_hold: false,
            middle_hold: false,
            right_hold: false,
            left_clicked: false,
            middle_clicked: false,
            right_clicked: false,
            left_click_time: Instant::now(),
            middle_click_time: Instant::now(),
            right_click_time: Instant::now(),
        }
    }
}

/// A window event that is sent whenever a windows logical size has changed
#[derive(Debug, Clone)]
pub struct WindowResized {
    pub id: Entity,
    /// The new logical width of the window
    pub width: u32,
    /// The new logical height of the window
    pub height: u32,
}

/// An event that indicates that a new window should be created.
#[derive(Debug, Clone)]
pub struct CreateWindow;

/// An event that indicates a window should be closed.
#[derive(Debug, Clone)]
pub struct CloseWindow {
    pub id: Entity,
}

/// An event that is sent whenever a new window is created.
#[derive(Debug, Clone)]
pub struct WindowCreated {
    pub id: Entity,
}

/// An event that is sent whenever a close was requested for a window. For example: when the "close"
/// button is pressed on a window.
#[derive(Debug, Clone)]
pub struct WindowClosing {
    pub id: Entity,
}

#[derive(Debug, Clone)]
pub struct CursorMoved {
    pub id: Entity,
    // In pixel space
    pub position: Vec2,
    pub delta: Vec2,
}

#[derive(Debug, Clone)]
pub struct CursorEntered {
    pub id: Entity,
}

#[derive(Debug, Clone)]
pub struct CursorLeft {
    pub id: Entity,
}

/// An event that is sent whenever a window receives a character from the OS or underlying system.
#[derive(Debug, Clone)]
pub struct ReceivedCharacter {
    pub id: Entity,
    pub char: char,
}

/// An event that indicates a window has received or lost focus.
#[derive(Debug, Clone)]
pub struct WindowFocus {
    pub id: Entity,
    pub focused: bool,
}

/// An event that indicates a window's scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowScaleFactorChanged {
    pub id: Entity,
    pub scale_factor: f64,
}
/// An event that indicates a window's OS-reported scale factor has changed.
#[derive(Debug, Clone)]
pub struct WindowBackendScaleFactorChanged {
    pub id: Entity,
    pub scale_factor: f64,
}

/// Events related to files being dragged and dropped on a window.
#[derive(Debug, Clone)]
pub enum FileDragAndDrop {
    DroppedFile { id: Entity, path_buf: PathBuf },
    HoveredFile { id: Entity, path_buf: PathBuf },
    HoveredFileCancelled { id: Entity },
}

/// An event that is sent when a window is repositioned in physical pixels.
#[derive(Debug, Clone)]
pub struct WindowMoved {
    pub id: Entity,
    pub position: IVec2,
}

#[derive(Debug, Clone)]
pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
}

#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub delta: f32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct RedrawWindow {
    pub id: Entity,
}
