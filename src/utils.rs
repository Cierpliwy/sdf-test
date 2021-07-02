use glium::glutin::{self, event_loop::ControlFlow};
use std::time::{Duration, Instant};

pub trait ElapsedSeconds {
    fn elapsed_seconds(&self) -> f64;
}

impl ElapsedSeconds for Instant {
    fn elapsed_seconds(&self) -> f64 {
        let duration = self.elapsed();
        duration.as_secs() as f64 + f64::from(duration.subsec_nanos()) / 1_000_000_000f64
    }
}

pub enum Action {
    Stop,
    Continue,
}

pub fn start_loop<F>(event_loop: glutin::event_loop::EventLoop<()>, mut callback: F) -> !
where
    F: 'static + FnMut(&Vec<glutin::event::Event<'_, ()>>) -> Action,
{
    let mut events_buffer = Vec::new();
    let mut next_frame_time = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        let run_callback = match event.to_static() {
            Some(glutin::event::Event::NewEvents(cause)) => {
                matches!(
                    cause,
                    glutin::event::StartCause::ResumeTimeReached { .. }
                        | glutin::event::StartCause::Init
                )
            }
            Some(event) => {
                events_buffer.push(event);
                false
            }
            None => {
                // Ignore this event.
                false
            }
        };

        let action = if run_callback {
            let action = callback(&events_buffer);
            next_frame_time = Instant::now() + Duration::from_nanos(16666667);
            events_buffer.clear();
            action
        } else {
            Action::Continue
        };

        match action {
            Action::Continue => {
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            }
            Action::Stop => *control_flow = ControlFlow::Exit,
        }
    })
}
