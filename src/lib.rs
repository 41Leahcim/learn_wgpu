#![warn(clippy::pedantic, clippy::nursery)]

use wgpu::SurfaceError;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::state::State;

pub mod state;

/// # Panics
/// panics if the window couldn't be created
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[allow(clippy::future_not_send)]
pub async fn run() {
    // Required for wgpu error messages to be printed
    cfg_if::cfg_if! {
        if #[cfg(target_arch="wasm32")]{
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        }else{
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(window).await;

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set the size manually when on the web.
        use winit::dpi::PhysicalSize;
        use winit::platform::web::WindowExtWebSys;

        state.window().set_inner_size(PhysicalSize::new(450, 400));

        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(window_id) if window_id == state.window().id() => {
            state.update();
            match state.render() {
                Ok(()) => {}
                // Reconfigue the surface if lost
                Err(SurfaceError::Lost) => state.resize(state.size()),

                // The system is out of memory, we should probably quit
                Err(SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                // All other erros (Outdated, TimeOut) should be resolved by the next frame
                Err(e) => eprintln!("{e:?}"),
            }
        }
        // RedrawRequested will onluy trigger once unless we manually request it.
        Event::MainEventsCleared => state.window().request_redraw(),

        // If the window changed
        Event::WindowEvent { window_id, event } if window_id == state.window().id() => {
            // And none of the applications inputs were used
            if !state.input(&event) {
                {
                    // Check what event happened
                    match event {
                        // If the window resized, update the states size
                        WindowEvent::Resized(physical_size) => state.resize(physical_size),

                        // If the scale factor changed, update the states size
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(*new_inner_size);
                        }

                        // If close was requested or escape was pressed, close the application
                        winit::event::WindowEvent::CloseRequested
                        | winit::event::WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    });
}
