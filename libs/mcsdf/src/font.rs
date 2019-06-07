use super::geometry::{Curve, Line, Rect};
use super::shape::{AllocatedShape, Segment, Shape};
use super::texture::{Texture, TextureViewAllocator};
use cgmath::Point2;
use rusttype::{Contour, Scale, Segment as FontSegment};
use rusttype::{Error as RustTypeError, Font as RustTypeFont};
use std::collections::HashMap;
use std::iter::{once, FromIterator};
use std::mem::replace;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum FontError {
    CannotLoadFont,
}

impl From<RustTypeError> for FontError {
    fn from(_error: RustTypeError) -> Self {
        FontError::CannotLoadFont
    }
}

struct GlyphInfo {
    texture_id: u32,
    texture_view: Rect<f32>,
}

pub struct GlyphLayout {
    pub texture_id: u32,
    pub screen_coord: Rect<f32>,
    pub texture_coord: Rect<f32>,
}

pub struct TextBlockLayout {
    pub font_size: u8,
    pub shadow_size: u8,
    pub bounding_box: Rect<f32>,
    pub glyph_layouts: Vec<GlyphLayout>,
}

pub struct TextureRenderBatch {
    pub texture_id: u32,
    pub texture: Arc<Mutex<Texture>>,
    pub allocated_shapes: Vec<AllocatedShape>,
}

struct TextureMetadata {
    texture: Arc<Mutex<Texture>>,
    allocator: TextureViewAllocator,
    allocated_shapes: Vec<AllocatedShape>,
}

pub struct Font {
    texture_metadatas: Vec<TextureMetadata>,
    free_texture_index: u32,
    texture_width: u32,
    texture_height: u32,
    font_size: u8,
    shadow_size: u8,
    font: RustTypeFont<'static>,
    glyphs: HashMap<char, Option<GlyphInfo>>,
}

impl Font {
    pub fn new(
        texture_width: u32,
        texture_height: u32,
        font_size: u8,
        shadow_size: u8,
        font_data: Vec<u8>,
    ) -> Result<Self, FontError> {
        let font = RustTypeFont::from_bytes(font_data)?;
        let (texture, allocator) = Texture::new(texture_width, texture_height);
        let texture_metadatas = vec![TextureMetadata {
            texture: Arc::new(Mutex::new(texture)),
            allocator,
            allocated_shapes: Vec::new(),
        }];

        Ok(Font {
            texture_metadatas,
            free_texture_index: 0,
            texture_width,
            texture_height,
            font_size,
            shadow_size,
            font,
            glyphs: HashMap::new(),
        })
    }

    pub fn invalidate(&mut self) {
        let (texture, allocator) = Texture::new(self.texture_width, self.texture_height);
        let texture_metadatas = vec![TextureMetadata {
            texture: Arc::new(Mutex::new(texture)),
            allocator,
            allocated_shapes: Vec::new(),
        }];

        self.texture_metadatas = texture_metadatas;
        self.free_texture_index = 0;
        self.glyphs = HashMap::new();
    }

    pub fn allocate_glyph(&mut self, c: char) {
        if self.glyphs.contains_key(&c) {
            return;
        }

        let glyph = self.font.glyph(c);
        let allocated_shape =
            if let Some(shape) = glyph.scaled(Scale::uniform(self.font_size as f32)).shape() {
                loop {
                    let allocated_shape = {
                        let texture_allocator =
                            &mut self.texture_metadatas[self.free_texture_index as usize].allocator;
                        AllocatedShape::new(
                            shape.as_slice().into(),
                            texture_allocator,
                            self.shadow_size as f32,
                        )
                    };

                    if let Some(s) = allocated_shape {
                        break Some(s);
                    } else {
                        let (texture, allocator) =
                            Texture::new(self.texture_width, self.texture_height);

                        self.texture_metadatas.push(TextureMetadata {
                            texture: Arc::new(Mutex::new(texture)),
                            allocated_shapes: Vec::new(),
                            allocator,
                        });

                        self.free_texture_index += 1;
                    }
                }
            } else {
                None
            };

        let glyph_info = allocated_shape.map(|allocated_shape| {
            let texture_view = allocated_shape.texture_view.get_view();
            let texture_id = self.free_texture_index;

            self.texture_metadatas[texture_id as usize]
                .allocated_shapes
                .push(allocated_shape);

            GlyphInfo {
                texture_id,
                texture_view: Rect::new(
                    texture_view.min.x as f32 / self.texture_width as f32,
                    texture_view.min.y as f32 / self.texture_height as f32,
                    texture_view.max.x as f32 / self.texture_width as f32,
                    texture_view.max.y as f32 / self.texture_height as f32,
                ),
            }
        });

        self.glyphs.insert(c, glyph_info);
    }

    pub fn allocate_glyphs(&mut self, text: &str) {
        text.chars().for_each(|c| self.allocate_glyph(c));
    }

    pub fn get_texture(&self, texture_id: u32) -> Arc<Mutex<Texture>> {
        self.texture_metadatas[texture_id as usize].texture.clone()
    }

    pub fn get_texture_width(&self) -> u32 {
        self.texture_width
    }

    pub fn get_texture_height(&self) -> u32 {
        self.texture_height
    }

    pub fn set_texture_size(&mut self, width: u32, height: u32) {
        self.texture_width = width;
        self.texture_height = height;
        self.invalidate();
    }

    pub fn get_shadow_size(&self) -> u8 {
        self.shadow_size
    }

    pub fn set_shadow_size(&mut self, shadow_size: u8) {
        self.shadow_size = shadow_size;
        self.invalidate();
    }

    pub fn get_font_size(&self) -> u8 {
        self.font_size
    }

    pub fn set_font_size(&mut self, font_size: u8) {
        self.font_size = font_size;
        self.invalidate();
    }

    pub fn get_ascent(&self) -> f32 {
        let scale = Scale::uniform(1.0);
        let v_metrics = self.font.v_metrics(scale);
        v_metrics.ascent
    }

    pub fn get_descent(&self) -> f32 {
        let scale = Scale::uniform(1.0);
        let v_metrics = self.font.v_metrics(scale);
        v_metrics.descent
    }

    pub fn get_line_gap(&self) -> f32 {
        let scale = Scale::uniform(1.0);
        let v_metrics = self.font.v_metrics(scale);
        v_metrics.line_gap
    }

    pub fn get_texture_render_batches(&mut self) -> Vec<TextureRenderBatch> {
        let mut batches = Vec::new();

        for (texture_id, texture_metadata) in self.texture_metadatas.iter_mut().enumerate() {
            if !texture_metadata.allocated_shapes.is_empty() {
                let allocated_shapes = replace(&mut texture_metadata.allocated_shapes, Vec::new());

                batches.push(TextureRenderBatch {
                    texture_id: texture_id as u32,
                    texture: texture_metadata.texture.clone(),
                    allocated_shapes,
                })
            }
        }

        batches
    }

    pub fn layout_text_block(&mut self, text: &str) -> TextBlockLayout {
        self.allocate_glyphs(text);

        let mut glyph_layouts = Vec::new();

        let mut bb_min_x = 0.0;
        let mut bb_min_y = 0.0;
        let mut bb_max_x = 0.0;
        let mut bb_max_y = 0.0;

        let shadow = self.shadow_size as f32 / self.font_size as f32;
        let scale = Scale::uniform(1.0);
        let v_metrics = self.font.v_metrics(scale);

        let mut last_glyph = None;
        let mut offset_x = 0.0;
        let mut offset_y = 0.0;

        for c in text.chars() {
            if c == '\n' {
                offset_x = 0.0;
                last_glyph = None;
                offset_y -= v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
                continue;
            }

            let glyph = self.font.glyph(c).scaled(scale);
            let glyph_info = self.glyphs.get(&c).unwrap();

            if let Some(last_glyph) = last_glyph {
                offset_x += self.font.pair_kerning(scale, last_glyph, glyph.id());
            }

            let advance_width = glyph.h_metrics().advance_width;

            if let Some(bb) = glyph.exact_bounding_box() {
                let min_x = offset_x + bb.min.x;
                let min_y = offset_y - bb.max.y;
                let max_x = offset_x + bb.max.x;
                let max_y = offset_y - bb.min.y;

                bb_min_x = min_x.min(bb_min_x);
                bb_min_y = min_y.min(bb_min_y);
                bb_max_x = max_x.max(bb_max_x);
                bb_max_y = max_y.max(bb_max_y);

                if let Some(glyph_info) = glyph_info {
                    glyph_layouts.push(GlyphLayout {
                        texture_id: glyph_info.texture_id,
                        screen_coord: Rect::new(
                            min_x - shadow,
                            min_y - shadow,
                            max_x + shadow,
                            max_y + shadow,
                        ),
                        texture_coord: glyph_info.texture_view,
                    });
                }
            }

            offset_x += advance_width;
            last_glyph = Some(glyph.id());
        }

        TextBlockLayout {
            font_size: self.font_size,
            shadow_size: self.shadow_size,
            bounding_box: Rect::new(bb_min_x, bb_min_y, bb_max_x, bb_max_y),
            glyph_layouts,
        }
    }
}

impl<'a> From<&'a [Contour]> for Shape {
    fn from(contours: &'a [Contour]) -> Shape {
        let segments = contours.iter().flat_map(|contour| {
            once(Segment::Start {
                count: contour.segments.len(),
            })
            .chain(contour.segments.iter().map(|segment| match segment {
                FontSegment::Line(line) => Segment::Line {
                    line: Line {
                        p0: Point2::new(line.p[0].x, line.p[0].y),
                        p1: Point2::new(line.p[1].x, line.p[1].y),
                    },
                },
                FontSegment::Curve(curve) => Segment::Curve {
                    curve: Curve {
                        p0: Point2::new(curve.p[0].x, curve.p[0].y),
                        p1: Point2::new(curve.p[1].x, curve.p[1].y),
                        p2: Point2::new(curve.p[2].x, curve.p[2].y),
                    },
                },
            }))
        });

        Shape::from_iter(segments)
    }
}
