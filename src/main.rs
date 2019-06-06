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

use glium::{glutin, Surface};
use sdf::font::Font;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

fn main() {
    // Create GL objects
    let screen_dim = [1400.0, 900.0];
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions((f64::from(screen_dim[0]), f64::from(screen_dim[1])).into())
        .with_title("Multi-channel Signed Distance Field Font Demo");
    let context = glutin::ContextBuilder::new().with_vsync(true);
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

    let text_area_texture_size = 1024;
    let text_area_font_size = 48;
    let text_area_shadow_size = 4;

    let text_area_font = Font::new(
        text_area_texture_size,
        text_area_texture_size,
        text_area_font_size,
        text_area_shadow_size,
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
        opacity: 1.0,
    };

    let label_right_style = UILabelStyle {
        align: UILabelAlignment::Right,
        ..label_style
    };

    let title_label_style = UILabelStyle {
        size: 25.0,
        align: UILabelAlignment::Center,
        color: [1.0, 1.0, 1.0, 1.0],
        shadow_color: [0.0, 0.0, 0.0, 1.0],
        opacity: 1.0,
    };

    let mut text_style = UITextAreaStyle {
        text_size: 30.0,
        inner_dist: 0.0,
        outer_dist: 0.55,
        shadow_dist: 1.1,
        sharpness: 0.4,
        text_color: Color::new(1.0, 1.0, 1.0),
        shadow_color: Color::new(0.19, 0.36, 1.0),
        shadow_pos: 0.24,
        shadow_size: 0.21,
        shadow_alpha: 0.05,
        texture_visibility: 0.0,
        animation: false,
    };

    let text_area = manager.create(UITextArea::new(
        text_area_context.clone(),
        r#"Welcome to the multi-channel distance fields font tech demo!
        
        • Left panel - use it to modify font rendering settings, which update only uniform values used in text shaders.
        
        • Right panel - use it to modify font texture, which affects the quality of glyphs. Make sure to check out animation as well :)
        
        • Mouse/scroll - use it to move and zoom a text in the center.
        
        • Keyboard - use to type anything you want.
        
        Enjoy!"#,
        text_style,
    ));

    let drawer_block_style = UIBlockStyle {
        alpha: 0.99,
        radius: 15.0,
        sharpness: 1.0,
        left_offset: 0.0,
        left_color: [0.015, 0.015, 0.015],
        right_offset: 0.0,
        right_color: [0.015, 0.015, 0.015],
        inner_shadow: 30.0,
        shade_color: [0.005, 0.005, 0.005],
    };

    let left_drawer_block = manager.create(UIBlock::new(block_context.clone(), drawer_block_style));

    let right_drawer_block =
        manager.create(UIBlock::new(block_context.clone(), drawer_block_style));

    macro_rules! create_styled_label {
        ($text:expr, $style:expr) => {
            manager.create(UILabel::new(label_context.clone(), $text, $style))
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
                0.0,
                1.0,
                1.0 / 256.0,
                $default,
                2,
            ))
        };
        ($default:expr, $min:expr, $max:expr, $step:expr, $precision:expr) => {
            manager.create(UISlider::new(
                &slider_context,
                $min,
                $max,
                $step,
                $default,
                $precision,
            ))
        };
    };

    // Create UI elements
    let outline_label = create_styled_label!("Outline", title_label_style);

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

    let shadow_label = create_styled_label!("Shadow", title_label_style);

    let shadow_red_label = create_label!("red", 0.988, 0.576, 0.576);
    let shadow_red_slider = create_slider!(text_style.shadow_color.r);

    let shadow_green_label = create_label!("green", 0.735, 0.941, 0.724);
    let shadow_green_slider = create_slider!(text_style.shadow_color.g);

    let shadow_blue_label = create_label!("blue", 0.716, 0.708, 0.933);
    let shadow_blue_slider = create_slider!(text_style.shadow_color.g);

    let shadow_alpha_label = create_label!("opacity");
    let shadow_alpha_slider = create_slider!(text_style.shadow_alpha);

    let shadow_pos_label = create_label!("position");
    let shadow_pos_slider = create_slider!(text_style.shadow_pos);

    let shadow_size_label = create_label!("size");
    let shadow_size_slider = create_slider!(text_style.shadow_size);

    let texture_label = create_styled_label!("Texture", title_label_style);

    let texture_size_label = create_label!("size");
    let texture_size_slider = create_slider!(
        text_area_texture_size as f32,
        1024.0,
        1024.0 * 8.0,
        512.0,
        0
    );

    let texture_font_size_label = create_label!("font size");
    let texture_font_size_slider = create_slider!(text_area_font_size as f32, 16.0, 255.0, 1.0, 0);

    let texture_shadow_size_label = create_label!("shadow size");
    let texture_shadow_size_slider =
        create_slider!(text_area_shadow_size as f32, 1.0, 64.0, 1.0, 0);

    let render_stats_label = create_styled_label!("Render stats", title_label_style);

    let render_glyph_label = create_label!("Avg. glyph render time:");
    let render_glyph_value_label = create_styled_label!("-", label_right_style);

    let render_texture_label = create_label!("Avg. texture copy time:");
    let render_texture_value_label = create_styled_label!("-", label_right_style);

    let other_label = create_styled_label!("Other", title_label_style);

    let texture_visibility_label = create_label!("texture visibility");
    let texture_visibility_slider = create_slider!(text_style.texture_visibility);

    let animation_button = manager.create(UIButton::new(&button_context, "Show animation"));

    // Create screen layout
    let main_layout = manager.create(UIMainLayout {
        padding: 20.0,
        min_width: 300.0,
        max_width: 400.0,
        ratio: 0.3,
    });

    let left_drawer_layout = manager.create(UIRelativeLayout {
        size: UISize {
            width: 1.0,
            height: 1.0,
        },
        pos: UIPoint {
            left: 0.00,
            top: 0.00,
        },
    });

    let right_drawer_layout = manager.create(UIRelativeLayout {
        size: UISize {
            width: 1.0,
            height: 1.0,
        },
        pos: UIPoint {
            left: 0.00,
            top: 0.00,
        },
    });

    let left_vbox_layout = manager.create(UIVBoxLayout {
        min_height: 30.0,
        max_height: 50.0,
        hpadding: 20.0,
        vpadding: 8.0,
    });

    let right_vbox_layout = manager.create(UIVBoxLayout {
        min_height: 30.0,
        max_height: 50.0,
        hpadding: 20.0,
        vpadding: 8.0,
    });

    let hbox_layout = UIHBoxLayout {
        min_width: std::f32::MIN,
        max_width: std::f32::MAX,
        hpadding: 5.0,
        vpadding: 0.0,
    };

    let slider_layout = UISliderLayout { label_offset: 20.0 };
    let red_layout = manager.create(slider_layout);
    let green_layout = manager.create(slider_layout);
    let blue_layout = manager.create(slider_layout);
    let inner_dist_layout = manager.create(slider_layout);
    let outer_dist_layout = manager.create(slider_layout);
    let sharpness_layout = manager.create(slider_layout);

    let shadow_red_layout = manager.create(slider_layout);
    let shadow_green_layout = manager.create(slider_layout);
    let shadow_blue_layout = manager.create(slider_layout);
    let shadow_alpha_layout = manager.create(slider_layout);
    let shadow_pos_layout = manager.create(slider_layout);
    let shadow_size_layout = manager.create(slider_layout);

    let texture_size_layout = manager.create(slider_layout);
    let texture_font_size_layout = manager.create(slider_layout);
    let texture_shadow_size_layout = manager.create(slider_layout);

    let render_glyph_layout = manager.create(hbox_layout);
    let render_texture_layout = manager.create(hbox_layout);

    let texture_visibility_layout = manager.create(slider_layout);

    // Organize views

    manager.root(main_layout);
    manager.add_child(main_layout, left_drawer_layout);
    manager.add_child(left_drawer_layout, left_drawer_block);

    manager.add_child(main_layout, text_area);

    manager.add_child(main_layout, right_drawer_layout);
    manager.add_child(right_drawer_layout, right_drawer_block);

    manager.add_child(left_drawer_layout, left_vbox_layout);
    manager.add_child(right_drawer_layout, right_vbox_layout);

    manager.add_child(render_glyph_layout, render_glyph_label);
    manager.add_child(render_glyph_layout, render_glyph_value_label);

    manager.add_child(render_texture_layout, render_texture_label);
    manager.add_child(render_texture_layout, render_texture_value_label);

    // Left drawer

    manager.add_child(left_vbox_layout, outline_label);
    manager.add_child(left_vbox_layout, red_layout);
    manager.add_child(left_vbox_layout, green_layout);
    manager.add_child(left_vbox_layout, blue_layout);
    manager.add_child(left_vbox_layout, inner_dist_layout);
    manager.add_child(left_vbox_layout, outer_dist_layout);
    manager.add_child(left_vbox_layout, sharpness_layout);

    manager.add_child(left_vbox_layout, shadow_label);
    manager.add_child(left_vbox_layout, shadow_red_layout);
    manager.add_child(left_vbox_layout, shadow_green_layout);
    manager.add_child(left_vbox_layout, shadow_blue_layout);
    manager.add_child(left_vbox_layout, shadow_alpha_layout);
    manager.add_child(left_vbox_layout, shadow_pos_layout);
    manager.add_child(left_vbox_layout, shadow_size_layout);

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

    manager.add_child(shadow_red_layout, shadow_red_slider);
    manager.add_child(shadow_red_layout, shadow_red_label);
    manager.add_child(shadow_green_layout, shadow_green_slider);
    manager.add_child(shadow_green_layout, shadow_green_label);
    manager.add_child(shadow_blue_layout, shadow_blue_slider);
    manager.add_child(shadow_blue_layout, shadow_blue_label);
    manager.add_child(shadow_alpha_layout, shadow_alpha_slider);
    manager.add_child(shadow_alpha_layout, shadow_alpha_label);
    manager.add_child(shadow_pos_layout, shadow_pos_slider);
    manager.add_child(shadow_pos_layout, shadow_pos_label);
    manager.add_child(shadow_size_layout, shadow_size_slider);
    manager.add_child(shadow_size_layout, shadow_size_label);

    // Right drawer

    manager.add_child(right_vbox_layout, texture_label);
    manager.add_child(right_vbox_layout, texture_size_layout);
    manager.add_child(right_vbox_layout, texture_font_size_layout);
    manager.add_child(right_vbox_layout, texture_shadow_size_layout);

    manager.add_child(right_vbox_layout, render_stats_label);
    manager.add_child(right_vbox_layout, render_glyph_layout);
    manager.add_child(right_vbox_layout, render_texture_layout);

    manager.add_child(right_vbox_layout, other_label);
    manager.add_child(right_vbox_layout, animation_button);
    manager.add_child(right_vbox_layout, texture_visibility_layout);

    manager.add_child(texture_visibility_layout, texture_visibility_slider);
    manager.add_child(texture_visibility_layout, texture_visibility_label);

    manager.add_child(texture_size_layout, texture_size_slider);
    manager.add_child(texture_size_layout, texture_size_label);

    manager.add_child(texture_font_size_layout, texture_font_size_slider);
    manager.add_child(texture_font_size_layout, texture_font_size_label);

    manager.add_child(texture_shadow_size_layout, texture_shadow_size_slider);
    manager.add_child(texture_shadow_size_layout, texture_shadow_size_label);

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
    let mut text = String::new();

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
                glutin::WindowEvent::ReceivedCharacter(c) => {
                    if (!c.is_whitespace() && c != '\x08' &&  c != '\x7f') || c == ' ' {
                        println!("{}", c as u32);
                        text.push(c);
                    }
                    manager.update(text_area, |t| {
                        t.set_text(&text);
                    });
                }
                glutin::WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(glutin::VirtualKeyCode::Escape) = input.virtual_keycode {
                        exit = true;
                    }
                    if let Some(glutin::VirtualKeyCode::Back) = input.virtual_keycode {
                        text.pop();
                    }
                    if let Some(glutin::VirtualKeyCode::Return) = input.virtual_keycode {
                        text.push('\n');
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
                        glutin::MouseScrollDelta::LineDelta(_, y) => y * 2.0,
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
                _ => (),
            },
            glutin::Event::Awakened => {
                while let Ok(result) = renderer_result_receiver.try_recv() {
                    match result {
                        RendererResult::ShapesRendered(name, batch, avg_duration) => {
                            let texture = batch.texture.lock().unwrap();
                            let texture_upload_time = Instant::now();

                            if name == "label_context" {
                                label_context
                                    .borrow_mut()
                                    .update_texture_cache(batch.texture_id, &texture)
                                    .expect("Coudn't upload texture to label context");
                            }

                            if name == "text_area_context" {
                                manager.update(render_glyph_value_label, |l| {
                                    l.set_text(&format!("{:?}", avg_duration));
                                });

                                text_area_context
                                    .borrow_mut()
                                    .update_texture_cache(batch.texture_id, &texture)
                                    .expect("Couldn't upload texture to text area context");

                                manager.update(render_texture_value_label, |l| {
                                    l.set_text(&format!("{:?}", texture_upload_time.elapsed()));
                                });
                            }
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

        // Handle left panel actions.

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

        handle_font_style_slider!(shadow_red_slider, shadow_color, |v: f32| Color::new(
            v,
            text_style.shadow_color.g,
            text_style.shadow_color.b
        ));
        handle_font_style_slider!(shadow_green_slider, shadow_color, |v: f32| Color::new(
            text_style.shadow_color.r,
            v,
            text_style.shadow_color.b
        ));
        handle_font_style_slider!(shadow_blue_slider, shadow_color, |v: f32| Color::new(
            text_style.shadow_color.r,
            text_style.shadow_color.g,
            v
        ));
        handle_font_style_slider!(shadow_pos_slider, shadow_pos, |v: f32| v);
        handle_font_style_slider!(shadow_size_slider, shadow_size, |v: f32| v);
        handle_font_style_slider!(shadow_alpha_slider, shadow_alpha, |v: f32| v);

        // Handle right panel actions.

        handle_font_style_slider!(texture_visibility_slider, texture_visibility, |v: f32| v);

        macro_rules! handle_texture_setting {
            ($slider:expr, $func:ident) => {
                let mut value = None;
                manager.poll_events($slider, |e| {
                    match e {
                        UISliderEvent::ValueChanged(_) => {}
                        UISliderEvent::ValueFinished(v) => value = Some(*v),
                    };
                });
                if let Some(v) = value {
                    text_area_context.borrow_mut().$func(v);
                    manager.update(text_area, |t| {
                        t.invalidate();
                    });
                }
            };
        };

        handle_texture_setting!(texture_size_slider, set_texture_size);
        handle_texture_setting!(texture_font_size_slider, set_font_size);
        handle_texture_setting!(texture_shadow_size_slider, set_shadow_size);

        manager.poll_events(animation_button, |e| match e {
            UIButtonEvent::Toggled(toggled) => {
                text_style = UITextAreaStyle {
                    animation: *toggled,
                    ..text_style
                };
            }
        });
    }

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
