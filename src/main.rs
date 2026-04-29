mod ocean_material;
mod shaders;

use bevy::prelude::*;
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState};
use ocean_material::OceanMaterial;
use ocean_material::OceanMaterialUniform;

fn main() {
    App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(FreeCameraPlugin) // <- add this
    .add_plugins(MaterialPlugin::<OceanMaterial>::default()) // <- add this
    .add_systems(Startup, setup)
    .add_systems(Update, update_ocean_uniforms) // <- add update system
    .add_plugins(CameraPlugin)
    .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<OceanMaterial>>,
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

    let mesh = meshes.add(Plane3d::default().mesh().size(100.0, 100.0).subdivisions(500).build());

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