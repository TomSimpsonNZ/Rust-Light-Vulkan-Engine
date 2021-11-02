use winit::{
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
    dpi::LogicalSize,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const NAME: &str = "Hello Vulkan!";


pub struct VulkanApp {
    _window: Window,
}

impl VulkanApp {
    pub fn new() -> (Self, EventLoop<()>) {
        // Create the event loop and application window
        let (event_loop, window) = Self::new_window(WIDTH, HEIGHT, NAME);

        (Self {
            _window: window,
        }, event_loop)
    }

    fn new_window(w: u32, h: u32, name: &str) -> (EventLoop<()>, Window) {
        log::debug!("Starting event loop");
        let event_loop = EventLoop::new();

        log::debug!("Creating window");
        let winit_window = WindowBuilder::new()
            .with_title(name)
            .with_inner_size(LogicalSize::new(w, h))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        (event_loop, winit_window)
    }

}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        log::debug!("Dropping application")
    }
}