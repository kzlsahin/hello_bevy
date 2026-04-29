use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::{ShaderRef},
};

use crate::shaders::OCEAN_SHADER_PATH;
use crate::shaders::OCEAN_FRAGMENT_SHADER_PATH;

#[derive(Debug, Clone, ShaderType)]
pub struct OceanMaterialUniform {
    pub camera_pos: Vec3,
    pub time: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct OceanMaterial {
    #[uniform(0)]
    pub params: OceanMaterialUniform,
}

impl Material for OceanMaterial {
    fn vertex_shader() -> ShaderRef {
        OCEAN_SHADER_PATH.into()
    }
     fn fragment_shader() -> ShaderRef {
        OCEAN_FRAGMENT_SHADER_PATH.into()
    }
}