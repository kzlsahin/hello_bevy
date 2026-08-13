use avian3d::prelude::*;
use bevy::prelude::*;

use crate::ocean_waves::wave_height;

const GRAVITY: f32 = 9.81;

/// Marks an entity as a floating body driven by the ocean surface. Rigid-body integration,
/// collision, mass properties, and damping are all handled by Avian — see the `RigidBody`,
/// `Collider`, `ColliderDensity`, `LinearDamping`, and `AngularDamping` components spawned
/// alongside this one in `main.rs`. This component only carries what's needed to integrate
/// submerged volume against the waves each physics step.
///
/// The hull volume is divided into a 3D grid of cells; each cell center is transformed to world
/// space and tested against the local wave height at its own (x, z). Cells below the surface
/// contribute buoyant force via Archimedes' principle (`water_density * g * cell_volume`),
/// applied through Avian's `Forces::apply_force_at_point` so Avian derives the resulting torque
/// (and integrates it) itself. Testing the whole volume (not just a fixed "bottom" face) keeps
/// the model correct at any orientation.
#[derive(Component, Debug, Clone)]
pub struct Buoyant {
    pub half_extents: Vec3,
    /// kg/m³. Fresh water ≈ 1000, seawater ≈ 1025. Equilibrium submerged volume is
    /// `mass / water_density` (Archimedes), so how deep the object rides falls out of the
    /// entity's `Mass`/`ColliderDensity` vs. this and the hull volume.
    pub water_density: f32,
    /// Resolution of the 3D grid used to integrate submerged volume across the hull.
    pub sample_grid: (u32, u32, u32),
}

impl Buoyant {
    pub fn new(half_extents: Vec3) -> Self {
        Self {
            half_extents,
            water_density: 1000.0,
            sample_grid: (8, 4, 4),
        }
    }
}

impl Default for Buoyant {
    fn default() -> Self {
        // 6x1x1 m hull (6:1 length/width).
        Self::new(Vec3::new(3.0, 1.0, 0.5))
    }
}

pub struct BuoyancyPlugin;

impl Plugin for BuoyancyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, apply_buoyancy);
    }
}

fn apply_buoyancy(time: Res<Time>, mut query: Query<(&Position, &Rotation, &Buoyant, Forces)>) {
    let t = time.elapsed_secs();

    for (position, rotation, buoy, mut forces) in &mut query {
        let (nx, ny, nz) = buoy.sample_grid;
        let size = buoy.half_extents * 2.0;
        let cell_size = Vec3::new(size.x / nx as f32, size.y / ny as f32, size.z / nz as f32);
        let cell_force = buoy.water_density * GRAVITY * cell_size.x * cell_size.y * cell_size.z;

        for ix in 0..nx {
            let local_x = -buoy.half_extents.x + cell_size.x * (ix as f32 + 0.5);
            for iy in 0..ny {
                let local_y = -buoy.half_extents.y + cell_size.y * (iy as f32 + 0.5);
                for iz in 0..nz {
                    let local_z = -buoy.half_extents.z + cell_size.z * (iz as f32 + 0.5);
                    let world = position.0 + rotation * Vec3::new(local_x, local_y, local_z);

                    let surface_y = wave_height(Vec2::new(world.x, world.z), t);
                    if world.y >= surface_y {
                        continue;
                    }

                    forces.apply_force_at_point(Vec3::new(0.0, cell_force, 0.0), world);
                }
            }
        }
    }
}
