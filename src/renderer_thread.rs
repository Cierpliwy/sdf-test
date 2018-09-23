use glium::glutin::EventsLoopProxy;
use rayon::prelude::*;
use sdf::renderer::render_shape;
use sdf::shape::AllocatedShape;
use sdf::texture::Texture;
use std::sync::mpsc::{Receiver, RecvError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct RendererContext {
    pub receiver: Receiver<RendererCommand>,
    pub sender: Sender<RendererResult>,
    pub proxy: EventsLoopProxy,
}

pub enum RendererCommand {
    RenderShapes {
        texture: Arc<Mutex<Texture>>,
        shapes: Vec<AllocatedShape>,
    },
    Exit,
}

pub enum RendererResult {
    ShapesRendered {
        texture: Arc<Mutex<Texture>>,
        shapes: Vec<AllocatedShape>,
    },
}

pub fn renderer_entry_point(context: RendererContext) -> Result<(), RecvError> {
    println!("Renderer thread started");
    loop {
        let command = context.receiver.recv()?;
        match command {
            RendererCommand::RenderShapes {
                texture,
                mut shapes,
            } => {
                {
                    let mut texture_mutex = texture.lock().unwrap();
                    let render_time = Instant::now();

                    let texture_lock = texture_mutex.lock();

                    println!("Rendering {} shape(s)...", shapes.len());

                    shapes.par_iter_mut().for_each(|shape| {
                        render_shape(shape, &texture_lock);
                    });

                    println!(
                        "Finished rendering {} shape(s) in {:?}.",
                        shapes.len(),
                        render_time.elapsed()
                    );
                }
                context
                    .sender
                    .send(RendererResult::ShapesRendered { texture, shapes })
                    .unwrap_or_else(|_| {
                        println!("Coudn't send rendered shapes result");
                    })
            }
            RendererCommand::Exit => {
                println!("Closing renderer thread...");
                break;
            }
        };

        context.proxy.wakeup().unwrap_or_else(|_| {
            println!("Coudn't wakeup main thread!");
        });
    }
    Ok(())
}
