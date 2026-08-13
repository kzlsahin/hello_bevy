use bevy::prelude::*;

use crate::ocean_waves::wave_height;

const GRAVITY: f32 = 9.81;

/// Marks an entity as a floating rigid body driven by the ocean surface.
/// Buoyancy is sampled at the 4 corners of the footprint defined by `half_extents`,
/// so torque (pitch/roll) emerges from uneven submersion across the waves rather
/// than being approximated from a single point.
#[derive(Component, Debug, Clone)]
pub struct Buoyant {
    pub mass: f32,
    /// Diagonal moment of inertia (kg·m²) about the body's local x/y/z axes.
    /// Off-axis (product of inertia) terms are assumed negligible for this shape.
    pub inertia: Vec3,
    pub half_extents: Vec3,
    /// > 1.0 makes the object ride higher (net buoyant force exceeds weight while
    /// fully submerged), settling into equilibrium partway out of the water.
    pub buoyancy_multiplier: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl Default for Buoyant {
    fn default() -> Self {
        Self {
            mass: 30.0,
            inertia: Vec3::new(8.0, 10.0, 8.0),
            half_extents: Vec3::new(1.0, 0.5, 1.0),
            buoyancy_multiplier: 1.15,
            linear_damping: 0.7,
            angular_damping: 1.3,
        }
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
        let corners_local = [
            Vec3::new(-buoy.half_extents.x, 0.0, -buoy.half_extents.z),
            Vec3::new(buoy.half_extents.x, 0.0, -buoy.half_extents.z),
            Vec3::new(-buoy.half_extents.x, 0.0, buoy.half_extents.z),
            Vec3::new(buoy.half_extents.x, 0.0, buoy.half_extents.z),
        ];

        // Force a fully-submerged corner would contribute, spread evenly over the 4 corners.
        let max_submersion = buoy.half_extents.y * 2.0;
        let force_per_corner_at_full_submersion =
            (buoy.mass * GRAVITY * buoy.buoyancy_multiplier) / (4.0 * max_submersion.max(0.001));

        let mut net_force = Vec3::new(0.0, -buoy.mass * GRAVITY, 0.0);
        let mut net_torque = Vec3::ZERO;

        for local in corners_local {
            let world = transform.transform_point(local);
            let surface_y = wave_height(Vec2::new(world.x, world.z), t);
            let submersion = (surface_y - world.y).clamp(0.0, max_submersion);
            if submersion <= 0.0 {
                continue;
            }

            let force = Vec3::new(0.0, force_per_corner_at_full_submersion * submersion, 0.0);
            net_force += force;
            net_torque += (world - transform.translation).cross(force);
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
