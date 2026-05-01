#import bevy_pbr::mesh_view_bindings::view

struct OceanMaterial {
    camera_pos: vec3<f32>,
    time: f32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: OceanMaterial;

struct WaveParams {
    direction: vec2<f32>,  // must be pre-normalized (normalize() is not const in WGSL)
    amplitude:  f32,
    wavelength: f32,
    speed:      f32,
    steepness:  f32,
    phase:      f32,
};

struct TrochoidalResult {
    position: vec3<f32>,
    normal: vec3<f32>,
};

// --- Single source of truth for all wave parameters ---
// To add/remove/tune a wave: edit this table only.
// Pre-normalize directions by hand:
//   normalize(vec2( 0.3, -1.0)) ≈ vec2( 0.2874, -0.9578)
//   normalize(vec2(-0.3,  1.0)) ≈ vec2(-0.2874,  0.9578)
// When adding/removing a wave: change WAVE_COUNT and the two literals on the next two lines together.
const WAVE_COUNT: u32 = 11u;
fn ocean_waves() -> array<WaveParams, 11> {
    //                    direction                   amp    λ(m)  speed  Q     phase
    return array<WaveParams, 11>(
        WaveParams(vec2<f32>( 1.0,     0.0    ),     0.80, 23.0,  8.0, 0.7, 0.0),
        WaveParams(vec2<f32>( 0.6,     0.8    ),     0.50, 60.0, 15.0, 0.3, 1.2),
        WaveParams(vec2<f32>( 1.0,     0.0    ),     0.40, 20.0,  6.0, 0.6, 0.7),
        WaveParams(vec2<f32>(-0.8,     0.6    ),     0.10, 10.0,  6.0, 0.6, 2.4),
        WaveParams(vec2<f32>( 0.8,    -0.6    ),     0.10, 10.0,  6.0, 0.6, 1.8),
        WaveParams(vec2<f32>( 0.2874, -0.9578 ),     0.05,  1.0,  0.4, 0.9, 0.2),
        WaveParams(vec2<f32>(-0.2874,  0.9578 ),     0.05,  1.0,  0.5, 0.8, 0.8),
        WaveParams(vec2<f32>(-0.2874,  0.9578 ),     0.04,  5.0,  0.6, 0.8, 0.8),
        WaveParams(vec2<f32>(0.2874,  0.9578 ),     0.05,  1.0,  0.3, 0.9, 0.4),
        WaveParams(vec2<f32>(0.2874,  0.9578 ),     0.05,  10.0,  0.4, 0.9, 0.4),
        WaveParams(vec2<f32>(0.2874,  0.9578 ),     0.05,  3.0,  0.5, 0.9, 0.4),
    );
}
// ------------------------------------------------------

fn trochoidal_wave(vertex: vec3<f32>, wave: WaveParams, time: f32) -> TrochoidalResult {
    let k     = 2.0 * 3.14159265359 / wave.wavelength;
    let omega = wave.speed * k;
    let theta = k * dot(wave.direction, vec2<f32>(vertex.x, vertex.z))
                - omega * time + wave.phase;
    let c  = cos(theta);
    let s  = sin(theta);
    let ka = k * wave.amplitude;

    var r: TrochoidalResult;
    r.position = vec3<f32>(
        vertex.x + wave.steepness * wave.amplitude * wave.direction.x * c,
        vertex.y + wave.amplitude * s,
        vertex.z + wave.steepness * wave.amplitude * wave.direction.y * c,
    );
    r.normal = vec3<f32>(
        -wave.direction.x * ka * c,
        1.0 - wave.steepness * ka * s,
        -wave.direction.y * ka * c,
    );
    return r;
}

// Normal-only variant — skips displacement; used per-fragment where displaced pos is unavailable.
fn wave_normal_only(xz: vec2<f32>, wave: WaveParams, time: f32) -> vec3<f32> {
    let k     = 2.0 * 3.14159265359 / wave.wavelength;
    let omega = wave.speed * k;
    let theta = k * dot(wave.direction, xz) - omega * time + wave.phase;
    let c  = cos(theta);
    let s  = sin(theta);
    let ka = k * wave.amplitude;
    return vec3<f32>(
        -wave.direction.x * ka * c,
        1.0 - wave.steepness * ka * s,
        -wave.direction.y * ka * c,
    );
}

// Precise analytical normal at an undisplaced grid position — avoids vertex interpolation blur.
fn precise_normal(xz: vec2<f32>, time: f32) -> vec3<f32> {
    let waves = ocean_waves();
    var n = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < WAVE_COUNT; i++) {
        n += wave_normal_only(xz, waves[i], time);
    }
    return normalize(n);
}

struct OceanVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position:      vec4<f32>,
    @location(1) world_normal:        vec3<f32>,
    @location(2) uv:                  vec2<f32>,
    @location(3) orig_xz:             vec2<f32>,
};

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
) -> OceanVertexOutput {
    let waves = ocean_waves();
    var pos = position;
    var n   = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < WAVE_COUNT; i++) {
        let r = trochoidal_wave(pos, waves[i], material.time);
        pos = r.position;
        n  += r.normal;
    }

    var out: OceanVertexOutput;
    out.clip_position  = view.clip_from_world * vec4<f32>(pos, 1.0);
    out.world_position = vec4<f32>(pos, 1.0);
    out.world_normal   = normalize(n);
    out.uv             = uv;
    out.orig_xz        = position.xz;
    return out;
}

@fragment
fn fragment(in: OceanVertexOutput) -> @location(0) vec4<f32> {
    let N = precise_normal(in.orig_xz, material.time);
    let V = normalize(material.camera_pos - in.world_position.xyz);
    // Derived from Transform::from_rotation(Quat::from_euler(XYZ, -45°, 45°, 0°))
    let L = vec3<f32>(0.5, 0.707, 0.5);

    // Schlick Fresnel (water n=1.33 → F0 ≈ 0.02)
    let NdotV   = max(dot(N, V), 0.0);
    let fresnel = 0.02 + 0.98 * pow(1.0 - NdotV, 5.0);

    // Sun specular (Blinn-Phong)
    let H        = normalize(L + V);
    let sun_spec = pow(max(dot(N, H), 0.0), 512.0) * vec3<f32>(1.0, 0.95, 0.85);

    // Water color: dark trough → bright teal crest
    let height_t = clamp(in.world_position.y * 0.5 + 0.5, 0.0, 1.0);
    let base_color = mix(vec3<f32>(0.01, 0.07, 0.22), vec3<f32>(0.02, 0.28, 0.48), height_t);

    let NdotL       = max(dot(N, L), 0.0);
    let water_color = base_color * (0.12 + 0.45 * NdotL);
    let reflection  = vec3<f32>(0.25, 0.45, 0.75) + sun_spec * 2.5;

    return vec4<f32>(mix(water_color, reflection, fresnel), 1.0);
}
