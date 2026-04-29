#import bevy_pbr::{
    mesh_view_bindings::globals,
    forward_io::VertexOutput,
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Base ocean color
    let deep_color = vec3<f32>(0.0, 0.1, 0.3);
    let shallow_color = vec3<f32>(0.0, 0.1, 0.8);

    // Simple depth-based blend using wave height
    let wave_height = in.world_position.y;
    let blend = clamp(wave_height, 0.0, 1.0);
    let base_color = mix(deep_color, shallow_color, blend);

    // Diffuse lighting
    let light_dir = normalize(vec3<f32>(1.0, 2.0, 1.0));
    let normal = normalize(in.world_normal);
    let diffuse = max(dot(normal, light_dir), 0.0);

    // Specular (Blinn-Phong)
    let view_dir = normalize(vec3<f32>(0.0, 1.0, 0.0)); // approximate
    let half_dir = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, half_dir), 0.0), 64.0);

    let lit_color = base_color * (0.3 + 0.7 * diffuse) + vec3<f32>(1.0) * specular * 0.5;

    return vec4<f32>(lit_color, 1.0);
}