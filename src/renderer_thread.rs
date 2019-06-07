use glium::glutin::EventsLoopProxy;
use rayon::prelude::*;
use mcsdf::font::TextureRenderBatch;
use mcsdf::renderer::render_shape;
use std::sync::mpsc::{Receiver, RecvError, Sender};
use std::time::Instant;

pub struct RendererContext {
    pub receiver: Receiver<RendererCommand>,
    pub sender: Sender<RendererResult>,
    pub proxy: EventsLoopProxy,
}

pub enum RendererCommand {
    RenderShapes(String, TextureRenderBatch),
    Exit,
}

pub enum RendererResult {
    ShapesRendered(String, TextureRenderBatch, std::time::Duration),
}

#[allow(clippy::needless_pass_by_value)]
pub fn renderer_entry_point(context: RendererContext) -> Result<(), RecvError> {
    println!("Renderer thread started");
    loop {
        let command = context.receiver.recv()?;
        match command {
            RendererCommand::RenderShapes(name, mut batch) => {
                let time = {
                    let mut texture_mutex = batch.texture.lock().unwrap();
                    let render_time = Instant::now();
                    let texture_lock = texture_mutex.lock();

                    batch.allocated_shapes.par_iter_mut().for_each(|shape| {
                        render_shape(shape, &texture_lock);
                    });

                    render_time.elapsed() / batch.allocated_shapes.len() as u32
                };

                context
                    .sender
                    .send(RendererResult::ShapesRendered(name, batch, time))
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
