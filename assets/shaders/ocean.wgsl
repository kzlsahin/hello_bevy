#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct OceanMaterial {
    camera_pos: vec3<f32>,
    time: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: OceanMaterial;

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    // multiple waves layered together
    let wave1 = sin(position.x * 0.3 + material.time * 2.5) * 0.8;
    let wave2 = sin(position.x * 0.1 + material.time * 1.5) * 0.4;
    let wave3 = sin(position.x * 0.6 - material.time * 3.5) * 0.2;
    let wave4 = sin(position.x * 0.6 + material.time * 3.5) * 0.2;
    let wave5 = sin(position.z * 0.05 + material.time * 3.3) * 0.4;
    let wave6 = sin(0.2 * position.z + material.time * 4.5) * 0.1;
    let wave_height = wave1 + wave2 + wave3 + wave4 + wave5 + wave6;

    let displaced = vec3<f32>(position.x, position.y + wave_height, position.z);

    // approximate the normal based on wave slope
    let dx = cos(position.x * 0.3 + material.time * 1.2) * 0.3 * 0.4
           + cos((position.x + position.z) * 0.15 + material.time * 1.5) * 0.15 * 0.2;
    let dz = cos(position.z * 0.2 + material.time * 0.8) * 0.2 * 0.3
           + cos((position.x + position.z) * 0.15 + material.time * 1.5) * 0.15 * 0.2;
    let wave_normal = normalize(vec3<f32>(-dx, 1.0, -dz));

    var out: VertexOutput;
    out.position = view.clip_from_world * vec4<f32>(displaced, 1.0);
    out.world_position = vec4<f32>(displaced, 1.0);
    out.world_normal = wave_normal;
    out.uv = uv;
    return out;
}