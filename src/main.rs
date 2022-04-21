mod first_app;

use first_app::*;

fn main() {
    // Begin the rust logging functionality
    env_logger::init();

    // Create the application and events loop
    let (vulkan_app, event_loop) = VulkanApp::new();

    log::debug!("Running Application");

    vulkan_app.run(event_loop);
}
