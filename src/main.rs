mod first_app;
mod fps_counter;

use std::time::Instant;

use first_app::*;

use fps_counter::FPSCounter;

use winit::{
    dpi::PhysicalSize,
    event::{ElementState, Event, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    // Begin the rust logging functionality
    env_logger::init();

    // Create the application and events loop
    let (mut vulkan_app, event_loop) = VulkanApp::new();

    log::debug!("Running Application");

    let mut current_time = Instant::now();

    let mut keys_pressed: Vec<VirtualKeyCode> = Vec::new();

    let mut fps_counter = FPSCounter::new(100);

    // Begin the events loop
    event_loop.run(move |event, _, control_flow| {
        // Set the behavior to poll the window for user events
        *control_flow = ControlFlow::Poll;

        // Create a reference to the vulkan app here so that it is dropped
        // properly and can be used in the loop
        let app = &mut vulkan_app;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log::debug!("Closing window");
                *control_flow = ControlFlow::Exit
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(PhysicalSize { width, height }),
                ..
            } => {
                log::debug!("Resizing window");
                log::info!("New window size: {}x{}", width, height);
                app.resize();
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                match input.virtual_keycode {
                    Some(VirtualKeyCode::Escape) => {
                        log::debug!("Closing window");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    Some(input_key) => {
                        match input.state {
                            ElementState::Pressed => {
                                if !keys_pressed.contains(&input_key) {
                                    keys_pressed.push(input_key);
                                }
                            }
                            ElementState::Released => {
                                let index = keys_pressed
                                    .iter()
                                    .position(|key| *key == input_key)
                                    .unwrap();
                                keys_pressed.remove(index);
                            }
                        };
                    }
                    None => {}
                };
            }
            Event::MainEventsCleared => {
                app.window.request_redraw();
            }
            Event::RedrawRequested(_window_id) => {
                let time_since_last_frame = current_time.elapsed().as_secs_f32();
                current_time = Instant::now();
                app.run(&keys_pressed, time_since_last_frame);
                let window_title = format!(
                    "HELLO VULAKN | fps: {}",
                    fps_counter.tick(time_since_last_frame)
                );
                app.window.set_title(&window_title);
            }
            _ => (),
        };
    });
}
