pub mod renderer_thread;
pub mod text;
pub mod ui;
pub mod utils;

use crate::renderer_thread::{
    renderer_entry_point, RendererCommand, RendererContext, RendererResult,
};
use crate::ui::block::*;
use crate::ui::button::*;
use crate::ui::label::*;
use crate::ui::layout::*;
use crate::ui::slider::*;
use crate::ui::*;
use crate::utils::*;

use glium::{glutin, Surface};
use sdf::font::Font;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

fn main() {
    // Create GL objects
    let screen_dim = [640.0, 480.0];
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions((f64::from(screen_dim[0]), f64::from(screen_dim[1])).into())
        .with_title("Multi-channel signed distance font demo - by Cierpliwy");
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // Create state
    let mut state = UIState::new(screen_dim[0], screen_dim[1]);

    // Create UI contexts
    let block_context = Rc::new(UIBlockContext::new(&display));
    let font = Font::new(
        1024,
        1024,
        32,
        8,
        (&include_bytes!("../assets/monserat.ttf")[..]).into(),
    )
    .expect("Cannot load UI font");
    let label_context = Rc::new(RefCell::new(UILabelContext::new(&display, font)));
    let button_context = Rc::new(UIButtonContext::new(
        block_context.clone(),
        label_context.clone(),
    ));
    let slider_context = Rc::new(UISliderContext::new(
        block_context.clone(),
        label_context.clone(),
    ));

    // Create UI elements
    let (label, slider, button) = state.update_widget(|wm| {
        let label = wm.create(UILabel::new(
            label_context.clone(),
            "Trademark(R)",
            UILabelStyle {
                size: 20.0,
                align: UILabelAlignment::Left,
                color: [1.0, 1.0, 1.0, 1.0],
                shadow_color: [0.0, 0.0, 0.0, 1.0],
            },
        ));

        let slider = wm.create(UISlider::new(&slider_context, 100.0, 1000.0, 5.0, 500.0));
        let button = wm.create(UIButton::new(&button_context, "Show textures"));

        (label, slider, button)
    });

    // Create screen layout
    let (label_layout, slider_layout, button_layout) = state.update_layout(|lm| {
        let label_layout = lm.root(UIRelativeLayout {
            size: [0.2, 0.4],
            pos: [0.3, 0.5],
        });

        let slider_layout = lm.root(UIRelativeLayout {
            size: [0.5, 0.5],
            pos: [0.0, 0.0],
        });

        let button_layout = lm.root(UIRelativeLayout {
            size: [0.5, 0.5],
            pos: [0.5, 0.5],
        });

        (label_layout, slider_layout, button_layout)
    });

    // Pin views
    state.pin_widget(label.into(), label_layout.into());
    state.pin_widget(slider.into(), slider_layout.into());
    state.pin_widget(button.into(), button_layout.into());

    // Handle font renderer command queues.
    let (renderer_command_sender, renderer_command_receiver) = channel();
    let (renderer_result_sender, renderer_result_receiver) = channel();
    let renderer_context = RendererContext {
        receiver: renderer_command_receiver,
        sender: renderer_result_sender,
        proxy: events_loop.create_proxy(),
    };
    let renderer_thread = thread::spawn(|| {
        renderer_entry_point(renderer_context).expect("Got an error on renderer thread");
    });

    let mut exit = false;
    let mut scale = false;
    let mut fps_array = [0.0; 64];
    let mut fps_index = 0;
    let mut start_frame_time: Option<Instant> = None;

    while !exit {
        // FPS counting
        let avg_fps: f64 = fps_array.iter().sum::<f64>() / fps_array.len() as f64;
        state.update_widget(|wm| {
            wm.update(label, |l| {
                l.set_text(&format!("FPS: {:.2}", avg_fps));
            })
        });
        if let Some(time) = start_frame_time {
            let fps = 1.0 / time.elapsed_seconds();
            fps_array[fps_index] = fps;
            fps_index = (fps_index + 1) % fps_array.len();
        }
        start_frame_time = Some(Instant::now());

        // Draw scene
        let mut target = display.draw();
        target.clear_color(0.02, 0.02, 0.02, 1.0);

        // Render UI
        state.render(&mut target);

        // Vsync
        target.finish().expect("finish failed");

        // Send fonts to renderer thread
        {
            for batch in label_context.borrow_mut().get_texture_render_batches() {
                renderer_command_sender
                    .send(RendererCommand::RenderShapes(batch))
                    .expect("Cannot send render shapes to the renderer");
            }
        }

        // Handle window events
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(glutin::VirtualKeyCode::Escape) = input.virtual_keycode {
                        exit = true;
                    }
                }
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    let height = state.layout(|lm| lm.get_screen().height);
                    state.set_mouse_pos([position.x as f32, height - position.y as f32]);
                }
                glutin::WindowEvent::MouseInput {
                    button: b,
                    state: button_state,
                    ..
                } => {
                    state.set_left_mouse_button_pressed(
                        b == glutin::MouseButton::Left
                            && button_state == glutin::ElementState::Pressed,
                    );
                    state.set_right_mouse_button_pressed(
                        b == glutin::MouseButton::Right
                            && button_state == glutin::ElementState::Pressed,
                    );
                }
                glutin::WindowEvent::CloseRequested => exit = true,
                glutin::WindowEvent::Resized(position) => {
                    state.update_layout(|lm| {
                        lm.set_screen(UIScreen {
                            width: position.width as f32,
                            height: position.height as f32,
                        });
                    });
                }
                glutin::WindowEvent::ReceivedCharacter(c) => {
                    if c == 's' {
                        scale = !scale;
                    }
                }
                _ => (),
            },
            glutin::Event::Awakened => {
                let result = renderer_result_receiver.try_recv();
                if let Ok(result) = result {
                    match result {
                        RendererResult::ShapesRendered(batch) => {
                            let texture = batch.texture.lock().unwrap();
                            let texture_upload_time = Instant::now();

                            label_context
                                .borrow_mut()
                                .update_texture_cache(batch.texture_id, &texture)
                                .expect("Coudn't upload texture to label context");

                            println!("Texture uploaded in {:?}.", texture_upload_time.elapsed());
                        }
                    }
                }
            }
            _ => (),
        });

        // Handle user events
        state.update_widget(|wm| {
            wm.poll_events(button, |e| match e {
                UIButtonEvent::Toggled(toggled) => println!("Button toggled: {}", toggled),
            });
            wm.poll_events(slider, |e| match e {
                UISliderEvent::ValueChanged(v) => println!("Value changed: {}", v),
                UISliderEvent::ValueFinished(v) => println!("Value finished: {}", v),
            })
        });
    }

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
