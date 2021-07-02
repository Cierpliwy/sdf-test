use crate::ui::widget::{UIFrameInput, UILayout, UIPoint, UISize, UIWidget};
use glium::backend::{Context, Facade};
use glium::draw_parameters::DrawParameters;
use glium::index::PrimitiveType;
use glium::texture::{ClientFormat, MipmapsOption, RawImage2d, Texture2d, TextureCreationError};
use glium::uniforms::{AsUniformValue, UniformValue};
use glium::{
    implement_vertex, program, uniform, Blend, Frame, IndexBuffer, Program, Rect as GLRect,
    Surface, VertexBuffer,
};
use mcsdf::font::{Font, TextBlockLayout, TextureRenderBatch};
use mcsdf::texture::Texture;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;

use std::collections::VecDeque;
use std::rc::Rc;
#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b }
    }

    pub fn black() -> Self {
        Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }

    pub fn white() -> Self {
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        }
    }
}

impl AsUniformValue for Color {
    fn as_uniform_value(&self) -> UniformValue {
        UniformValue::Vec4([self.r, self.g, self.b, 1.0])
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UITextAreaStyle {
    pub text_size: f32,
    pub inner_dist: f32,
    pub outer_dist: f32,
    pub sharpness: f32,
    pub shadow_dist: f32,
    pub text_color: Color,
    pub shadow_color: Color,
    pub shadow_pos: f32,
    pub shadow_size: f32,
    pub shadow_alpha: f32,
    pub texture_visibility: f32,
    pub animation: bool,
}

impl Default for UITextAreaStyle {
    fn default() -> Self {
        UITextAreaStyle {
            text_size: 32.0,
            inner_dist: 0.0,
            outer_dist: 0.5,
            sharpness: 0.4,
            shadow_dist: 0.5,
            text_color: Color::black(),
            shadow_color: Color::black(),
            shadow_pos: 0.0,
            shadow_size: 0.0,
            shadow_alpha: 0.0,
            texture_visibility: 0.0,
            animation: false,
        }
    }
}

pub struct UITextAreaContext {
    context: Rc<Context>,
    program: Program,
    font: Font,
    texture_cache: HashMap<u32, Texture2d>,
}

impl UITextAreaContext {
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
            out vec2 vPos;

            uniform float uFontSize;
            uniform vec2 uScreen;
            uniform vec2 uPosition;

            void main() {
                vPos = (uPosition + pos * uFontSize) * 2.0 / uScreen - 1.0;
                vCoord = coord;
                gl_Position = vec4(vPos, 0.0, 1.0);
            }
        "#,
        fragment: r#"
            #version 140

            in vec2 vCoord;
            in vec2 vPos;

            out vec4 color;

            uniform sampler2D uTexture;
            uniform float uSharpness;
            uniform float uInnerDist;
            uniform float uOuterDist;
            uniform vec4 uColor;
            uniform vec4 uShadowColor;
            uniform float uShadowPos;
            uniform float uShadowSize;
            uniform float uShadowAlpha;
            uniform float uTextureVisibility;
            uniform vec2 uMouse;
            uniform bool uAnimation;
            uniform vec2 uScreen;
            uniform float uFontSize;

            float median(float a, float b, float c) {
                return max(min(a,b), min(max(a,b),c));
            }

            void main() {
                vec4 t = texture(uTexture, vCoord);
                float d = median(t.r, t.g, t.b);

                if (uAnimation) {
                    float mouse_dist = length(vPos - (uMouse / uScreen - vec2(0.5)) * 2.0);
                    d = d * (1.0 + 1.0 * clamp(1.0 - mouse_dist * 2.0, 0.0, 1.0));
                }

                vec4 outline_color = uColor;
                float outer_alpha = smoothstep(uOuterDist - uSharpness, uOuterDist + uSharpness, d);
                float inner_alpha = uInnerDist == 1.0 ? 1.0 : smoothstep(uInnerDist + uSharpness, uInnerDist - uSharpness, d);
                outline_color.a = inner_alpha * outer_alpha;

                vec4 shadow_color = uShadowColor;
                shadow_color.a = (1.0 - clamp(abs(d - uShadowPos) / uShadowSize, 0.0, 1.0)) * uShadowAlpha;

                vec4 font_color = mix(outline_color, shadow_color, 1.0 - outline_color.a);
                color = mix(font_color, t, uTextureVisibility);
            }
        "#,
        })
        .expect("Cannot create program for text area");

        Self {
            context,
            program,
            font,
            texture_cache,
        }
    }

    pub fn invalidate(&mut self) {
        self.texture_cache = HashMap::new();
    }

    pub fn set_texture_size(&mut self, texture_size: f32) {
        self.font
            .set_texture_size(texture_size as u32, texture_size as u32);
        self.invalidate();
    }

    pub fn set_font_size(&mut self, font_size: f32) {
        self.font.set_font_size(font_size as u8);
        self.invalidate();
    }

    pub fn set_shadow_size(&mut self, shadow_size: f32) {
        self.font.set_shadow_size(shadow_size as u8);
        self.invalidate();
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

#[derive(Copy, Clone, Debug)]
struct UITextAreaGlyphVertex {
    pos: [f32; 2],
    coord: [f32; 2],
}

implement_vertex!(UITextAreaGlyphVertex, pos, coord);

impl UITextAreaGlyphVertex {
    fn new(pos_x: f32, pos_y: f32, coord_x: f32, coord_y: f32) -> Self {
        Self {
            pos: [pos_x, pos_y],
            coord: [coord_x, coord_y],
        }
    }
}

struct UITextAreaRenderPass {
    vertex_buffer: VertexBuffer<UITextAreaGlyphVertex>,
    index_buffer: IndexBuffer<u16>,
}

pub struct UITextArea {
    style: UITextAreaStyle,
    passes: HashMap<u32, UITextAreaRenderPass>,
    context: Rc<RefCell<UITextAreaContext>>,
    last_size: UISize,
    last_text: String,
    offset: UIPoint,
    drag_offset: UIPoint,
    drag_start: Option<UIPoint>,
    zoom: f32,
    mouse_x: f32,
    mouse_y: f32,
}

impl UITextArea {
    pub fn new(
        context: Rc<RefCell<UITextAreaContext>>,
        text: &str,
        style: UITextAreaStyle,
    ) -> Self {
        Self {
            context,
            last_size: UISize::zero(),
            last_text: text.into(),
            offset: UIPoint::zero(),
            drag_offset: UIPoint::zero(),
            drag_start: None,
            zoom: 1.0,
            passes: HashMap::new(),
            style,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    pub fn get_style(&self) -> UITextAreaStyle {
        self.style
    }

    pub fn set_style(&mut self, style: UITextAreaStyle) {
        self.style = style;
    }

    pub fn set_text(&mut self, text: &str) {
        if self.last_text != text {
            self.last_text = text.into();
            self.invalidate();
        }
    }

    pub fn invalidate(&mut self) {
        let mut context = self.context.borrow_mut();

        enum FormattedText<'a> {
            End,
            NewLine,
            Word(&'a str),
        }

        struct ProcessTextCtx {
            line_y: f32,
            line_total_space: f32,
            line_word_space: f32,
            line_words: VecDeque<TextBlockLayout>,
        }

        struct PassData {
            vertices: Vec<UITextAreaGlyphVertex>,
            indices: Vec<u16>,
        }

        struct RenderWordContext {
            passes: HashMap<u32, PassData>,
        }

        let line_gap = context.font.get_line_gap();
        let ascent = context.font.get_ascent();
        let descent = context.font.get_descent();
        let line_height = line_gap + ascent - descent;
        let line_max_width = self.last_size.width / self.style.text_size;
        let line_min_space = 0.3;

        let mut render_word_ctx = RenderWordContext {
            passes: HashMap::new(),
        };

        let mut render_word = |word_layout: &TextBlockLayout, x: f32, y: f32| {
            let ctx = &mut render_word_ctx;
            for glyph_layout in &word_layout.glyph_layouts {
                let pass_data = ctx
                    .passes
                    .entry(glyph_layout.texture_id)
                    .or_insert(PassData {
                        vertices: Vec::new(),
                        indices: Vec::new(),
                    });

                let new_index = pass_data.vertices.len();
                let scr = glyph_layout.screen_coord;
                let tex = glyph_layout.texture_coord;

                let tl =
                    UITextAreaGlyphVertex::new(scr.min.x + x, scr.max.y + y, tex.min.x, tex.max.y);
                let tr =
                    UITextAreaGlyphVertex::new(scr.max.x + x, scr.max.y + y, tex.max.x, tex.max.y);
                let bl =
                    UITextAreaGlyphVertex::new(scr.min.x + x, scr.min.y + y, tex.min.x, tex.min.y);
                let br =
                    UITextAreaGlyphVertex::new(scr.max.x + x, scr.min.y + y, tex.max.x, tex.min.y);

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
        };

        let mut layout_line = |text_ctx: &mut ProcessTextCtx, align: bool| {
            let word_count = text_ctx.line_words.len();
            if word_count == 0 {
                return;
            }

            let mut line_x = 0.0;
            let space = if align {
                (line_max_width - text_ctx.line_word_space) / (word_count - 1) as f32
            } else {
                line_min_space
            };

            while let Some(word) = text_ctx.line_words.pop_front() {
                render_word(&word, line_x, text_ctx.line_y);
                line_x += word.bounding_box.width() + space;
            }

            text_ctx.line_total_space = 0.0;
            text_ctx.line_word_space = 0.0;
        };

        let mut process_text_ctx = ProcessTextCtx {
            line_y: -ascent,
            line_total_space: 0.0,
            line_word_space: 0.0,
            line_words: VecDeque::new(),
        };

        let mut process_text = |formatted_text: FormattedText| {
            let mut ctx = &mut process_text_ctx;
            match formatted_text {
                FormattedText::End => {
                    layout_line(ctx, false);
                }
                FormattedText::NewLine => {
                    layout_line(ctx, false);
                    ctx.line_y -= line_height
                }
                FormattedText::Word(word) => {
                    let word_layout = context.font.layout_text_block(word);
                    let word_width = word_layout.bounding_box.width();
                    if word_width <= line_max_width - ctx.line_total_space {
                        ctx.line_total_space += word_width + line_min_space;
                        ctx.line_word_space += word_width;
                    } else {
                        layout_line(ctx, true);
                        ctx.line_y -= line_height;
                        ctx.line_total_space = word_width + line_min_space;
                        ctx.line_word_space = word_width;
                    }
                    ctx.line_words.push_back(word_layout);
                }
            };
        };

        let mut format_text = || {
            let mut word_start = None;
            for (index, character) in self.last_text.char_indices() {
                match character {
                    '\n' => {
                        if let Some(start) = word_start {
                            process_text(FormattedText::Word(&self.last_text[start..index]));
                            word_start = None;
                        }
                        process_text(FormattedText::NewLine);
                    }
                    x if x.is_whitespace() => {
                        if let Some(start) = word_start {
                            process_text(FormattedText::Word(&self.last_text[start..index]));
                            word_start = None;
                        }
                    }
                    _ => {
                        if word_start.is_none() {
                            word_start = Some(index);
                        }
                    }
                }
            }

            if let Some(start) = word_start {
                process_text(FormattedText::Word(
                    &self.last_text[start..self.last_text.len()],
                ));
            }

            process_text(FormattedText::End);
        };

        format_text();

        let mut gl_passes = HashMap::<u32, UITextAreaRenderPass>::new();
        let gl_context = &context.context;

        for (id, pass_data) in render_word_ctx.passes {
            let vertex_buffer = VertexBuffer::immutable(gl_context, pass_data.vertices.as_slice())
                .expect("Cannot create vertex buffer for text_area");

            let index_buffer = IndexBuffer::immutable(
                gl_context,
                PrimitiveType::TrianglesList,
                pass_data.indices.as_slice(),
            )
            .expect("Cannot create index buffer for text_area");

            gl_passes.insert(
                id,
                UITextAreaRenderPass {
                    vertex_buffer,
                    index_buffer,
                },
            );
        }

        self.passes = gl_passes;
    }

    pub fn render_styled(
        &self,
        frame: &mut Frame,
        layout: UILayout,
        style: UITextAreaStyle,
        screen: UISize,
    ) {
        let pos = [
            layout.left + self.offset.left + self.drag_offset.left,
            layout.top + layout.height + self.offset.top + self.drag_offset.top,
        ];
        let screen = [screen.width, screen.height];
        let context = self.context.borrow_mut();
        let shadow_size = context.font.get_shadow_size();
        let font_size = context.font.get_font_size();
        let sharpness = self.style.sharpness
            / f32::from(shadow_size)
            / (style.text_size * self.zoom / f32::from(font_size));

        for (texture_id, pass_data) in &self.passes {
            if let Some(texture) = context.get_texture(*texture_id) {
                frame
                    .draw(
                        &pass_data.vertex_buffer,
                        &pass_data.index_buffer,
                        &context.program,
                        &uniform! {
                            uTexture: texture,
                            uInnerDist: 1.0 - style.inner_dist,
                            uOuterDist: 1.0 - style.outer_dist,
                            uSharpness: sharpness,
                            uFontSize: style.text_size * self.zoom,
                            uPosition: pos,
                            uScreen: screen,
                            uColor: style.text_color,
                            uShadowColor: style.shadow_color,
                            uShadowPos: style.shadow_pos,
                            uShadowSize: style.shadow_size,
                            uShadowAlpha: style.shadow_alpha,
                            uTextureVisibility: style.texture_visibility,
                            uMouse: [self.mouse_x, self.mouse_y],
                            uAnimation: self.style.animation
                        },
                        &DrawParameters {
                            blend: Blend::alpha_blending(),
                            color_mask: (true, true, true, false),
                            ..Default::default()
                        },
                    )
                    .expect("Cannot draw UITextArea pass");
            }
        }
    }
}

impl UIWidget for UITextArea {
    type Event = ();

    fn render(&self, frame: &mut Frame, layout: UILayout, screen: UISize) {
        self.render_styled(frame, layout, self.style, screen)
    }

    fn update_input(
        &mut self,
        layout: UILayout,
        frame_input: UIFrameInput,
        _events: &mut Vec<Self::Event>,
    ) {
        self.mouse_x = frame_input.mouse_pos.left;
        self.mouse_y = frame_input.mouse_pos.top;

        if (layout.width - self.last_size.width).abs() > f32::EPSILON
            || (layout.height - self.last_size.height).abs() > f32::EPSILON
        {
            self.last_size = UISize {
                width: layout.width,
                height: layout.height,
            };
            self.invalidate();
        }

        let left = frame_input.mouse_pos.left - layout.left;
        let top = frame_input.mouse_pos.top - layout.top - layout.height;

        if let Some(drag_start) = self.drag_start {
            if !frame_input.left_mouse_button_pressed {
                self.drag_start = None;
                self.offset = UIPoint {
                    left: self.offset.left + left - drag_start.left,
                    top: self.offset.top + top - drag_start.top,
                };
                self.drag_offset = UIPoint::zero();
            } else {
                self.drag_offset = UIPoint {
                    left: left - drag_start.left,
                    top: top - drag_start.top,
                };
            }
        } else if layout.is_inside(frame_input.mouse_pos) {
            if frame_input.left_mouse_button_pressed {
                self.drag_start = Some(UIPoint { left, top });
            }

            if let Some(mouse_wheel_delta) = frame_input.mouse_wheel_delta {
                let new_zoom = (self.zoom + mouse_wheel_delta / 100.0 * self.zoom)
                    .max(1.0 / 8.0)
                    .min(128.0);
                let new_offset_left = left - (left - self.offset.left) * (new_zoom / self.zoom);
                let new_offset_top = top - (top - self.offset.top) * (new_zoom / self.zoom);
                self.zoom = new_zoom;
                self.offset = UIPoint {
                    left: new_offset_left,
                    top: new_offset_top,
                };
            }
        }
    }
}
