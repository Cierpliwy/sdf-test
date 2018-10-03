use glium::glutin::EventsLoopProxy;
use rayon::prelude::*;
use sdf::font::TextureRenderBatch;
use sdf::renderer::render_shape;
use std::sync::mpsc::{Receiver, RecvError, Sender};
use std::time::Instant;

pub struct RendererContext {
    pub receiver: Receiver<RendererCommand>,
    pub sender: Sender<RendererResult>,
    pub proxy: EventsLoopProxy,
}

pub enum RendererCommand {
    RenderShapes(TextureRenderBatch),
    Exit,
}

pub enum RendererResult {
    ShapesRendered(TextureRenderBatch),
}

pub fn renderer_entry_point(context: RendererContext) -> Result<(), RecvError> {
    println!("Renderer thread started");
    loop {
        let command = context.receiver.recv()?;
        match command {
            RendererCommand::RenderShapes(mut batch) => {
                {
                    let mut texture_mutex = batch.texture.lock().unwrap();
                    let render_time = Instant::now();

                    let texture_lock = texture_mutex.lock();

                    println!("Rendering {} shape(s)...", batch.allocated_shapes.len());

                    batch.allocated_shapes.par_iter_mut().for_each(|shape| {
                        render_shape(shape, &texture_lock);
                    });

                    println!(
                        "Finished rendering {} shape(s) in {:?}.",
                        batch.allocated_shapes.len(),
                        render_time.elapsed()
                    );
                }
                context
                    .sender
                    .send(RendererResult::ShapesRendered(batch))
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
