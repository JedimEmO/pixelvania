//! The low-resolution renderer.
//!
//! The world renders at 640 x 360. The colour buffer holds what things look
//! like, the light buffer how lit each pixel is, and the glow buffer additive light on top.
//! Separate background and foreground buffers hold scenery with its own distant lighting.
//! A composite pass combines them and scales the result to the window by a whole number.
//!
//! Each buffer is one pixel larger than the screen on every side. The cameras sit on whole
//! pixels, and the composite pass shifts its read by the camera's fraction of a pixel,
//! so the world stays on the pixel grid while the view still moves smoothly.
//!
//! What draws where is decided by render layers: the colour buffer sees layer 0, which is
//! where everything goes by default; lights belong on [`LIGHT_LAYER`] and glows on [`GLOW_LAYER`].
//! [`Light2d`] spawns both for you, and follows changes to it.
//! [`AdditiveImage`] adds a texture into the light or glow buffer, for light painted on the CPU.

use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    ShaderType, SpecializedMeshPipelineError, TextureFormat,
};
use bevy::render::view::{DebandDither, Tonemapping};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{
    AlphaMode2d, Material2d, Material2dKey, Material2dPipeline, Material2dPlugin,
};
use bevy::window::PrimaryWindow;

/// The game's resolution, in pixels.
pub const WIDTH: u32 = 640;
pub const HEIGHT: u32 = 360;
/// Extra pixels on each side of the buffers, for the sub-pixel camera.
const MARGIN: u32 = 1;
const BUFFER_W: u32 = WIDTH + 2 * MARGIN;
const BUFFER_H: u32 = HEIGHT + 2 * MARGIN;

pub const LIGHT_LAYER: usize = 1;
pub const GLOW_LAYER: usize = 2;
const SCREEN_LAYER: usize = 3;
pub const BACKGROUND_LAYER: usize = 4;
pub const FOREGROUND_LAYER: usize = 5;

/// The light buffer stores light divided by this, so a pixel can be lit to twice full brightness.
const LIGHT_RANGE: f32 = 2.0;

pub struct PixelRenderPlugin;

impl Plugin for PixelRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<CompositeMaterial>::default(),
            Material2dPlugin::<LightMaterial>::default(),
            Material2dPlugin::<AdditiveImage>::default(),
        ))
        .init_resource::<CameraFocus>()
        .init_resource::<Lighting>()
        .add_systems(Startup, setup)
        .add_systems(
            PostUpdate,
            (spawn_light_visuals, update_light_visuals)
                .chain()
                .before(TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (place_cameras, update_composite).before(TransformSystems::Propagate),
        );
    }
}

/// Where the view is centred, in world pixels. Set by the camera logic; may be fractional.
#[derive(Resource, Default)]
pub struct CameraFocus(pub Vec2);

/// Scene-wide lighting settings.
#[derive(Resource, Clone)]
pub struct Lighting {
    /// Light everywhere, before any light source adds to it.
    pub ambient: LinearRgba,
    /// Light falls off in this many steps per unit of brightness.
    pub bands: f32,
    /// How strongly a solid pixel's edge picks up the light next to it.
    pub rim: f32,
    /// The colour of empty space, before lighting.
    pub background: LinearRgba,
    /// Colour saturation of the final frame: 1 as drawn, less towards grey, more richer.
    /// The mood sets it: the Comedown drains the world's colour, the High deepens it.
    pub saturation: f32,
    /// How much colour the progression fade has taken from lit surfaces, 0 to 1.
    /// The glow buffer keeps its colour, so a faded world is grey stone under vivid lamps.
    pub fade: f32,
    /// The black curtain drawn while passing between rooms, 0 open to 1 closed.
    pub curtain: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient: LinearRgba::rgb(0.10, 0.09, 0.14),
            bands: 5.0,
            rim: 0.55,
            background: Color::srgb_u8(0x1a, 0x16, 0x26).to_linear(),
            saturation: 1.0,
            fade: 0.0,
            curtain: 0.0,
        }
    }
}

/// A light source. Spawns its light and glow quads as children.
#[derive(Component, Clone, Debug)]
#[require(Transform, Visibility)]
pub struct Light2d {
    pub color: LinearRgba,
    /// Radius in pixels.
    pub radius: f32,
    /// Brightness at the centre. 1.0 shows a surface's own colour; up to 2.0 is allowed.
    pub strength: f32,
    /// Strength of the hot glow at the centre. 0 for none.
    pub glow: f32,
}

#[derive(Component)]
struct WorldCamera;

/// The camera that draws the composite to the window. Debug overlays attach here.
#[derive(Component)]
pub struct ScreenCamera;

#[derive(Resource)]
struct Composite(Handle<CompositeMaterial>);

/// All buffers are sRGB. Bevy gives a plain `Rgba8Unorm` camera target an sRGB view as
/// well, which WebGL2 cannot do; and sRGB storage keeps more precision in dark light values.
fn target_image() -> Image {
    Image::new_target_texture(BUFFER_W, BUFFER_H, TextureFormat::Rgba8UnormSrgb, None)
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut composites: ResMut<Assets<CompositeMaterial>>,
    lighting: Res<Lighting>,
) {
    let color = images.add(target_image());
    let light = images.add(target_image());
    let glow = images.add(target_image());
    let background = images.add(target_image());
    let foreground = images.add(target_image());

    let ambient = lighting.ambient * (1.0 / LIGHT_RANGE);
    let buffers = [
        (background.clone(), BACKGROUND_LAYER, Color::NONE, -5),
        (foreground.clone(), FOREGROUND_LAYER, Color::NONE, -4),
        (color.clone(), 0, Color::NONE, -3),
        (
            light.clone(),
            LIGHT_LAYER,
            Color::LinearRgba(ambient.with_alpha(1.0)),
            -2,
        ),
        (glow.clone(), GLOW_LAYER, Color::BLACK, -1),
    ];
    for (image, layer, clear, order) in buffers {
        commands.spawn((
            Name::new("world camera"),
            WorldCamera,
            Camera2d,
            Camera {
                order,
                clear_color: ClearColorConfig::Custom(clear),
                ..default()
            },
            RenderTarget::Image(image.into()),
            RenderLayers::layer(layer),
            Msaa::Off,
            Tonemapping::None,
            DebandDither::Disabled,
        ));
    }

    let material = composites.add(CompositeMaterial {
        params: CompositeParams::default(),
        color,
        light,
        glow,
        background,
        foreground,
    });
    commands.insert_resource(Composite(material.clone()));
    commands.spawn((
        Name::new("screen camera"),
        ScreenCamera,
        Camera2d,
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        RenderLayers::layer(SCREEN_LAYER),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
    ));
    // One quad larger than any window; the shader works from the pixel position alone.
    commands.spawn((
        Name::new("composite"),
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(material),
        Transform::from_scale(Vec3::splat(100_000.0)),
        RenderLayers::layer(SCREEN_LAYER),
    ));
}

fn place_cameras(focus: Res<CameraFocus>, mut cameras: Query<&mut Transform, With<WorldCamera>>) {
    let snapped = focus.0.round();
    for mut t in &mut cameras {
        t.translation = snapped.extend(t.translation.z);
    }
}

fn update_composite(
    focus: Res<CameraFocus>,
    lighting: Res<Lighting>,
    window: Single<&Window, With<PrimaryWindow>>,
    composite: Res<Composite>,
    mut materials: ResMut<Assets<CompositeMaterial>>,
) {
    let size = window.physical_size();
    let scale = (size.x / WIDTH).min(size.y / HEIGHT).max(1);
    let origin = (size - UVec2::new(WIDTH, HEIGHT) * scale) / 2;
    let frac = focus.0 - focus.0.round();
    let params = CompositeParams {
        origin: origin.as_vec2(),
        scale: scale as f32,
        bands: lighting.bands,
        // Texture rows run down while world y runs up, hence the flipped y.
        offset: Vec2::new(frac.x, -frac.y),
        rim: lighting.rim,
        light_range: LIGHT_RANGE,
        background: lighting.background.to_vec4(),
        saturation: lighting.saturation,
        fade: lighting.fade,
        curtain: lighting.curtain,
    };
    // Only touch the asset when something changed, so it is not re-uploaded every frame.
    if materials
        .get(&composite.0)
        .is_some_and(|m| m.params != params)
        && let Some(mut m) = materials.get_mut(&composite.0)
    {
        m.params = params;
    }
}

#[derive(ShaderType, Clone, Copy, Default, PartialEq, Debug)]
struct CompositeParams {
    origin: Vec2,
    scale: f32,
    bands: f32,
    offset: Vec2,
    rim: f32,
    light_range: f32,
    background: Vec4,
    saturation: f32,
    fade: f32,
    curtain: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct CompositeMaterial {
    #[uniform(0)]
    params: CompositeParams,
    #[texture(1)]
    color: Handle<Image>,
    #[texture(2)]
    light: Handle<Image>,
    #[texture(3)]
    glow: Handle<Image>,
    #[texture(4)]
    background: Handle<Image>,
    #[texture(5)]
    foreground: Handle<Image>,
}

impl Material2d for CompositeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/composite.wesl".into()
    }
}

#[derive(ShaderType, Clone, Copy, Default, Debug)]
struct LightParams {
    color: Vec4,
    /// Exponent of the falloff from centre to edge. Higher is tighter.
    falloff: f32,
}

/// A soft disc of additive light.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct LightMaterial {
    #[uniform(0)]
    params: LightParams,
}

impl Material2d for LightMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/light.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        blend_additive(descriptor);
        Ok(())
    }
}

/// Adds the fragment's colour to the buffer instead of blending over it.
fn blend_additive(descriptor: &mut RenderPipelineDescriptor) {
    let add = BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    };
    if let Some(fragment) = descriptor.fragment.as_mut() {
        for target in fragment.targets.iter_mut().flatten() {
            target.blend = Some(BlendState {
                color: add,
                alpha: add,
            });
        }
    }
}

/// A texture added into the buffer its render layer draws to; its alpha is ignored.
/// Put it on [`GLOW_LAYER`] for light that glows, or on [`LIGHT_LAYER`] for light that lights.
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct AdditiveImage {
    #[texture(0)]
    #[sampler(1)]
    pub image: Handle<Image>,
}

impl Material2d for AdditiveImage {
    fn fragment_shader() -> ShaderRef {
        "shaders/additive_image.wesl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        blend_additive(descriptor);
        Ok(())
    }
}

/// The quads drawing a [`Light2d`], kept so changes to the light reach them.
#[derive(Component)]
struct LightVisuals {
    pool: (Entity, Handle<LightMaterial>),
    glow: Option<(Entity, Handle<LightMaterial>)>,
}

fn pool_color(light: &Light2d) -> Vec4 {
    (light.color * (light.strength / LIGHT_RANGE)).to_vec4()
}

fn glow_radius(light: &Light2d) -> f32 {
    (light.radius * 0.15).max(6.0)
}

fn spawn_light_visuals(
    mut commands: Commands,
    lights: Query<(Entity, &Light2d), Added<Light2d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LightMaterial>>,
) {
    for (entity, light) in &lights {
        let quad = meshes.add(Rectangle::new(2.0, 2.0));
        let pool = materials.add(LightMaterial {
            params: LightParams {
                color: pool_color(light),
                falloff: 1.6,
            },
        });
        let pool_quad = commands
            .spawn((
                Mesh2d(quad.clone()),
                MeshMaterial2d(pool.clone()),
                Transform::from_scale(Vec3::splat(light.radius)),
                RenderLayers::layer(LIGHT_LAYER),
                ChildOf(entity),
            ))
            .id();
        let glow = (light.glow > 0.0).then(|| {
            let glow = materials.add(LightMaterial {
                params: LightParams {
                    color: (light.color * light.glow).to_vec4(),
                    falloff: 2.5,
                },
            });
            let quad = commands
                .spawn((
                    Mesh2d(quad),
                    MeshMaterial2d(glow.clone()),
                    Transform::from_scale(Vec3::splat(glow_radius(light))),
                    RenderLayers::layer(GLOW_LAYER),
                    ChildOf(entity),
                ))
                .id();
            (quad, glow)
        });
        commands.entity(entity).insert(LightVisuals {
            pool: (pool_quad, pool),
            glow,
        });
    }
}

/// Carries changes to a light's colour, strength and radius over to its quads.
fn update_light_visuals(
    lights: Query<(&Light2d, &LightVisuals), Changed<Light2d>>,
    mut materials: ResMut<Assets<LightMaterial>>,
    mut transforms: Query<&mut Transform>,
) {
    for (light, visuals) in &lights {
        let (quad, pool) = &visuals.pool;
        if let Some(mut m) = materials.get_mut(pool) {
            m.params.color = pool_color(light);
        }
        if let Ok(mut t) = transforms.get_mut(*quad) {
            t.scale = Vec3::splat(light.radius);
        }
        if let Some((quad, glow)) = &visuals.glow {
            if let Some(mut m) = materials.get_mut(glow) {
                m.params.color = (light.color * light.glow).to_vec4();
            }
            if let Ok(mut t) = transforms.get_mut(*quad) {
                t.scale = Vec3::splat(glow_radius(light));
            }
        }
    }
}
