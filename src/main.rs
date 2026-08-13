mod buoyancy;
mod ocean_material;
mod ocean_waves;
mod shaders;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use buoyancy::{Buoyant, BuoyancyPlugin};
use ocean_material::OceanMaterial;
use ocean_material::OceanMaterialUniform;

fn main() {
    App::new()
    // Pins the asset root to the crate directory at compile time so `assets/` is found
    // whether the app is launched via `cargo run` or by running target/debug/hello_bevy.exe
    // directly (bevy otherwise falls back to looking next to the .exe in the latter case).
    .add_plugins(DefaultPlugins.set(AssetPlugin {
        file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.60, 0.72, 0.87))) // must match horizon fog in ocean.wgsl
    .add_plugins(FreeCameraPlugin)
    .add_plugins(MaterialPlugin::<OceanMaterial>::default())
    .add_plugins(PhysicsPlugins::default())
    .add_plugins(BuoyancyPlugin)
    .add_systems(Startup, setup)
    .add_systems(Update, update_ocean_uniforms)
    .add_plugins(CameraPlugin)
    .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OceanMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(( DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.8), // warm sunlight
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -45.0_f32.to_radians(),
            45.0_f32.to_radians(),
            0.0,
        ))
    ));

    let mesh = meshes.add(Plane3d::default().mesh().size(500.0, 500.0).subdivisions(500).build());

    let material = materials.add(OceanMaterial {
         params: OceanMaterialUniform {
            camera_pos: Vec3::ZERO,
            time: 0.0,
        },
    });

    // Spawn a plane to represent the "empty field"
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d (material),
    Transform::from_xyz(0.0, 0.0, 0.0)
    ));

    // Placeholder floating object — swap this mesh for a real boat/buoy asset later.
    // Half-extents drive both the buoyancy sample footprint and this box's size.
    // Rigid-body integration, mass, and damping are all Avian's job here; buoyancy.rs only
    // supplies the per-frame force from integrating submerged volume against the waves.
    let buoy = Buoyant::default();
    let size = buoy.half_extents * 2.0;
    let float_mesh = meshes.add(Cuboid::from_size(size));
    let float_material = standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.35, 0.2),
        ..default()
    });
    commands.spawn((
        Mesh3d(float_mesh),
        MeshMaterial3d(float_material),
        Transform::from_xyz(15.0, 1.5, 0.0),
        buoy,
        RigidBody::Dynamic,
        Collider::cuboid(size.x, size.y, size.z),
        ColliderDensity(500.0), // wood-like density; combined with hull volume gives mass/inertia
        LinearDamping(0.7),
        AngularDamping(1.3),
    ));
}

// Plugin that spawns the camera.
struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
        // This component stores all camera settings and state, which is used by the FreeCameraPlugin to
        // control it. These properties can be changed at runtime, but beware the controller system is
        // constantly using and modifying those values unless the enabled field is false.
        FreeCamera {
            sensitivity: 0.2,
            friction: 25.0,
            walk_speed: 3.0,
            run_speed: 9.0,
            ..default()
        },
    ));
}

fn update_ocean_uniforms(
    camera_query: Query<&Transform, With<Camera>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<OceanMaterial>>,
) {
    let cam_tf = camera_query.single().expect("camera is required.");
    for (_, mat) in materials.iter_mut() {
        mat.params.camera_pos = cam_tf.translation;
        mat.params.time = time.elapsed_secs();
    }
}