#[macro_use]
extern crate glium;
extern crate cgmath;
extern crate image;
extern crate rand;
extern crate rayon;
extern crate rusttype;

pub mod sdf;
use cgmath::Point2;
use glium::index::PrimitiveType;
use glium::texture::ClientFormat;
use glium::{glutin, Surface};
use rayon::prelude::*;
use sdf::font::create_segments_from_glyph_contours;
use sdf::geometry::{Curve, Line};
use sdf::shape::{SegmentPrimitive, Shape};
use sdf::texture::Texture;
use std::alloc::System;
use std::io::prelude::*;
use std::time::Instant;

#[global_allocator]
static GLOBAL: System = System;

fn main() {
    let tex_size = std::env::args().nth(1).unwrap().parse::<u32>().unwrap();
    let font_size = std::env::args().nth(2).unwrap().parse::<f32>().unwrap();
    let shade_size = std::env::args().nth(3).unwrap().parse::<f32>().unwrap();
    let path = std::env::args().nth(4).unwrap();

    let mut font = Vec::<u8>::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_end(&mut font)
        .unwrap();
    let font = rusttype::Font::from_bytes(font).unwrap();

    let possible_glyphs = (64..)
        .filter_map(|n| std::char::from_u32(n))
        .filter_map(|c| {
            font.glyph(c)
                .scaled(rusttype::Scale::uniform(font_size))
                .shape()
        });

    let (mut texture, mut allocator) = Texture::new(tex_size, tex_size);
    let mut views = Vec::new();

    let gen_time = Instant::now();
    for shape in possible_glyphs {
        let mut primitives = create_segments_from_glyph_contours(&shape);
        if let Some(view) = Shape::new(primitives, &mut allocator, shade_size) {
            views.push(view);
        } else {
            break;
        }
    }

    println!(
        "{} texture views allocated: {:?}",
        views.len(),
        gen_time.elapsed()
    );

    {
        let render_time = Instant::now();
        let lock = texture.lock();
        views.par_iter_mut().for_each(|view| {
            view.render(&lock);
        });
        println!("Rendered: {:?}", render_time.elapsed());
    }

    // let save_time = Instant::now();
    // image::png::PNGEncoder::new(std::fs::File::create("demo.png").unwrap())
    //     .encode(
    //         texture.get_data(),
    //         texture.get_width(),
    //         texture.get_height(),
    //         image::ColorType::RGB(8),
    //     ).unwrap();
    // println!("Saved image: {:?}", save_time.elapsed());

    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_dimensions((tex_size, tex_size).into());
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let vertex_buffer = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
            coord: [f32; 2],
        }

        implement_vertex!(Vertex, position, coord);

        glium::VertexBuffer::new(
            &display,
            &[
                Vertex {
                    position: [-1.0, -1.0],
                    coord: [0.0, 1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                    coord: [1.0, 1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                    coord: [1.0, 0.0],
                },
                Vertex {
                    position: [-1.0, 1.0],
                    coord: [0.0, 0.0],
                },
            ],
        ).unwrap()
    };

    let index_buffer = glium::IndexBuffer::new(
        &display,
        PrimitiveType::TrianglesList,
        &[0u16, 1, 2, 2, 3, 0],
    ).unwrap();

    let image = glium::texture::RawImage2d {
        data: std::borrow::Cow::Borrowed(texture.get_data()),
        width: texture.get_width(),
        height: texture.get_height(),
        format: ClientFormat::U8U8U8,
    };

    let texture = glium::texture::Texture2d::with_mipmaps(
        &display,
        image,
        glium::texture::MipmapsOption::NoMipmap,
    ).unwrap();

    let program = program!(&display, 140 => {
        vertex: r#"
            #version 140
            
            in vec2 position;
            in vec2 coord;
            out vec2 vCoord;
            out float scale;

            uniform vec2 mouse;
            uniform vec2 res;

            void main() {
                scale = mouse.x / res.x * 8.0;
                gl_Position = vec4(position * scale, 0.0, 1.0);
                vCoord = coord;
            }
        "#,
        fragment: r#"
            #version 140

            in vec2 vCoord;
            in float scale;
            out vec4 color;

            uniform sampler2D tex;
            uniform vec2 mouse;
            uniform vec2 res;
            uniform float tex_size;
            uniform float shade_size;

            float median(float a, float b, float c) {
                return max(min(a,b), min(max(a,b),c));
            }

            void main() {
                vec4 s = texture(tex, vCoord);
                float d = median(s.r, s.g, s.b);
                float z = 0.25 / (shade_size * scale);
                float h = mouse.y / res.y;
                vec4 shadow = vec4(mix(vec3(1.0), vec3(0.8, 0.8, 0.8), smoothstep(h*h, h, d)), 1.0);
                color = mix(shadow, vec4(0.0, 0.0, 0.0, 1.0), smoothstep(h - z, h + z, d));
            }
        "#,
    }).unwrap();

    let mut mouse_x = 0.0;
    let mut mouse_y = 0.0;
    let mut res_x = 0.0;
    let mut res_y = 0.0;

    let draw = |mouse_x: f32, mouse_y: f32, res_x: f32, res_y: f32| {
        let mut target = display.draw();
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target
            .draw(
                &vertex_buffer,
                &index_buffer,
                &program,
                &uniform!{
                    tex: &texture,
                    mouse: [mouse_x, mouse_y],
                    res: [res_x, res_y],
                    tex_size: tex_size as f32,
                    shade_size: shade_size
                },
                &Default::default(),
            ).unwrap();
        target.finish().unwrap();
    };

    draw(mouse_x, mouse_y, res_x, res_y);

    events_loop.run_forever(|event| {
        match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => return glutin::ControlFlow::Break,
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    mouse_x = position.x as f32;
                    mouse_y = position.y as f32;
                    draw(mouse_x, mouse_y, res_x, res_y);
                }
                glutin::WindowEvent::Resized(position) => {
                    res_x = position.width as f32;
                    res_y = position.height as f32;
                    draw(mouse_x, mouse_y, res_x, res_y);
                }
                _ => (),
            },
            _ => (),
        }
        glutin::ControlFlow::Continue
    });
}
