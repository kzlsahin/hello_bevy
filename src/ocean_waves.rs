use bevy::prelude::*;
use std::f32::consts::TAU;

// Rust port of the two dominant waves from `ocean_waves()` in assets/shaders/ocean.wgsl.
// Used by the CPU-side buoyancy solver, which needs to sample wave height outside the
// render pipeline. Keep these two entries in sync with the shader's first two rows.
pub struct WaveParams {
    pub direction: Vec2,
    pub amplitude: f32,
    pub wavelength: f32,
    pub speed: f32,
    pub phase: f32,
}

pub fn buoyancy_waves() -> [WaveParams; 2] {
    [
        WaveParams { direction: Vec2::new(1.0000, 0.0000), amplitude: 0.80, wavelength: 28.0, speed: 6.6, phase: 0.0 }, // main swell
        WaveParams { direction: Vec2::new(0.9659, 0.2588), amplitude: 0.55, wavelength: 45.0, speed: 8.4, phase: 1.4 }, // swell +15°
    ]
}

// Height of the wave surface above y=0 at world-space (x, z). Ignores the shader's horizontal
// (Gerstner) displacement, i.e. treats world xz as if it were the undisplaced vertex xz — an
// approximation that's accurate enough for buoyancy sampling at this steepness.
pub fn wave_height(xz: Vec2, time: f32) -> f32 {
    let mut y = 0.0;
    for wave in buoyancy_waves() {
        let k = TAU / wave.wavelength;
        let omega = wave.speed * k;
        let theta = k * wave.direction.dot(xz) - omega * time + wave.phase;
        y += wave.amplitude * theta.sin();
    }
    y
}
