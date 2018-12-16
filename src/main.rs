pub mod renderer_thread;
pub mod text;
pub mod ui;

use crate::renderer_thread::{
    renderer_entry_point, RendererCommand, RendererContext, RendererResult,
};
use crate::ui::block::*;
use crate::ui::label::*;
use crate::ui::*;

use glium::{glutin, Surface};
use sdf::font::Font;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

fn main() {
    // Create GL objects
    let screen_dim = [685.0f32, 480.0];
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions((screen_dim[0] as f64, screen_dim[1] as f64).into())
        .with_title("Multi-channel signed distance font demo - by Cierpliwy");
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // Create UI contexts
    let ui_context = UIContext::new(&display);
    let block_context = Rc::new(UIBlockContext::new(ui_context.clone()));
    let font = Font::new(
        1024,
        1024,
        32,
        4,
        (&include_bytes!("../assets/monserat.ttf")[..]).into(),
    )
    .expect("Cannot load UI font");
    let label_context = Rc::new(RefCell::new(UILabelContext::new(ui_context, font)));

    // Create UI elements
    let mut label = UILabel::new(
        label_context.clone(),
        "Trademark(R)",
        20.0,
        UILabelAlignment::Left,
    );

    let background_style = UIBlockStyle {
        alpha: 0.5,
        radius: 0.0,
        sharpness: 0.0,
        left_offset: -1.0,
        left_color: [0.0, 0.0, 0.0],
        right_offset: 0.0,
        right_color: [0.0, 1.0, 0.0],
        inner_shadow: -1.0,
        shade_color: [0.0, 0.0, 0.0],
    };
    let bb_style = UIBlockStyle {
        right_color: [1.0, 0.0, 0.0],
        ..background_style
    };
    let label_background = UIBlock::new(block_context.clone());
    let label_bounding_box = UIBlock::new(block_context.clone());

    // Create screen layout
    let screen: UIScreen = Rc::new(Cell::new(UIScreenInfo::new(screen_dim, 1.0)));
    let label_layout = UIRelativeLayout::new(&screen, [0.2, 0.4], [0.3, 0.5]);
    let bb = label.get_bounding_box();
    let label_bb_layout = UIAbsoluteLayout::new(
        &label_layout,
        [bb.width(), bb.height()],
        [bb.min.x, bb.min.y],
    );

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
        label.set_text(&format!("FPS: {:.2}", avg_fps));
        if let Some(time) = start_frame_time {
            let duration = time.elapsed();
            let fps = 1.0
                / (duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000f64);
            fps_array[fps_index] = fps;
            fps_index = (fps_index + 1) % fps_array.len();
        }
        start_frame_time = Some(Instant::now());

        // Draw scene
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        // Render UI
        label_background.render(&mut target, &background_style, &label_layout);
        label_bounding_box.render(&mut target, &bb_style, &label_bb_layout);
        label.render(&mut target, &label_layout);

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
                glutin::WindowEvent::CloseRequested => exit = true,
                glutin::WindowEvent::Resized(position) => {
                    let res = [position.width as f32, position.height as f32];
                    screen.set(UIScreenInfo::new(
                        [res[0], res[1]],
                        screen.get().get_ratio(),
                    ))
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
    }

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
