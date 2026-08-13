use bevy::prelude::*;

use crate::ocean_waves::wave_height;

const GRAVITY: f32 = 9.81;

/// Marks an entity as a floating rigid body driven by the ocean surface.
/// The hull volume is divided into a 3D grid of cells; each cell center is transformed to world
/// space and tested against the local wave height at its own (x, z). Cells below the surface
/// contribute buoyant force via Archimedes' principle (`water_density * g * cell_volume`).
/// Testing the whole volume (not just a fixed "bottom" face) keeps the model correct at any
/// orientation, so torque genuinely restores the hull toward upright instead of only doing so
/// for small tilts.
#[derive(Component, Debug, Clone)]
pub struct Buoyant {
    pub mass: f32,
    /// Diagonal moment of inertia (kg·m²) about the body's local x/y/z axes.
    /// Off-axis (product of inertia) terms are assumed negligible for this shape.
    pub inertia: Vec3,
    pub half_extents: Vec3,
    /// kg/m³. Fresh water ≈ 1000, seawater ≈ 1025. Equilibrium submerged volume is
    /// `mass / water_density` (Archimedes), so how deep the object rides falls out of
    /// `mass` vs. this and the hull volume rather than a hand-tuned multiplier.
    pub water_density: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    /// Resolution of the 3D grid used to integrate submerged volume across the hull.
    pub sample_grid: (u32, u32, u32),
}

impl Buoyant {
    /// Builds a `Buoyant` for a solid cuboid hull, deriving its moment of inertia from `mass`
    /// and `half_extents` so the two stay physically consistent when either is tuned.
    pub fn new(mass: f32, half_extents: Vec3) -> Self {
        let size = half_extents * 2.0;
        let inertia = Vec3::new(
            mass / 12.0 * (size.y * size.y + size.z * size.z),
            mass / 12.0 * (size.x * size.x + size.z * size.z),
            mass / 12.0 * (size.x * size.x + size.y * size.y),
        );
        Self {
            mass,
            inertia,
            half_extents,
            water_density: 1000.0,
            linear_damping: 0.7,
            angular_damping: 1.3,
            sample_grid: (4, 4, 4),
        }
    }
}

impl Default for Buoyant {
    fn default() -> Self {
        // 2x1x2 m hull at 2000 kg -> density 500 kg/m^3 (like wood), rides about half-submerged.
        Self::new(2000.0, Vec3::new(1.0, 0.5, 1.0))
    }
}

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct LinearVelocity(pub Vec3);

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct AngularVelocity(pub Vec3);

pub struct BuoyancyPlugin;

impl Plugin for BuoyancyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_buoyancy);
    }
}

fn apply_buoyancy(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut LinearVelocity, &mut AngularVelocity, &Buoyant)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let t = time.elapsed_secs();

    for (mut transform, mut lin_vel, mut ang_vel, buoy) in &mut query {
        let (nx, ny, nz) = buoy.sample_grid;
        let size = buoy.half_extents * 2.0;
        let cell_size = Vec3::new(size.x / nx as f32, size.y / ny as f32, size.z / nz as f32);
        let cell_volume = cell_size.x * cell_size.y * cell_size.z;
        let cell_force = buoy.water_density * GRAVITY * cell_volume;

        let mut net_force = Vec3::new(0.0, -buoy.mass * GRAVITY, 0.0);
        let mut net_torque = Vec3::ZERO;

        for ix in 0..nx {
            let local_x = -buoy.half_extents.x + cell_size.x * (ix as f32 + 0.5);
            for iy in 0..ny {
                let local_y = -buoy.half_extents.y + cell_size.y * (iy as f32 + 0.5);
                for iz in 0..nz {
                    let local_z = -buoy.half_extents.z + cell_size.z * (iz as f32 + 0.5);
                    let world = transform.transform_point(Vec3::new(local_x, local_y, local_z));

                    let surface_y = wave_height(Vec2::new(world.x, world.z), t);
                    if world.y >= surface_y {
                        continue;
                    }

                    let force = Vec3::new(0.0, cell_force, 0.0);
                    net_force += force;
                    net_torque += (world - transform.translation).cross(force);
                }
            }
        }

        // Linear drag approximates water resistance.
        net_force -= lin_vel.0 * buoy.linear_damping * buoy.mass;

        let linear_accel = net_force / buoy.mass;
        lin_vel.0 += linear_accel * dt;
        transform.translation += lin_vel.0 * dt;

        // Torque -> angular acceleration in the body's local (principal-axis) frame, then
        // back to world space to integrate. Ignores gyroscopic cross-coupling between axes.
        let local_torque = transform.rotation.inverse() * net_torque;
        let local_angular_accel = local_torque / buoy.inertia;
        let world_angular_accel = transform.rotation * local_angular_accel;

        let damped = ang_vel.0 * buoy.angular_damping * dt;
        ang_vel.0 += world_angular_accel * dt;
        ang_vel.0 -= damped;

        if ang_vel.0 != Vec3::ZERO {
            transform.rotation = (Quat::from_scaled_axis(ang_vel.0 * dt) * transform.rotation).normalize();
        }
    }
}
