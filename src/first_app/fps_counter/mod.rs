use std::collections::VecDeque;
use std::time::Instant;

pub struct FPSCounter {
    frame_times: VecDeque<f32>,
    buffer_size: usize,
    update_time: Instant,
    fps: u32,
}

impl FPSCounter {
    // Initialise the FPS Counter
    pub fn new(capacity: usize) -> FPSCounter {
        FPSCounter {
            frame_times: VecDeque::with_capacity(capacity),
            buffer_size: capacity,
            update_time: Instant::now(),
            fps: 0,
        }
    }

    // Increment the tick counter and return most up to date fps count
    pub fn tick(&mut self, frame_time: f32) -> u32 {
        // Add the frame time to the buffer
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > self.buffer_size {
            self.frame_times.pop_front();
        }
        // Check if a second has passed
        if self.update_time.elapsed().as_secs_f32() >= 0.5 {
            // calculate the average of all the frame times in the buffer
            let sum = self.frame_times.iter().fold(0_f32, |acc, t| acc + 1.0 / t);
            self.fps = (sum / self.frame_times.len() as f32).round() as u32;
            self.update_time = Instant::now();
        }
        self.fps
    }
}
