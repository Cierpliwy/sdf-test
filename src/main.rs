pub mod font;
pub mod renderer_thread;
pub mod text;

pub mod ui;

use crate::font::*;
use crate::renderer_thread::{
    renderer_entry_point, RendererCommand, RendererContext, RendererResult,
};
use crate::text::*;
use crate::ui::block::*;
use crate::ui::*;

use glium::{glutin, Surface};
use std::cell::Cell;
use std::io::prelude::*;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

fn main() {
    let tex_size = 1024;
    let font_size = 64;
    let shadow_size = 8;
    let render_size = 64;
    let path = std::env::args().nth(1).unwrap();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_dimensions((800, 600).into())
        .with_title("Multi-channel signed distance font demo - by Cierpliwy");
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let screen: UIScreen = Rc::new(Cell::new(UIScreenInfo::new([800.0, 600.0], 1.0)));

    let ui_ctx = UIContext::new(&display);
    let block_ctx = Rc::new(UIBlockContext::new(ui_ctx).expect("Creating UI Block Context failed"));
    let block_style = UIBlockStyle {
        alpha: 0.95,
        sharpness: 1.0,
        radius: 50.0,
        left_offset: -10.0,
        left_color: [1.0, 0.7, 8.0],
        right_offset: 450.0,
        right_color: [6.0, 1.0, 9.0],
        inner_shadow: 60.0,
        shade_color: [0.0, 0.0, 0.0],
    };

    let block_layout = UIAbsoluteLayout::new(&screen, [100.0, 100.0], [300.0, 50.0]);
    let block = UIBlock::new(block_ctx.clone());

    let block_layout2 = UIRelativeLayout::new(&screen, [0.1, 0.2], [0.5, 0.8]);
    let block2 = UIBlock::new(block_ctx.clone());

    let mut font_data = Vec::<u8>::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_end(&mut font_data)
        .unwrap();
    let mut font =
        sdf::font::Font::new(tex_size, tex_size, font_size, shadow_size, font_data).unwrap();

    let layout = font.layout_text_block(
        r##"Lorem Ipsum jest tekstem stosowanym jako przykładowy
wypełniacz w przemyśle poligraficznym. Został po raz pierwszy
użyty w XV w. przez nieznanego drukarza do wypełnienia 
tekstem próbnej książki. Pięć wieków później zaczął być
używany przemyśle elektronicznym, pozostając praktycznie
niezmienionym. Spopularyzował się w latach 60. XX w. wraz
z publikacją arkuszy Letrasetu, zawierających fragmenty
Lorem Ipsum, a ostatnio z zawierającym różne wersje Lorem
Ipsum oprogramowaniem przeznaczonym do realizacji druków
na komputerach osobistych, jak Aldus PageMaker"##,
    );

    let mut text = GLText::new(30.0, &display);
    text.set_text("   Hello there this is\na\n    something new.", &mut font);

    let mut config = GLTextBlockLayoutConfig {
        font_size: render_size as f32,
        font_sharpness: 0.5,
        screen_width: tex_size,
        screen_height: tex_size,
        position_x: 0.0,
        position_y: 0.0,
    };

    let program = GLTextBlockLayoutProgram::new(&display).unwrap();
    let gl_layout = GLTextBlockLayout::new(&display, &layout).unwrap();
    let mut gl_texture_cache = GLFontTextureCache::new();

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
        // Draw scene
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        gl_layout
            .render(&mut target, &gl_texture_cache, &program, &config)
            .unwrap();

        block
            .render(&mut target, &block_style, &block_layout)
            .unwrap();

        block2
            .render(&mut target, &block_style, &block_layout2)
            .unwrap();

        // Vsync
        target.finish().unwrap();

        // Send fonts to renderer thread
        for batch in font.get_texture_render_batches() {
            renderer_command_sender
                .send(RendererCommand::RenderShapes(batch))
                .unwrap();
        }

        // Handle window events
        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => exit = true,
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    if scale {
                        config.font_size = position.y as f32 / config.screen_height as f32 * 500.0;
                        config.font_sharpness = position.x as f32 / config.screen_width as f32;
                    }

                    let pos = [
                        position.x as f32,
                        config.screen_height as f32 - position.y as f32,
                    ];

                    config.position_x = pos[0];
                    config.position_y = pos[1];
                }
                glutin::WindowEvent::Resized(position) => {
                    let res = [position.width as f32, position.height as f32];
                    config.screen_width = res[0] as u32;
                    config.screen_height = res[1] as u32;
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
                            gl_texture_cache
                                .update_texture(batch.texture_id, &texture, &display)
                                .unwrap();
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
