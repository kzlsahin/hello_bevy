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
    // Wind from +X. All directions pre-normalized at spread angles; Q decreases with wavelength
    // to prevent stripes. Speeds follow deep-water dispersion: c ≈ 1.25 * sqrt(lambda).
    //                   dir (normalized)            amp   λ(m)  speed   Q    phase
    return array<WaveParams, 11>(
        WaveParams(vec2<f32>( 1.0000,  0.0000 ),    0.80, 28.0,  6.6, 0.55, 0.0 ),  // main swell
        WaveParams(vec2<f32>( 0.9659,  0.2588 ),    0.55, 45.0,  8.4, 0.40, 1.4 ),  // swell +15°
        WaveParams(vec2<f32>( 0.8660, -0.5000 ),    0.30, 18.0,  5.3, 0.40, 0.7 ),  // cross -30°
        WaveParams(vec2<f32>( 0.9659, -0.2588 ),    0.22, 22.0,  5.9, 0.38, 2.1 ),  // cross -15°
        WaveParams(vec2<f32>( 0.7071,  0.7071 ),    0.14, 10.0,  4.0, 0.30, 0.3 ),  // chop  +45°
        WaveParams(vec2<f32>( 0.8660,  0.5000 ),    0.11,  8.0,  3.5, 0.28, 1.8 ),  // chop  +30°
        WaveParams(vec2<f32>( 0.7071, -0.7071 ),    0.08,  5.5,  2.9, 0.20, 2.5 ),  // chop  -45°
        WaveParams(vec2<f32>(-0.7071,  0.7071 ),    0.05,  4.0,  2.5, 0.25, 0.9 ),  // reflected
        WaveParams(vec2<f32>( 0.9239,  0.3827 ),    0.04,  2.8,  2.1, 0.32, 1.6 ),  // ripple +22.5°
        WaveParams(vec2<f32>( 0.5000, -0.8660 ),    0.03,  2.0,  1.8, 0.10, 0.5 ),  // ripple -60°
        WaveParams(vec2<f32>(-0.5000,  0.8660 ),    0.02,  1.5,  1.5, 0.08, 3.0 ),  // micro  +120°
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

// Precise analytical normal with distance-based LOD.
// Waves are ordered largest→smallest, so trimming the tail drops sub-pixel detail first.
//   < 20 m : all 11 waves
//   20–60 m : 8 waves  (skip λ < 3 m ripples)
//   > 60 m : 5 waves  (skip λ < 8 m chop + ripples)
fn precise_normal(xz: vec2<f32>, time: f32, dist_sq: f32) -> vec3<f32> {
    let waves = ocean_waves();
    var count = WAVE_COUNT;
    if dist_sq > 3600.0 { count = 5u; }
    else if dist_sq > 400.0 { count = 8u; }
    var n = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < count; i++) {
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

// Waves beyond this index have wavelengths < 3 m: negligible geometry displacement,
// so the vertex stage only runs the large/medium waves. The fragment stage handles
// the remainder as normal detail (see precise_normal).
const VERTEX_WAVE_COUNT: u32 = 8u;

@vertex
fn vertex(
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) uv:       vec2<f32>,
) -> OceanVertexOutput {
    let waves = ocean_waves();
    var pos = position;
    var n   = vec3<f32>(0.0, 0.0, 0.0);
    for (var i = 0u; i < VERTEX_WAVE_COUNT; i++) {
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
    let to_cam  = material.camera_pos - in.world_position.xyz;
    let dist_sq = dot(to_cam, to_cam);
    let N = precise_normal(in.orig_xz, material.time, dist_sq);
    let V = normalize(to_cam);
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

    let lit = mix(water_color, reflection, fresnel);

    // Foam on breaking crests.
    // N.y drops toward 0 where the surface is steepest (Gerstner crest about to overturn).
    // Only show foam above mean water level so troughs stay clean.
    let steepness  = 1.0 - N.y;
    let foam_t     = clamp((steepness - 0.15) / 0.15, 0.0, 1.0);
    let above_mean = clamp(in.world_position.y * 1.2, 0.0, 1.0);
    let foam       = foam_t * above_mean;

    return vec4<f32>(mix(lit, vec3<f32>(0.92, 0.95, 1.0), foam), 1.0);
}
