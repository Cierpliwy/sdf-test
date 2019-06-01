pub mod renderer_thread;
pub mod text;
pub mod ui;
pub mod utils;

use crate::renderer_thread::*;
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
        .with_title("Multi-channel Signed Distance Field Font Demo");
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    // Create state
    let mut manager = UIWidgetManager::new(UISize {
        width: screen_dim[0],
        height: screen_dim[1],
    });

    // Create fonts
    let font = Font::new(
        1024,
        1024,
        32,
        8,
        (&include_bytes!("../assets/monserat.ttf")[..]).into(),
    )
    .expect("Cannot load UI font");
    let text_area_font = Font::new(
        1024,
        1024,
        64,
        16,
        (&include_bytes!("../assets/monserat.ttf")[..]).into(),
    )
    .expect("Cannot load TextArea font");

    // Create UI contexts
    let block_context = Rc::new(UIBlockContext::new(&display));
    let label_context = Rc::new(RefCell::new(UILabelContext::new(&display, font)));
    let button_context = Rc::new(UIButtonContext::new(
        block_context.clone(),
        label_context.clone(),
    ));
    let slider_context = Rc::new(UISliderContext::new(
        block_context.clone(),
        label_context.clone(),
    ));
    let text_area_context = Rc::new(RefCell::new(UITextAreaContext::new(
        &display,
        text_area_font,
    )));

    // Prepare UI elements styles and common functions.
    let label_style = UILabelStyle {
        size: 16.0,
        align: UILabelAlignment::Left,
        color: [1.0, 1.0, 1.0, 1.0],
        shadow_color: [0.0, 0.0, 0.0, 1.0],
        opacity: 1.0
    };

    let titile_label_style = UILabelStyle {
        size: 25.0,
        align: UILabelAlignment::Center,
        color: [1.0, 1.0, 1.0, 1.0],
        shadow_color: [0.0, 0.0, 0.0, 1.0],
        opacity: 1.0
    };

    let mut text_style = UITextAreaStyle {
        text_size: 40.0,
        inner_dist: 0.0,
        outer_dist: 0.55,
        shadow_dist: 1.1,
        sharpness: 0.4,
        text_color: Color::new(0.75, 0.82, 1.0),
        shadow_color: Color::new(0.8, 0.4, 0.8),
    };

    let text_area = manager.create(UITextArea::new(
        text_area_context.clone(),
        r#"Welcome to the MCSDF font demo!
        
        • Use the left panel to adjust font rendering settings.
        • Use the right panel to regenerate MCSDF texture.
        • Type anything you want."#,
        text_style,
    ));

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

    macro_rules! create_title_label {
        ($text:expr) => {
            manager.create(UILabel::new(
                label_context.clone(),
                $text,
                titile_label_style,
            ))
        };
    };

    macro_rules! create_label {
        ($text:expr) => {
            manager.create(UILabel::new(label_context.clone(), $text, label_style))
        };
        ($text:expr, $r:expr, $g:expr, $b:expr) => {
            manager.create(UILabel::new(
                label_context.clone(),
                $text,
                UILabelStyle {
                    color: [$r, $g, $b, 1.0],
                    ..label_style
                },
            ))
        };
    };

    macro_rules! create_slider {
        ($default:expr) => {
            manager.create(UISlider::new(
                &slider_context,
                1.0 / 256.0,
                1.0,
                1.0 / 256.0,
                $default,
            ))
        };
    };

    // Create UI elements
    let outline_label = create_title_label!("Outline");

    let red_label = create_label!("red", 0.988, 0.576, 0.576);
    let red_slider = create_slider!(text_style.text_color.r);

    let green_label = create_label!("green", 0.735, 0.941, 0.724);
    let green_slider = create_slider!(text_style.text_color.g);

    let blue_label = create_label!("blue", 0.716, 0.708, 0.933);
    let blue_slider = create_slider!(text_style.text_color.b);

    let inner_dist_label = create_label!("inner distance");
    let inner_dist_slider = create_slider!(text_style.inner_dist);

    let outer_dist_label = create_label!("outer distance");
    let outer_dist_slider = create_slider!(text_style.outer_dist);

    let sharpness_label = create_label!("sharpness");
    let sharpness_slider = create_slider!(text_style.sharpness);

    let shadow_label = create_title_label!("Shadow");

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
        hpadding: 20.0,
        vpadding: 8.0,
    });

    let slider_layout = UISliderLayout { label_offset: 20.0 };
    let red_layout = manager.create(slider_layout);
    let green_layout = manager.create(slider_layout);
    let blue_layout = manager.create(slider_layout);
    let inner_dist_layout = manager.create(slider_layout);
    let outer_dist_layout = manager.create(slider_layout);
    let sharpness_layout = manager.create(slider_layout);

    // Organize views
    manager.root(main_layout);
    manager.add_child(main_layout, drawer_layout);
    manager.add_child(drawer_layout, drawer_block);
    manager.add_child(main_layout, text_area);

    manager.add_child(drawer_layout, vbox_layout);

    manager.add_child(vbox_layout, outline_label);
    manager.add_child(vbox_layout, red_layout);
    manager.add_child(vbox_layout, green_layout);
    manager.add_child(vbox_layout, blue_layout);
    manager.add_child(vbox_layout, inner_dist_layout);
    manager.add_child(vbox_layout, outer_dist_layout);
    manager.add_child(vbox_layout, sharpness_layout);

    manager.add_child(vbox_layout, shadow_label);

    manager.add_child(red_layout, red_slider);
    manager.add_child(red_layout, red_label);
    manager.add_child(green_layout, green_slider);
    manager.add_child(green_layout, green_label);
    manager.add_child(blue_layout, blue_slider);
    manager.add_child(blue_layout, blue_label);
    manager.add_child(inner_dist_layout, inner_dist_slider);
    manager.add_child(inner_dist_layout, inner_dist_label);
    manager.add_child(outer_dist_layout, outer_dist_slider);
    manager.add_child(outer_dist_layout, outer_dist_label);
    manager.add_child(sharpness_layout, sharpness_slider);
    manager.add_child(sharpness_layout, sharpness_label);

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

    while !exit {
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

        // Handle font style
        macro_rules! handle_font_style_slider {
            ($slider:expr, $name:ident, $map:expr) => {
                manager.poll_events($slider, |e| {
                    let value = match e {
                        UISliderEvent::ValueChanged(v) => v,
                        UISliderEvent::ValueFinished(v) => v,
                    };
                    text_style = UITextAreaStyle {
                        $name: $map(*value),
                        ..text_style
                    };
                });
            };
        }

        handle_font_style_slider!(red_slider, text_color, |v: f32| Color::new(
            v,
            text_style.text_color.g,
            text_style.text_color.b
        ));
        handle_font_style_slider!(green_slider, text_color, |v: f32| Color::new(
            text_style.text_color.r,
            v,
            text_style.text_color.b
        ));
        handle_font_style_slider!(blue_slider, text_color, |v: f32| Color::new(
            text_style.text_color.r,
            text_style.text_color.g,
            v
        ));
        handle_font_style_slider!(inner_dist_slider, inner_dist, |v: f32| v);
        handle_font_style_slider!(outer_dist_slider, outer_dist, |v: f32| v);
        handle_font_style_slider!(sharpness_slider, sharpness, |v: f32| v);
    }

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
