use crate::ui::widget::{UILayout, UISize, UIWidget};
use glium::backend::{Context, Facade};
use glium::draw_parameters::DrawParameters;
use glium::index::PrimitiveType;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d, TextureCreationError};
use glium::{
    implement_vertex, program, uniform, Blend, Frame, IndexBuffer, Program, Rect as GLRect,
    Surface, VertexBuffer,
};
use mcsdf::font::{Font, GlyphLayout, TextureRenderBatch};
use mcsdf::geometry::Rect;
use mcsdf::texture::Texture;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct UILabelContext {
    context: Rc<Context>,
    program: Program,
    font: Font,
    texture_cache: HashMap<u32, Texture2d>,
}

impl UILabelContext {
    #[allow(clippy::redundant_closure)]
    pub fn new<F: ?Sized + Facade>(facade: &F, font: Font) -> Self {
        let context = facade.get_context().clone();
        let texture_cache = HashMap::new();

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
            uniform vec4 uColor;
            uniform vec4 uShadowColor;
            uniform float uOpacity;

            float median(float a, float b, float c) {
                return max(min(a,b), min(max(a,b),c));
            }

            void main() {
                vec4 t = texture(uTexture, vCoord);
                float d = median(t.r, t.g, t.b);
                float alpha = smoothstep(0.6, 0.3, d);
                color = mix(uColor, uShadowColor, alpha);
                color.a = color.a * smoothstep(0.45 - uSharpness, 0.45 + uSharpness, d) * uOpacity;
            }
        "#,
        })
        .expect("Cannot create program for label");

        Self {
            context,
            program,
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
                &self.context,
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

#[derive(Copy, Clone)]
pub enum UILabelAlignment {
    Left,
    Right,
    Center,
}

#[derive(Copy, Clone)]
pub struct UILabelStyle {
    pub align: UILabelAlignment,
    pub size: f32,
    pub color: [f32; 4],
    pub shadow_color: [f32; 4],
    pub opacity: f32,
}

pub struct UILabel {
    style: UILabelStyle,
    text: String,
    bounding_box: Rect<f32>,
    passes: HashMap<u32, UILabelRenderPass>,
    context: Rc<RefCell<UILabelContext>>,
}

impl UILabel {
    pub fn new(context: Rc<RefCell<UILabelContext>>, text: &str, style: UILabelStyle) -> Self {
        let mut label = Self {
            context,
            text: String::new(),
            bounding_box: Rect::new(0.0, 0.0, 0.0, 0.0),
            passes: HashMap::new(),
            style,
        };

        label.set_text(text);
        label
    }

    pub fn get_style(&self) -> UILabelStyle {
        self.style
    }

    pub fn set_style(&mut self, style: UILabelStyle) {
        self.style = style;
    }

    pub fn get_bounding_box(&self, style: UILabelStyle) -> Rect<f32> {
        let bb = self.bounding_box;
        let size = style.size;
        Rect::new(
            bb.min.x * size,
            bb.min.y * size,
            bb.max.x * size,
            bb.max.y * size,
        )
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.style.color = color;
    }

    pub fn set_shadow_color(&mut self, shadow_color: [f32; 4]) {
        self.style.shadow_color = shadow_color;
    }

    pub fn set_size(&mut self, size: f32) {
        self.style.size = size
    }

    pub fn set_text(&mut self, text: &str) {
        if self.text == text {
            return;
        }
        self.text = text.into();

        let mut context = self.context.borrow_mut();
        let text_layout = context.font.layout_text_block(text);
        let gl_context = &context.context;

        struct PassData {
            vertices: Vec<UILabelGlyphVertex>,
            indices: Vec<u16>,
        }

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

    pub fn render_styled(
        &self,
        frame: &mut Frame,
        layout: UILayout,
        style: UILabelStyle,
        screen: UISize,
    ) {
        let mut pos = [layout.left, layout.top];
        let size = [layout.width, layout.height];
        let screen = [screen.width, screen.height];

        let context = self.context.borrow_mut();
        let shadow_size = context.font.get_shadow_size();
        let font_size = context.font.get_font_size();
        let font_sharpness = 0.4;
        let sharpness =
            font_sharpness / f32::from(shadow_size) / (style.size / f32::from(font_size));

        let bb = self.get_bounding_box(style);
        pos[1] -= (bb.height() - size[1]) / 2.0;
        match style.align {
            UILabelAlignment::Left => {}
            UILabelAlignment::Right => {
                pos[0] += size[0] - bb.width();
            }
            UILabelAlignment::Center => {
                pos[0] += (size[0] - bb.width()) / 2.0;
            }
        };

        for (texture_id, pass_data) in &self.passes {
            if let Some(texture) = context.get_texture(*texture_id) {
                frame
                    .draw(
                        &pass_data.vertex_buffer,
                        &pass_data.index_buffer,
                        &context.program,
                        &uniform! {
                            uTexture: texture,
                            uSharpness: sharpness,
                            uFontSize: style.size,
                            uPosition: pos,
                            uScreen: screen,
                            uColor: style.color,
                            uOpacity: style.opacity,
                            uShadowColor: style.shadow_color
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

impl UIWidget for UILabel {
    type Event = ();

    fn render(&self, frame: &mut Frame, layout: UILayout, screen: UISize) {
        self.render_styled(frame, layout, self.style, screen)
    }
}
