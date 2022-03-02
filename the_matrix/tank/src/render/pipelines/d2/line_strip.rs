



use bevy::prelude::*;






use crate::render::pipelines::d2::line::{
    line_system, startup_line, LineCorePlugin,
};






pub struct LineStripPlugin;

impl Plug for LineStripPlugin {
    fn deps<'a, 'b>(loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader.load(LineCorePlugin)
    }

    fn load<'a>(app: &'a mut App) -> &mut App {
        // TODO: gather all uniforms in plugin by using a macro
        app.startup_wgpu_pipeline_system(startup_line::<LineStripPipeline>)
            //            .startup_wgpu_pipeline_system(startup_line::<SampledLineStripPipeline>)
            .render_system(line_system::<LineStripPipeline, LineStripMaterial>)
        //            .render_system(line_system::<SampledLineStripPipeline,
        // SampledLineStripMaterial>)
    }
}

pub struct LineStripPipeline;
pub struct SampledLineStripPipeline;
#[derive(Component)]
pub struct LineStripMaterial;
#[derive(Component)]
pub struct SampledLineStripMaterial;
