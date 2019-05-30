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
use crate::ui::text_area::*;
use crate::ui::widget::*;
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
    let mut manager = UIWidgetManager::new(UISize {
        width: screen_dim[0],
        height: screen_dim[1],
    });

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
    let font2 = Font::new(
        1024,
        1024,
        64,
        16,
        (&include_bytes!("../assets/monserat.ttf")[..]).into(),
    )
    .expect("Cannot load TextArea font");
    let text_area_context = Rc::new(RefCell::new(UITextAreaContext::new(&display, font2)));

    // Create UI elements

    let mut text_style = UITextAreaStyle {
        text_size: 40.0,
        inner_dist: 0.0,
        outer_dist: 1.0,
        shadow_dist: 1.1,
        text_color: Color::new(1.0, 0.3, 0.4),
        shadow_color: Color::new(0.8, 0.4, 0.8),
    };

    let text_area = manager.create(UITextArea::new(
        text_area_context.clone(),
        "Welcome to the SDF font demo!\n\nYou can use left menu to adjust font and texture settings. Move text and zoom it with touchpad and type anything which you want :)\n\nPrzemysław Lenart",
       text_style
    ));

    let label_style = UILabelStyle {
        size: 20.0,
        align: UILabelAlignment::Center,
        color: [1.0, 1.0, 1.0, 1.0],
        shadow_color: [0.0, 0.0, 0.0, 1.0],
    };
    let drawer_block = manager.create(UIBlock::new(
        block_context.clone(),
        UIBlockStyle {
            alpha: 0.99,
            radius: 15.0,
            sharpness: 1.0,
            left_offset: 0.0,
            left_color: [0.015, 0.015, 0.015],
            right_offset: 0.0,
            right_color: [0.015, 0.015, 0.015],
            inner_shadow: 30.0,
            shade_color: [0.005, 0.005, 0.005],
        },
    ));

    let red_label = manager.create(UILabel::new(
        label_context.clone(),
        "red",
        UILabelStyle {
            color: [0.988, 0.576, 0.576, 1.0],
            ..label_style
        },
    ));
    let red_slider = manager.create(UISlider::new(
        &slider_context,
        0.0,
        1.0,
        1.0 / 256.0,
        text_style.text_color.r,
    ));

    let green_label = manager.create(UILabel::new(
        label_context.clone(),
        "green",
        UILabelStyle {
            color: [0.735, 0.941, 0.724, 1.0],
            ..label_style
        },
    ));
    let green_slider = manager.create(UISlider::new(
        &slider_context,
        0.0,
        1.0,
        1.0 / 256.0,
        text_style.text_color.g,
    ));

    let blue_label = manager.create(UILabel::new(
        label_context.clone(),
        "blue",
        UILabelStyle {
            color: [0.716, 0.708, 0.933, 1.0],
            ..label_style
        },
    ));

    let blue_slider = manager.create(UISlider::new(
        &slider_context,
        0.0,
        1.0,
        1.0 / 256.0,
        text_style.text_color.b,
    ));

    let inner_label = manager.create(UILabel::new(
        label_context.clone(),
        "inner distance",
        label_style,
    ));
    let outer_label = manager.create(UILabel::new(
        label_context.clone(),
        "outer distance",
        label_style,
    ));

    // Create screen layout
    let main_layout = manager.create(UIMainLayout {
        padding: 20.0,
        min_width: 150.0,
        max_width: 300.0,
        ratio: 0.3,
    });

    let drawer_layout = manager.create(UIRelativeLayout {
        size: UISize {
            width: 1.0,
            height: 1.0,
        },
        pos: UIPoint {
            left: 0.00,
            top: 0.00,
        },
    });

    let vbox_layout = manager.create(UIVBoxLayout {
        min_height: 30.0,
        max_height: 50.0,
        padding: 20.0,
    });

    let slider_layout = UISliderLayout { label_offset: 20.0 };
    let red_layout = manager.create(slider_layout);
    let green_layout = manager.create(slider_layout);
    let blue_layout = manager.create(slider_layout);

    // Organize views
    manager.root(main_layout);
    manager.add_child(main_layout, drawer_layout);
    manager.add_child(drawer_layout, drawer_block);
    manager.add_child(main_layout, text_area);

    manager.add_child(drawer_layout, vbox_layout);
    manager.add_child(vbox_layout, red_layout);
    manager.add_child(vbox_layout, green_layout);
    manager.add_child(vbox_layout, blue_layout);

    manager.add_child(red_layout, red_slider);
    manager.add_child(red_layout, red_label);
    manager.add_child(green_layout, green_slider);
    manager.add_child(green_layout, green_label);
    manager.add_child(blue_layout, blue_slider);
    manager.add_child(blue_layout, blue_label);


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
        // manager.update(label, |l| {
        //     l.set_text(&format!("FPS: {:.2}", avg_fps));
        // });

        if let Some(time) = start_frame_time {
            let fps = 1.0 / time.elapsed_seconds();
            fps_array[fps_index] = fps;
            fps_index = (fps_index + 1) % fps_array.len();
        }
        start_frame_time = Some(Instant::now());

        // Update widgets
        manager.update(text_area, |t| {
            t.set_style(text_style);
        });

        // Draw scene
        let mut target = display.draw();
        target.clear_color(0.02, 0.02, 0.02, 1.0);

        // Render UI
        manager.render(&mut target);

        // Vsync
        target.finish().expect("finish failed");

        // Send fonts to renderer thread
        {
            for batch in label_context.borrow_mut().get_texture_render_batches() {
                renderer_command_sender
                    .send(RendererCommand::RenderShapes("label_context".into(), batch))
                    .expect("Cannot send render shapes to the renderer");
            }
            for batch in text_area_context.borrow_mut().get_texture_render_batches() {
                renderer_command_sender
                    .send(RendererCommand::RenderShapes(
                        "text_area_context".into(),
                        batch,
                    ))
                    .expect("Cannot send render shapes to the renderer");
            }
        }

        // Handle window events
        manager.set_mouse_wheel_delta(None);
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(glutin::VirtualKeyCode::Escape) = input.virtual_keycode {
                        exit = true;
                    }
                }
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    let height = manager.get_screen().height;
                    manager.set_mouse_pos(UIPoint {
                        left: position.x as f32,
                        top: height - position.y as f32,
                    });
                }
                glutin::WindowEvent::MouseWheel { delta, .. } => {
                    let value = match delta {
                        glutin::MouseScrollDelta::LineDelta(_, y) => y,
                        glutin::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                    };
                    manager.set_mouse_wheel_delta(Some(value));
                }
                glutin::WindowEvent::MouseInput {
                    button: b,
                    state: button_state,
                    ..
                } => {
                    manager.set_left_mouse_button_pressed(
                        b == glutin::MouseButton::Left
                            && button_state == glutin::ElementState::Pressed,
                    );
                    manager.set_right_mouse_button_pressed(
                        b == glutin::MouseButton::Right
                            && button_state == glutin::ElementState::Pressed,
                    );
                }
                glutin::WindowEvent::CloseRequested => exit = true,
                glutin::WindowEvent::Resized(position) => {
                    manager.set_screen(UISize {
                        width: position.width as f32,
                        height: position.height as f32,
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
                        RendererResult::ShapesRendered(name, batch) => {
                            let texture = batch.texture.lock().unwrap();
                            let texture_upload_time = Instant::now();

                            if name == "label_context" {
                                label_context
                                    .borrow_mut()
                                    .update_texture_cache(batch.texture_id, &texture)
                                    .expect("Coudn't upload texture to label context");
                            }

                            if name == "text_area_context" {
                                text_area_context
                                    .borrow_mut()
                                    .update_texture_cache(batch.texture_id, &texture)
                                    .expect("Couldn't upload texture to text area context");
                            }

                            println!("Texture uploaded in {:?}.", texture_upload_time.elapsed());
                        }
                    }
                }
            }
            _ => (),
        });

        // Handle user events
        manager.poll_events(red_slider, |e| {
            let value = match e {
                UISliderEvent::ValueChanged(v) => v,
                UISliderEvent::ValueFinished(v) => v,
            };
            text_style = UITextAreaStyle {
                text_color: Color::new(*value, text_style.text_color.g, text_style.text_color.b),
                ..text_style
            };
        });

        manager.poll_events(green_slider, |e| {
            let value = match e {
                UISliderEvent::ValueChanged(v) => v,
                UISliderEvent::ValueFinished(v) => v,
            };
            text_style = UITextAreaStyle {
                text_color: Color::new(text_style.text_color.r, *value, text_style.text_color.b),
                ..text_style
            };
        });

        manager.poll_events(blue_slider, |e| {
            let value = match e {
                UISliderEvent::ValueChanged(v) => v,
                UISliderEvent::ValueFinished(v) => v,
            };
            text_style = UITextAreaStyle {
                text_color: Color::new(text_style.text_color.r, text_style.text_color.g, *value),
                ..text_style
            };
        });

        // manager.poll_events(button, |e| match e {
        //     UIButtonEvent::Toggled(toggled) => println!("Button toggled: {}", toggled),
        // });
        // manager.poll_events(slider, |e| match e {
        //     UISliderEvent::ValueChanged(v) => println!("Value changed: {}", v),
        //     UISliderEvent::ValueFinished(v) => println!("Value finished: {}", v),
        // });
    }

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
