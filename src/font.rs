use glium::backend::Facade;
use glium::draw_parameters::DrawParameters;
use glium::index::{BufferCreationError as IndexBufferCreationError, PrimitiveType};
use glium::program::ProgramChooserCreationError;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d, TextureCreationError};
use glium::vertex::BufferCreationError as VertexBufferCreationError;
use glium::{
    implement_vertex, program, uniform, Blend, DrawError, IndexBuffer, Program, Rect as GLRect,
    Surface, VertexBuffer,
};
use sdf::font::{GlyphLayout, TextBlockLayout};
use sdf::geometry::Rect;
use sdf::texture::Texture;
use std::borrow::Cow;
use std::collections::HashMap;

pub struct GLFontTextureCache {
    textures: HashMap<u32, Texture2d>,
}

impl GLFontTextureCache {
    pub fn new() -> Self {
        GLFontTextureCache {
            textures: HashMap::new(),
        }
    }

    pub fn update_texture<F: ?Sized + Facade>(
        &mut self,
        id: u32,
        texture: &Texture,
        facade: &F,
    ) -> Result<(), TextureCreationError> {
        let raw_texture = RawImage2d {
            data: Cow::Borrowed(texture.get_data()),
            width: texture.get_width(),
            height: texture.get_height(),
            format: ClientFormat::U8U8U8,
        };

        let new_texture = if let Some(current_texture) = self.textures.get_mut(&id) {
            current_texture.write(
                GLRect {
                    left: 0,
                    bottom: 0,
                    width: texture.get_width(),
                    height: texture.get_height(),
                },
                raw_texture,
            );
            None
        } else {
            Some(Texture2d::with_mipmaps(
                facade,
                raw_texture,
                MipmapsOption::NoMipmap,
            )?)
        };

        if let Some(new_texture) = new_texture {
            self.textures.insert(id, new_texture);
        }

        Ok(())
    }

    pub fn get_texture(&self, id: u32) -> Option<&Texture2d> {
        self.textures.get(&id)
    }
}

#[derive(Copy, Clone)]
struct GLTextBlockLayoutVertex {
    pos: [f32; 2],
    coord: [f32; 2],
}

implement_vertex!(GLTextBlockLayoutVertex, pos, coord);

impl GLTextBlockLayoutVertex {
    fn new(pos_x: f32, pos_y: f32, coord_x: f32, coord_y: f32) -> Self {
        Self {
            pos: [pos_x, pos_y],
            coord: [coord_x, coord_y],
        }
    }
}

pub struct GLTextBlockLayout {
    shadow_size: u8,
    font_size: u8,
    bounding_box: Rect<f32>,
    passes: HashMap<u32, GLTextBlockLayoutPass>,
}

struct GLTextBlockLayoutPass {
    vertex_buffer: VertexBuffer<GLTextBlockLayoutVertex>,
    index_buffer: IndexBuffer<u16>,
}

#[derive(Debug)]
pub enum GLTextBlockLayoutError {
    IndexBufferCreationError(IndexBufferCreationError),
    VertexBufferCreationError(VertexBufferCreationError),
}

impl From<IndexBufferCreationError> for GLTextBlockLayoutError {
    fn from(error: IndexBufferCreationError) -> Self {
        GLTextBlockLayoutError::IndexBufferCreationError(error)
    }
}

impl From<VertexBufferCreationError> for GLTextBlockLayoutError {
    fn from(error: VertexBufferCreationError) -> Self {
        GLTextBlockLayoutError::VertexBufferCreationError(error)
    }
}

impl GLTextBlockLayout {
    pub fn new<F: ?Sized + Facade>(
        facade: &F,
        text_block_layout: &TextBlockLayout,
    ) -> Result<Self, GLTextBlockLayoutError> {
        struct PassData {
            vertices: Vec<GLTextBlockLayoutVertex>,
            indices: Vec<u16>,
        };

        fn update_pass_data(pass_data: &mut PassData, glyph_layout: &GlyphLayout) {
            let new_index = pass_data.vertices.len();
            let scr = glyph_layout.screen_coord;
            let tex = glyph_layout.texture_coord;

            let tl = GLTextBlockLayoutVertex::new(scr.min.x, scr.max.y, tex.min.x, tex.max.y);
            let tr = GLTextBlockLayoutVertex::new(scr.max.x, scr.max.y, tex.max.x, tex.max.y);
            let bl = GLTextBlockLayoutVertex::new(scr.min.x, scr.min.y, tex.min.x, tex.min.y);
            let br = GLTextBlockLayoutVertex::new(scr.max.x, scr.min.y, tex.max.x, tex.min.y);

            pass_data.vertices.push(tl);
            pass_data.vertices.push(tr);
            pass_data.vertices.push(br);
            pass_data.vertices.push(bl);

            pass_data.indices.push(new_index as u16);
            pass_data.indices.push((new_index + 1) as u16);
            pass_data.indices.push((new_index + 2) as u16);
            pass_data.indices.push((new_index + 2) as u16);
            pass_data.indices.push((new_index + 3) as u16);
            pass_data.indices.push(new_index as u16);
        }

        let mut passes = HashMap::<u32, PassData>::new();
        for glyph_layout in &text_block_layout.glyph_layouts {
            let pass_data = passes.entry(glyph_layout.texture_id).or_insert(PassData {
                vertices: Vec::new(),
                indices: Vec::new(),
            });
            update_pass_data(pass_data, glyph_layout);
        }

        let mut gl_passes = HashMap::<u32, GLTextBlockLayoutPass>::new();
        for (id, pass_data) in passes {
            let vertex_buffer = VertexBuffer::immutable(facade, pass_data.vertices.as_slice())?;
            let index_buffer = IndexBuffer::immutable(
                facade,
                PrimitiveType::TrianglesList,
                pass_data.indices.as_slice(),
            )?;
            gl_passes.insert(
                id,
                GLTextBlockLayoutPass {
                    vertex_buffer,
                    index_buffer,
                },
            );
        }

        Ok(GLTextBlockLayout {
            passes: gl_passes,
            shadow_size: text_block_layout.shadow_size,
            font_size: text_block_layout.font_size,
            bounding_box: text_block_layout.bounding_box,
        })
    }

    pub fn render<S: ?Sized + Surface>(
        &self,
        surface: &mut S,
        texture_cache: &GLFontTextureCache,
        program: &GLTextBlockLayoutProgram,
        config: &GLTextBlockLayoutConfig,
    ) -> Result<(), DrawError> {
        let sharpness = config.font_sharpness
            / self.shadow_size as f32
            / (config.font_size / self.font_size as f32);

        for (texture_id, pass_data) in &self.passes {
            if let Some(texture) = texture_cache.get_texture(*texture_id) {
                surface.draw(
                    &pass_data.vertex_buffer,
                    &pass_data.index_buffer,
                    &program.program,
                    &uniform! {
                        uTexture: texture,
                        uSharpness: sharpness,
                        uFontSize: config.font_size,
                        uPosition: [config.position_x, config.position_y],
                        uScreen: [config.screen_width as f32, config.screen_height as f32]
                    },
                    &DrawParameters {
                        blend: Blend::alpha_blending(),
                        color_mask: (true, true, true, false),
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(())
    }
}

pub struct GLTextBlockLayoutConfig {
    pub font_size: f32,
    pub font_sharpness: f32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub position_x: f32,
    pub position_y: f32,
}

pub struct GLTextBlockLayoutProgram {
    program: Program,
}

impl GLTextBlockLayoutProgram {
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Result<Self, ProgramChooserCreationError> {
        let program = program!(facade, 140 => {
        vertex: r#"
            #version 140
            
            in vec2 pos;
            in vec2 coord;

            out vec2 vCoord;

            uniform float uFontSize;
            uniform vec2 uScreen;
            uniform vec2 uPosition;

            void main() {
                gl_Position = vec4(uPosition / uScreen * 2.0 - 1.0 + pos * uFontSize / uScreen, 0.0, 1.0);
                vCoord = coord;
            }
        "#,
        fragment: r#"
            #version 140

            in vec2 vCoord;
            out vec4 color;

            uniform sampler2D uTexture;
            uniform float uSharpness;

            float median(float a, float b, float c) {
                return max(min(a,b), min(max(a,b),c));
            }

            void main() {
                vec4 t = texture(uTexture, vCoord);
                float d = median(t.r, t.g, t.b);
                color = vec4(0.0, 0.0, 0.0, smoothstep(0.5 - uSharpness, 0.5 + uSharpness, d));
            }
        "#,
        })?;

        Ok(Self { program })
    }
}
