use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Deref;

use bevy::prelude::*;
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};

pub fn startup_static_shader<T: StaticShader>(device: Res<Device>, mut commands: Commands) {
    let shader = device.create_shader_module(&ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(T::source())),
    });
    commands.insert_resource(Shader {
        module: shader,
        _p: PhantomData::<T>::default(),
    });
}

pub struct Shader<T> {
    module: ShaderModule,
    _p: PhantomData<T>,
}

impl<T> Deref for Shader<T> {
    type Target = ShaderModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

pub trait StaticShader: Send + Sync + 'static {
    fn source() -> &'static str;
}

pub trait AppShaderExt {
    fn init_shader<T: StaticShader>(&mut self, _shader: T) -> &mut Self;
}

impl AppShaderExt for App {
    fn init_shader<T: StaticShader>(&mut self, _shader: T) -> &mut Self {
        self.startup_wgpu_system(startup_static_shader::<T>)
    }
}

pub mod fragment;
pub mod vertex;
