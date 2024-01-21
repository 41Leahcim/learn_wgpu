use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration,
    SurfaceError, TextureFormat, TextureUsages, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

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
}

impl State {
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

        // Create an adapter to interact directly with the GPU
        // You can also use enumerate_adapters to iterate through possible adapters
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                // LowPower is favored when there is no HighPerformance option
                power_preference: wgpu::PowerPreference::default(),

                // The adapter should be compatible with the selected surface
                compatible_surface: Some(&surface),

                // Don't force an adapter, the application won't run without compatible hardware
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
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
            .unwrap();

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
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    match key {
                        winit::event::VirtualKeyCode::B => {
                            self.background_color = Color {
                                r: 0.0,
                                g: 0.0,
                                b: 1.0,
                                a: 1.0,
                            }
                        }
                        winit::event::VirtualKeyCode::G => {
                            self.background_color = Color {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            }
                        }
                        winit::event::VirtualKeyCode::R => {
                            self.background_color = Color {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x / f64::from(self.size.width);
                let y = position.y / f64::from(self.size.height);
                if (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) {
                    self.background_color = Color {
                        r: x,
                        g: y,
                        b: 1.0 - (x + y) / 2.0,
                        a: 1.0,
                    };
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.background_color = Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }
            }
            _ => {}
        }
        false
    }

    pub fn update(&mut self) {}

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

        // Clear the screen
        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render pass"),

            // Where we are going to draw our color
            color_attachments: &[Some(RenderPassColorAttachment {
                // The texture to save the colors to
                view: &view,

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
