#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate image;
extern crate rand;
extern crate rayon;
extern crate rusttype;

pub mod font;
pub mod renderer_thread;
pub mod sdf;

use font::*;
use glium::{glutin, Surface};
use renderer_thread::{renderer_entry_point, RendererCommand, RendererContext, RendererResult};
use std::io::prelude::*;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Instant;

fn main() {
    let tex_size = std::env::args().nth(1).unwrap().parse::<u32>().unwrap();
    let font_size = std::env::args().nth(2).unwrap().parse::<u8>().unwrap();
    let shadow_size = std::env::args().nth(3).unwrap().parse::<u8>().unwrap();
    let path = std::env::args().nth(4).unwrap();

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions((tex_size, tex_size).into());
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    let mut res_x = 0.0;
    let mut res_y = 0.0;

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
        0.07,
    );

    let program = GLTextBlockLayoutProgram::new(&display).unwrap();
    let gl_layout = GLTextBlockLayout::new(&display, &layout).unwrap();
    let mut gl_texture_cache = GLFontTextureCache::new();

    let draw = |gl_texture_cache: &GLFontTextureCache| {
        let mut target = display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);
        gl_layout
            .render(&mut target, gl_texture_cache, &program)
            .unwrap();
        target.finish().unwrap();
    };

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

    for batch in font.get_texture_render_batches() {
        renderer_command_sender
            .send(RendererCommand::RenderShapes(batch))
            .unwrap();
    }

    draw(&mut gl_texture_cache);
    events_loop.run_forever(|event| {
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => return glutin::ControlFlow::Break,
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mouse_x = position.x as f32;
                    mouse_y = position.y as f32;
                    draw(&mut gl_texture_cache);
                }
                glutin::WindowEvent::Resized(position) => {
                    res_x = position.width as f32;
                    res_y = position.height as f32;
                    draw(&mut gl_texture_cache);
                }
                glutin::WindowEvent::ReceivedCharacter(_c) => {}
                _ => (),
            },
            _ => (),
        }

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
                    draw(&mut gl_texture_cache);
                }
            }
        }

        glutin::ControlFlow::Continue
    });

    renderer_command_sender
        .send(RendererCommand::Exit)
        .expect("Coudn't terminate renderer thread before exit.");

    renderer_thread
        .join()
        .expect("Couldn't join renderer thread");
}
