pub mod hook;

use std::f32::consts::PI;

use bevy::{
    app::PluginsState,
    asset::RenderAssetUsages,
    color::palettes::css::SILVER,
    light::DirectionalLightShadowMap,
    log::{Level, LogPlugin},
    prelude::*,
};

use rcade_plugin_input_classic::ClassicController;

use wasm_bindgen::prelude::*;

use wgpu::{Extent3d, TextureDimension, TextureFormat};

use crate::hook::{RcadePluginExt, get_offscreen_canvas};

#[wasm_bindgen]

pub struct BevyApp {
    app: App,
}

#[wasm_bindgen(start)]

pub async fn start() {
    console_error_panic_hook::set_once();

    let mut app = BevyApp::new().await;

    loop {
        app.update();

        gloo_timers::future::sleep(std::time::Duration::from_nanos(0)).await;
    }
}

impl BevyApp {
    pub async fn new() -> Self {
        let mut app = App::new();

        let canvas = get_offscreen_canvas().unwrap();

        let controller = ClassicController::acquire().await.unwrap();

        app.add_plugins(
            DefaultPlugins
                .with_rcade(canvas.clone())
                .await
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    level: Level::WARN,

                    ..Default::default()
                }),
        )
        .insert_resource(DirectionalLightShadowMap { size: 512 })
        .insert_non_send_resource(controller)
        .insert_non_send_resource(canvas)
        .add_systems(PreStartup, hook::setup_added_window)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .add_systems(Update, camera_control_system);

        BevyApp { app }
    }

    pub fn update(&mut self) {
        if self.app.plugins_state() != PluginsState::Cleaned {
            if self.app.plugins_state() == PluginsState::Ready {
                self.app.finish();

                self.app.cleanup();
            }
        } else {
            self.app.update();
        }
    }
}

#[derive(Component)]

pub struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;

const EXTRUSION_X_EXTENT: f32 = 16.0;

const Z_EXTENT: f32 = 5.0;

pub fn setup(
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,

    mut images: ResMut<Assets<Image>>,

    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),

        alpha_mode: AlphaMode::Opaque,

        ..default()
    });

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
        meshes.add(Segment3d::default()),
        meshes.add(Polyline3d::new(vec![
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(0.0, 0.5, 0.0),
        ])),
    ];

    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                2.0,
                Z_EXTENT / 2.,
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
        ));
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands.spawn((
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                -EXTRUSION_X_EXTENT / 2.
                    + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                2.0,
                -Z_EXTENT / 2.,
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
        ));
    }

    commands.spawn((
        PointLight {
            shadows_enabled: true,

            intensity: 10_000_000.,

            range: 100.0,

            shadow_depth_bias: 0.2,

            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // ground plane

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_3, // 60 degrees in radians

            ..default()
        }),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        Msaa::Off,
    ));
}

pub fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/// Creates a colorful test pattern

pub fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];

    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;

        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);

        palette.rotate_right(4);
    }

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,

            height: TEXTURE_SIZE as u32,

            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

pub fn camera_control_system(
    controller: NonSend<ClassicController>,

    mut camera_query: Query<&mut Transform, With<Camera3d>>,

    time: Res<Time>,
) {
    let state = controller.state();

    if let Ok(mut transform) = camera_query.single_mut() {
        let move_speed = 5.0 * time.delta_secs();

        let rotate_speed = 2.0 * time.delta_secs();

        // Player 1: Movement (WASD-style)

        let forward = transform.forward();

        let right = transform.right();

        if state.player1_up {
            transform.translation += forward * move_speed;
        }

        if state.player1_down {
            transform.translation -= forward * move_speed;
        }

        if state.player1_left {
            transform.translation -= right * move_speed;
        }

        if state.player1_right {
            transform.translation += right * move_speed;
        }

        // Player 2: Rotation (look around)

        if state.player2_left {
            transform.rotate_y(rotate_speed);
        }

        if state.player2_right {
            transform.rotate_y(-rotate_speed);
        }

        if state.player2_up {
            transform.rotate_local_x(rotate_speed);
        }

        if state.player2_down {
            transform.rotate_local_x(-rotate_speed);
        }
    }
}
