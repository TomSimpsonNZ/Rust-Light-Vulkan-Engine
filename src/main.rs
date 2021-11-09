mod first_app;

use first_app::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() {
    // Begin the rust logging functionality
    env_logger::init();

    // Create the application and events loop
    let (mut vulkan_app, event_loop) = VulkanApp::new();

    log::debug!("Running Application");

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
            Event::MainEventsCleared => {
                app.draw_frame();
            }
            _ => (),
        }
    });
}
