use std::cell::RefCell;

use std::rc::Rc;


use bevy::prelude::*;
use bevy::utils::HashMap;
use imgui::{Condition, Context, DrawData, FontSource, Image, Ui};

use imgui_winit_support::{HiDpiMode, WinitPlatform};
use wgpu::{
    Device, Extent3d, TextureUsages,
};




use crate::render::{
    PrevRenderTargetSize, RedrawRenderTarget, RenderTextureId,
    ResizeRenderTarget,
};
use crate::winit_plugin::{RedrawWindow, Window, WindowClosing, WindowCreated, WinitPlugin};
use crate::{RenderTargetPos, RenderTargetSize};

pub struct ImguiPlugin;

impl Plug for ImguiPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(WinitPlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        app.init_non_send_resource::<ImguiContexts>()
            .init_non_send_resource::<SharedFontAtlas>()
            .init_resource::<ImguiDrawDatas>()
            .init_resource::<ImguiState>()
            .pre_pre_system(create_context.label(CreateContextLabel))
            .pre_pre_system(pre_ui.before(UiLabel).after(CreateContextLabel))
            .pre_pre_system(create_ui.label(UiLabel))
            .pre_pre_system(post_ui.after(UiLabel))
            .pre_pre_system(
                change_render_target_size
                    .after(UiLabel)
                    .label(RenderTargetSizeAppliedLabel),
            )
            .post_system(configure_render_target_texture)
            .add_system_to_stage(CoreStage::PostUpdate, remove_context)
    }
}

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct RenderTargetSizeAppliedLabel;

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
pub struct UiLabel;

#[derive(SystemLabel, Clone, Hash, Eq, PartialEq, Debug)]
struct CreateContextLabel;

#[derive(Default)]
struct ImguiState {
    example: i32,
}

#[derive(Deref, DerefMut, Default)]
pub struct ImguiContexts(pub HashMap<Entity, ImguiContext>);

fn create_context(
    mut contexts: NonSendMut<ImguiContexts>,
    atlas: NonSend<SharedFontAtlas>,
    mut events: EventReader<WindowCreated>,
    windows: Query<&Window>,
) {
    for e in events.iter() {
        let context = ImguiContext::new(windows.get(e.id).unwrap(), &*atlas);
        contexts.insert(e.id, context);
    }
}

fn remove_context(
    mut contexts: NonSendMut<ImguiContexts>,
    mut events: EventReader<WindowClosing>,
    mut datas: ResMut<ImguiDrawDatas>,
) {
    for e in events.iter() {
        datas.0.remove(&e.id);
        contexts.0.remove(&e.id);
    }
}

fn pre_ui(
    mut contexts: NonSendMut<ImguiContexts>,
    mut events: UniqueEventReader<RedrawWindow>,
    windows: Query<&Window>,
) {
    for e in events.iter() {
        let context: &mut ImguiContext = match contexts.get_mut(&e.id) {
            None => return,
            Some(x) => x,
        };
        let window = &windows.get(e.id).unwrap();
        context
            .platform
            .prepare_frame(context.context.io_mut(), &window)
            .expect("Failed to prepare frame");
        let ui = context.context.frame();
        // changing lifetime so that it could be shared between systems
        let ui = unsafe { std::mem::transmute(ui) };
        context.ui = Some(ui);
    }
}

fn post_ui(
    mut contexts: NonSendMut<ImguiContexts>,
    mut events: UniqueEventReader<RedrawWindow>,
    windows: Query<&Window>,
    mut datas: ResMut<ImguiDrawDatas>,
) {
    for e in events.iter() {
        let context: &mut ImguiContext = match contexts.get_mut(&e.id) {
            None => return,
            Some(x) => x,
        };
        let window = windows.get(e.id).unwrap();
        context
            .platform
            .prepare_render(context.ui.as_ref().unwrap(), window);
        let data = context.ui.take().unwrap().render();
        // renderer only reads data so it is safe
        datas.0.insert(e.id, unsafe { std::mem::transmute(data) });
    }
}

fn create_ui(
    mut contexts: NonSendMut<ImguiContexts>,
    mut events: UniqueEventReader<RedrawWindow>,
    mut writer: EventWriter<ResizeRenderTarget>,
    windows: Query<&Window>,
    mut query: Query<(
        &mut RenderTargetPos,
        &mut RenderTargetSize,
        Entity,
        &RenderTextureId,
    )>,
) {
    // NOTE: we must read with different query because deref triggers changed event
    // NOTE: borrow checker doesn't check this
    let (mut rpos, render_target_size, rt_id, texture_id) = query.iter_mut().next().unwrap();

    for e in events.iter() {
        let context: &mut ImguiContext = match contexts.get_mut(&e.id) {
            None => return,
            Some(x) => x,
        };
        let window = windows.get(e.id).unwrap();
        let size = window.inner_size();
        let margin = UVec2::new(15, 35);
        let texture_size = UVec2::new(size.width, size.height) - margin;
        if texture_size != render_target_size.0 {
            writer.send(ResizeRenderTarget {
                id: rt_id,
                size: texture_size,
            });
        }
        let ui = context.ui.as_mut().unwrap();
        let window = imgui::Window::new("Color button examples")
            .size([size.width as f32, size.height as f32], Condition::Always)
            .position([0., 0.], Condition::Appearing)
            //            .size(context.context.)
            //            .position([20.0, 20.0], Condition::Appearing)
            //            .size([3000.0, 100.0], Condition::Appearing)
            .movable(false)
            .resizable(true);
        window.build(&ui, || {
            let size = texture_size.as_vec2();
            let pos = ui.cursor_pos();
            rpos.0.x = pos[0] as u32;
            rpos.0.y = pos[1] as u32;
            Image::new(texture_id.0, size.to_array()).build(ui);
            //            let ex1 = ui.radio_button("Example 1: Basics", &mut state.example, 1);
            //            let ex2 = ui.radio_button("Example 2: Alpha component", &mut
            // state.example, 2);            let ex3 = ui.radio_button("Example 3: Input
            // format", &mut state.example, 3);            if ex1 || ex2 || ex3 {
            //                state.example = 0;
            //            }
        });
    }
}

fn configure_render_target_texture(
    mut reader: EventReader<ResizeRenderTarget>,
    device: Res<Device>,
    mut ui_renderer: ResMut<imgui_wgpu::Renderer>,
    query: Query<&RenderTextureId>,
) {
    // change node size -> compute layout
    // change texture
    for e in reader.iter() {
        let id = ok_loop!(query.get(e.id));
        let texture_config = imgui_wgpu::TextureConfig {
            size: Extent3d {
                width: e.size.x,
                height: e.size.y,
                ..Default::default()
            },
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            format: None,
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(&device, &ui_renderer, texture_config);
        ui_renderer.textures.replace(id.0, texture);
    }
}

fn change_render_target_size(
    mut reader: EventReader<ResizeRenderTarget>,
    mut writer: UniqueEventWriter<RedrawRenderTarget>,
    mut rts: Query<(&mut RenderTargetSize, &mut PrevRenderTargetSize)>,
) {
    for e in reader.iter() {
        let (mut size, mut prev_size) = ok_loop!(rts.get_mut(e.id));
        prev_size.0 = size.0;
        size.0 = e.size;
        writer.send(RedrawRenderTarget::new(e.id));
    }
}

static mut show_demo: bool = true;

#[derive(Debug)]
pub struct ImguiContext {
    pub platform: WinitPlatform,
    pub context: Context,
    pub ui: Option<Ui<'static>>,
}

impl ImguiContext {
    pub fn new(window: &Window, font_atlas: &SharedFontAtlas) -> Self {
        let mut context = Context::create_with_shared_font_atlas(font_atlas.0.clone());
        let mut platform = WinitPlatform::init(&mut context);
        let hidpi_factor = window.scale_factor();
        platform.attach_window(context.io_mut(), window, HiDpiMode::Default);
        context.set_ini_filename(None);
        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        Self {
            platform,
            context,
            ui: None,
        }
    }
}

pub struct SharedFontAtlas(Rc<RefCell<imgui::SharedFontAtlas>>);

impl FromWorld for SharedFontAtlas {
    fn from_world(_: &mut World) -> Self {
        let mut atlas = imgui::SharedFontAtlas::create();
        atlas.add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: 13.,
                ..Default::default()
            }),
        }]);
        Self(Rc::new(RefCell::new(atlas)))
    }
}

#[derive(Default)]
pub struct ImguiDrawDatas(pub HashMap<Entity, DrawDataSync>);
pub struct DrawDataSync(pub &'static DrawData);
unsafe impl Sync for DrawDataSync {}
unsafe impl Send for DrawDataSync {}
