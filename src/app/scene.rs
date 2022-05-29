use eframe::{egui, Frame};
use three_d::*;


use crate::app::SDFViewerApp;
use crate::input::InputTranslator;

/// Renders the main 3D scene, containing the SDF object
pub struct SDFViewerAppScene {
    // === CONTEXT ===
    /// The 3D rendering context of the library we use to render the scene
    pub ctx: three_d::Context,
    // === SDF ===
    // TODO: The SDF object (reference to server/file...) to render
    pub volume: Model<IsourfaceMaterial>,
    // === CAMERA ===
    /// The 3D perspective camera
    pub camera: Camera,
    /// The 3D perspective camera controller
    pub camera_ctrl: OrbitControl,
    // === ENVIRONMENT ===
    /// The ambient light of the scene (hits everything, in all directions)
    pub light_ambient: AmbientLight,
    /// The directional lights of the scene
    pub lights_dir: Vec<DirectionalLight>,
    /// The input event translation helper
    pub input_translator: InputTranslator,
}

impl SDFViewerAppScene {
    pub fn new(ctx: three_d::Context) -> Self {
        let camera = Camera::new_perspective(
            &ctx,
            Viewport { x: 0, y: 0, width: 0, height: 0 }, // Updated at runtime
            vec3(0.25, -0.5, -2.0),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            degrees(45.0),
            0.1,
            1000.0,
        ).unwrap();
        let camera_ctrl = OrbitControl::new(*camera.target(), 1.0, 100.0);

        // Source: https://web.cs.ucdavis.edu/~okreylos/PhDStudies/Spring2000/ECS277/DataSets.html
        // TODO: SDF infrastructure (webserver and file drag&drop)
        let mut loaded = Loaded::default();
        loaded.insert_bytes("", include_bytes!("../../assets/Skull.vol").to_vec());
        let cpu_volume = loaded.vol("").unwrap();
        let mut volume = Model::new_with_material(
            &ctx,
            &CpuMesh::cube(),
            IsourfaceMaterial {
                // FIXME: Do NOT clip cube's inside triangles (or render inverted cube) to render the surface while inside
                // TODO: Variable cube size same as SDF's bounding box
                // FIXME: HACK: Use gl_FragDepth to interact with other objects of the scene
                // FIXME: Cube seams visible from far away?
                voxels: std::rc::Rc::new(Texture3D::new(&ctx, &cpu_volume.voxels).unwrap()),
                lighting_model: LightingModel::Blinn,
                size: cpu_volume.size,
                threshold: 0.15,
                color: Color::WHITE,
                roughness: 1.0,
                metallic: 0.0,
            },
        ).unwrap();
        volume.material.color = Color::new(25, 125, 25, 255);
        volume.set_transformation(Mat4::from_nonuniform_scale(
            0.5 * cpu_volume.size.x,
            0.5 * cpu_volume.size.y,
            0.5 * cpu_volume.size.z,
        ));

        let ambient = AmbientLight::new(&ctx, 0.4, Color::WHITE).unwrap();
        let directional1 =
            DirectionalLight::new(&ctx, 2.0, Color::WHITE, &vec3(-1.0, -1.0, -1.0)).unwrap();
        let directional2 =
            DirectionalLight::new(&ctx, 2.0, Color::WHITE, &vec3(1.0, 1.0, 1.0)).unwrap();

        Self {
            ctx,
            camera,
            camera_ctrl,
            volume,
            light_ambient: ambient,
            lights_dir: vec![directional1, directional2],
            input_translator: InputTranslator::default(),
        }
    }
}

impl SDFViewerAppScene {
    pub fn render(&mut self, _app: &SDFViewerApp, egui_ctx: &egui::Context, _frame: &mut Frame) {
        // === Draw Three-D scene ===
        let viewport_rect = egui_ctx.available_rect();
        let viewport = Viewport {
            x: (viewport_rect.min.x * egui_ctx.pixels_per_point()) as i32,
            y: (viewport_rect.min.y * egui_ctx.pixels_per_point()) as i32,
            width: (viewport_rect.width() * egui_ctx.pixels_per_point()) as u32,
            height: (viewport_rect.height() * egui_ctx.pixels_per_point()) as u32,
        };
        self.camera.set_viewport(viewport).unwrap();
        // Handle inputs
        let mut events = self.input_translator.translate_input_events(egui_ctx);
        // TODO: HACK: Swap left and right click for the camera controls
        self.camera_ctrl.handle_events(&mut self.camera, &mut events).unwrap();
        // Collect lights
        let mut lights = self.lights_dir.iter().map(|e| e as &dyn Light).collect::<Vec<&dyn Light>>();
        lights.push(&self.light_ambient);
        // Draw the scene to screen
        // TODO: Sub- and super-sampling (eframe's pixels_per_point?)!
        let full_screen_rect = egui_ctx.input().screen_rect.size();
        let screen = RenderTarget::screen(
            &self.ctx, (full_screen_rect.x * egui_ctx.pixels_per_point()) as u32,
            (full_screen_rect.y * egui_ctx.pixels_per_point()) as u32);
        // Clear with the same background color as the UI
        let bg_color = egui_ctx.style().visuals.window_fill();
        // FIXME: Clear not working on web platforms (egui tooltip still visible)
        screen.clear(ClearState::color_and_depth(
            bg_color.r() as f32 / 255., bg_color.g() as f32 / 255.,
            bg_color.b() as f32 / 255., 1.0, 1.0)).unwrap();
        // Now render the main scene
        screen.render_partially(ScissorBox::from(viewport), &self.camera,
                                &[&self.volume], lights.as_slice()).unwrap();
    }
}
