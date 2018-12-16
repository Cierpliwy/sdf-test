use crate::ui::{UIContext, UILayout};
use glium::draw_parameters::DrawParameters;
use glium::index::PrimitiveType;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d, TextureCreationError};
use glium::{
    implement_vertex, program, uniform, Blend, IndexBuffer, Program, Rect as GLRect, Surface,
    VertexBuffer,
};
use sdf::font::{Font, GlyphLayout, TextureRenderBatch};
use sdf::geometry::Rect;
use sdf::texture::Texture;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct UILabelContext {
    context: UIContext,
    program: Program,
    font: Font,
    texture_cache: HashMap<u32, Texture2d>,
}

impl UILabelContext {
    pub fn new(context: UIContext, font: Font) -> Self {
        let texture_cache = HashMap::new();

        let program = program!(&context.gl_context, 140 => {
        vertex: r#"
            #version 140

            in vec2 pos;
            in vec2 coord;

            out vec2 vCoord;

            uniform float uFontSize;
            uniform vec2 uScreen;
            uniform vec2 uPosition;

            void main() {
                gl_Position = vec4((uPosition + pos * uFontSize) * 2.0 / uScreen - 1.0, 0.0, 1.0);
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
                color = vec4(0.0, 0.0, 0.0, smoothstep(0.4 - uSharpness, 0.4 + uSharpness, d));
            }
        "#,
        })
        .expect("Cannot create program for label");

        Self {
            program,
            context,
            font,
            texture_cache,
        }
    }

    pub fn update_texture_cache(
        &mut self,
        id: u32,
        texture: &Texture,
    ) -> Result<(), TextureCreationError> {
        let raw_texture = RawImage2d {
            data: Cow::Borrowed(texture.get_data()),
            width: texture.get_width(),
            height: texture.get_height(),
            format: ClientFormat::U8U8U8,
        };

        let new_texture = if let Some(current_texture) = self.texture_cache.get_mut(&id) {
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
                &self.context.gl_context,
                raw_texture,
                MipmapsOption::NoMipmap,
            )?)
        };

        if let Some(new_texture) = new_texture {
            self.texture_cache.insert(id, new_texture);
        }

        Ok(())
    }

    pub fn get_texture(&self, id: u32) -> Option<&Texture2d> {
        self.texture_cache.get(&id)
    }

    pub fn get_texture_render_batches(&mut self) -> Vec<TextureRenderBatch> {
        self.font.get_texture_render_batches()
    }
}

#[derive(Copy, Clone)]
struct UILabelGlyphVertex {
    pos: [f32; 2],
    coord: [f32; 2],
}

implement_vertex!(UILabelGlyphVertex, pos, coord);

impl UILabelGlyphVertex {
    fn new(pos_x: f32, pos_y: f32, coord_x: f32, coord_y: f32) -> Self {
        Self {
            pos: [pos_x, pos_y],
            coord: [coord_x, coord_y],
        }
    }
}

struct UILabelRenderPass {
    vertex_buffer: VertexBuffer<UILabelGlyphVertex>,
    index_buffer: IndexBuffer<u16>,
}

pub enum UILabelAlignment {
    Left,
    Right,
    Center,
}

pub struct UILabel {
    align: UILabelAlignment,
    text: String,
    size: f32,
    bounding_box: Rect<f32>,
    passes: HashMap<u32, UILabelRenderPass>,
    context: Rc<RefCell<UILabelContext>>,
}

impl UILabel {
    pub fn new(
        context: Rc<RefCell<UILabelContext>>,
        text: &str,
        size: f32,
        align: UILabelAlignment,
    ) -> Self {
        let mut label = Self {
            align,
            context,
            size,
            text: String::new(),
            bounding_box: Rect::new(0.0, 0.0, 0.0, 0.0),
            passes: HashMap::new(),
        };

        label.set_text(text);
        label
    }

    pub fn get_bounding_box(&self) -> Rect<f32> {
        let bb = self.bounding_box;
        let size = self.size;
        Rect::new(
            bb.min.x * size,
            bb.min.y * size,
            bb.max.x * size,
            bb.max.y * size,
        )
    }

    pub fn set_size(&mut self, size: f32) {
        self.size = size
    }

    pub fn set_text(&mut self, text: &str) {
        if self.text == text {
            return;
        }
        self.text = text.into();

        let mut context = self.context.borrow_mut();
        let text_layout = context.font.layout_text_block(text);
        let gl_context = &context.context.gl_context;

        struct PassData {
            vertices: Vec<UILabelGlyphVertex>,
            indices: Vec<u16>,
        };

        fn update_pass_data(pass_data: &mut PassData, glyph_layout: &GlyphLayout) {
            let new_index = pass_data.vertices.len();
            let scr = glyph_layout.screen_coord;
            let tex = glyph_layout.texture_coord;

            let tl = UILabelGlyphVertex::new(scr.min.x, scr.max.y, tex.min.x, tex.max.y);
            let tr = UILabelGlyphVertex::new(scr.max.x, scr.max.y, tex.max.x, tex.max.y);
            let bl = UILabelGlyphVertex::new(scr.min.x, scr.min.y, tex.min.x, tex.min.y);
            let br = UILabelGlyphVertex::new(scr.max.x, scr.min.y, tex.max.x, tex.min.y);

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
        for glyph_layout in &text_layout.glyph_layouts {
            let pass_data = passes.entry(glyph_layout.texture_id).or_insert(PassData {
                vertices: Vec::new(),
                indices: Vec::new(),
            });
            update_pass_data(pass_data, glyph_layout);
        }

        let mut gl_passes = HashMap::<u32, UILabelRenderPass>::new();
        for (id, pass_data) in passes {
            let vertex_buffer = VertexBuffer::immutable(gl_context, pass_data.vertices.as_slice())
                .expect("Cannot create vertex buffer for label");

            let index_buffer = IndexBuffer::immutable(
                gl_context,
                PrimitiveType::TrianglesList,
                pass_data.indices.as_slice(),
            )
            .expect("Cannot create index buffer for label");

            gl_passes.insert(
                id,
                UILabelRenderPass {
                    vertex_buffer,
                    index_buffer,
                },
            );
        }

        self.passes = gl_passes;
        self.bounding_box = text_layout.bounding_box;
    }

    pub fn render<S: ?Sized + Surface>(&self, surface: &mut S, layout: &UILayout) {
        let context = self.context.borrow_mut();
        let shadow_size = context.font.get_shadow_size();
        let font_size = context.font.get_font_size();
        let font_sharpness = 0.4;
        let sharpness = font_sharpness / shadow_size as f32 / (self.size / font_size as f32);
        let mut pos = layout.get_pos();
        let size = layout.get_size();
        let screen = layout.get_screen();

        let bb = self.get_bounding_box();
        pos[1] = pos[1] - (bb.height() - size[1]) / 2.0;
        match self.align {
            UILabelAlignment::Left => {}
            UILabelAlignment::Right => {
                pos[0] = pos[0] + size[0] - bb.width();
            }
            UILabelAlignment::Center => {
                pos[0] = pos[0] + (size[0] - bb.width()) / 2.0;
            }
        };

        for (texture_id, pass_data) in &self.passes {
            if let Some(texture) = context.get_texture(*texture_id) {
                surface
                    .draw(
                        &pass_data.vertex_buffer,
                        &pass_data.index_buffer,
                        &context.program,
                        &uniform! {
                            uTexture: texture,
                            uSharpness: sharpness,
                            uFontSize: self.size,
                            uPosition: pos,
                            uScreen: screen.get_size()
                        },
                        &DrawParameters {
                            blend: Blend::alpha_blending(),
                            color_mask: (true, true, true, false),
                            ..Default::default()
                        },
                    )
                    .expect("Cannot draw UILabel pass");
            }
        }
    }
}
