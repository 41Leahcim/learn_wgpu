use wgpu::{
    Adapter, Backends, BlendState, Color, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, Device, DeviceDescriptor, Face, Features, FragmentState, FrontFace,
    Instance, InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    RequestAdapterOptions, ShaderModuleDescriptor, ShaderSource, StoreOp, Surface,
    SurfaceConfiguration, SurfaceError, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, VertexState,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    window::Window,
};

pub struct State {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,

    /// The window must be declared after the surface so
    /// it gets dropped after after it as the surface contains
    /// unsafe references to the window's resources.
    window: Window,

    background_color: Color,
    render_pipeline: RenderPipeline,
    second_pipeline: RenderPipeline,
}

impl State {
    async fn create_adapter(instance: &Instance, surface: &Surface) -> Adapter {
        // Create an adapter to interact directly with the GPU
        // You can also use enumerate_adapters to iterate through possible adapters
        instance
            .request_adapter(&RequestAdapterOptions {
                // LowPower is favored when there is no HighPerformance option
                power_preference: wgpu::PowerPreference::default(),

                // The adapter should be compatible with the selected surface
                compatible_surface: Some(surface),

                // Don't force an adapter, the application won't run without compatible hardware
                force_fallback_adapter: false,
            })
            .await
            .unwrap()
    }

    async fn request_device(adapter: &Adapter) -> (Device, Queue) {
        adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,

                    // Extra features
                    features: Features::empty(),

                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        Limits::downlevel_webgl2_defaults()
                    } else {
                        Limits::default()
                    },
                },
                None,
            )
            .await
            .unwrap()
    }

    fn create_pipeline(
        device: &Device,
        config: &SurfaceConfiguration,
        fragment_entry_point: &str,
    ) -> RenderPipeline {
        // Read the shader.
        // Can also be done with:
        //let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create a layout for the pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader,
                // The function in the shader that should be the entry point.
                // In this case for the vertex shader.
                entry_point: "vs_main",

                // The types of vertices to pass to the vertex shader
                buffers: &[],
            },

            // The fragment state is optional, but here it's needed to store color data
            // to the surface
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: fragment_entry_point,

                // The color outputs to set up
                targets: &[Some(ColorTargetState {
                    // Using the surface's format makes copying to it easy
                    format: config.format,

                    // Blending should replace the old data with the new data
                    blend: Some(BlendState::REPLACE),

                    // Write to all colors
                    write_mask: ColorWrites::ALL,
                })],
            }),

            // How to interpret vertices when converting them into triangles
            primitive: PrimitiveState {
                // `PrimitiveTopology::TriangleList` means that every 3 vertices will correspond
                // to 1 triangle.
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,

                // Determine whether a triangle is facing forward.
                // With `FrontFace::Ccw`, a triangle is facing forward if the vertices are in
                // counter-clockwise direction.
                // Other triangles are culled as specified by `Face::Back`.
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),

                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: PolygonMode::Fill,

                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,

                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },

            // The depth.stencil buffer isn't used
            depth_stencil: None,

            multisample: MultisampleState {
                // The number of samples the pipeline uses
                count: 1,

                // Which samples should be active (all of them)
                mask: !0,

                // No anti aliasing is used
                alpha_to_coverage_enabled: false,
            },

            // Number of array layers the render attachments can have
            multiview: None,
        })
    }

    /// Creating some of the wgpu types requires async code
    ///
    /// # Panics
    /// Panics if no surface, adapter, device, or texture format could be created
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        // The surface is the part of the window we draw to.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        // Create an adapter
        let adapter = Self::create_adapter(&instance, &surface).await;

        let (device, queue) = Self::request_device(&adapter).await;

        // Retrieve the capabilities of the surface
        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(TextureFormat::is_srgb)
            .unwrap_or(surface_caps.formats[0]);

        // Create a configuration for the surface.
        // This will define how the surface creates its underlying surface textures.
        let config = SurfaceConfiguration {
            // How surface textures will be used, in this case to write to the screen.
            usage: TextureUsages::RENDER_ATTACHMENT,

            // How surface textures will be stored on the GPU.
            format: surface_format,

            // The dimensions of the surface texture in pixels, should always be larger than 0.
            width: size.width,
            height: size.height,

            // How to sync the surface with the display, we select the first option for simplicity.
            // PresentMode::Fifo will cap the display rate at the display's framerate (like VSync).
            // PresentMode::Fifo is supported on all platforms.
            // PresentMode::AutoVsync and PresentMode::AutoNoVsync have fallback support to work
            // on all platforms.
            // PresentMode can also be selected at run-time with surface_caps.present_modes.
            present_mode: surface_caps.present_modes[0],

            // How the alpha modes will be handled during compositing.
            alpha_mode: surface_caps.alpha_modes[0],

            // List of TextureFormats that can be used to create TextureViews
            view_formats: vec![],
        };

        // Apply the configurations
        surface.configure(&device, &config);

        let render_pipeline = Self::create_pipeline(&device, &config, "fs_main");
        let second_pipeline = Self::create_pipeline(&device, &config, "fs_main2");

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            background_color: Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            render_pipeline,
            second_pipeline,
        }
    }

    pub const fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // Store the new size
            self.size = new_size;
            self.config.width = new_size.width;

            // Reconfigure the surface for the new size
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            // Keyboard input received
            WindowEvent::KeyboardInput { input, .. } => {
                // If the user pressed a key
                if let Some(key) = input
                    .virtual_keycode
                    .filter(|_| input.state == ElementState::Pressed)
                {
                    // Check what key the user pressed
                    match key {
                        // If it is B, make the background blue
                        winit::event::VirtualKeyCode::B => {
                            self.background_color = Color {
                                r: 0.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }
                        }

                        // If it is G, make the background green
                        winit::event::VirtualKeyCode::G => {
                            self.background_color = Color {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            }
                        }

                        // If it is R, make the background red
                        winit::event::VirtualKeyCode::R => {
                            self.background_color = Color {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }
                        }

                        // If it is space, switch the render pipelines
                        winit::event::VirtualKeyCode::Space => {
                            core::mem::swap(&mut self.render_pipeline, &mut self.second_pipeline);
                        }
                        _ => return false,
                    }
                }
            }

            // If the cursor moved
            WindowEvent::CursorMoved { position, .. } => {
                // Calculate the normalized x and y positions
                let x = position.x / f64::from(self.size.width);
                let y = position.y / f64::from(self.size.height);

                // If they are between 0 and 1, calculate and set the new background colors
                if (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) {
                    self.background_color = Color {
                        r: x,
                        g: y,
                        b: 1.0 - (x + y) / 2.0,
                        a: 1.0,
                    };
                }
            }

            // If the cursor left the screen, make the background black
            WindowEvent::CursorLeft { .. } => {
                self.background_color = Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }
            }
            _ => return false,
        }
        true
    }

    pub fn update(&mut self) {}

    fn render_with_pipeline(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        // Clear the screen
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render pass"),

            // Where we are going to draw our color
            color_attachments: &[Some(RenderPassColorAttachment {
                // The texture to save the colors to
                view,

                // The texture that will receive the resolved output.
                // This will be the same as view unless multisampling is enabled.
                resolve_target: None,

                // What to do with the colors on the screen
                ops: Operations {
                    // How to handle colors from the previous frame
                    load: LoadOp::Clear(self.background_color),

                    // Whether we want to store the renderedd results to the texture
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Add the render pipeline to the render pass
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.draw(0..3, 0..1);
    }

    /// # Errors
    /// Returns an error if no render surface could be retrieved
    pub fn render(&mut self) -> Result<(), SurfaceError> {
        // Wait for the surface to provide a surface texture to render to
        let output = self.surface.get_current_texture()?;

        // Create a texture view with default settings.
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        // Create a command encoder to create the actual commands to send to the gpu.
        // The encoder builds a command buffer that we can then send to the GPU.
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.render_with_pipeline(&mut encoder, &view);

        // Submit will accept anything that implements IntoIter.
        // Send the render pass(es) to the GPU
        self.queue.submit(std::iter::once(encoder.finish()));

        // Display the image
        output.present();

        Ok(())
    }

    pub const fn size(&self) -> PhysicalSize<u32> {
        self.size
    }
}
