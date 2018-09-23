use super::geometry::{Curve, Line, Rect};
use super::shape::{AllocatedShape, Segment, Shape};
use super::texture::{Texture, TextureViewAllocator};
use cgmath::Point2;
use rusttype::{Contour, Scale, Segment as FontSegment};
use rusttype::{Error as FontError, Font};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;

pub enum SDFFontError {
    CannotOpenFont(io::Error),
    CannotLoadFont,
}

impl From<io::Error> for SDFFontError {
    fn from(error: io::Error) -> Self {
        SDFFontError::CannotOpenFont(error)
    }
}

impl From<FontError> for SDFFontError {
    fn from(_error: FontError) -> Self {
        SDFFontError::CannotLoadFont
    }
}

struct GlyphInfo {
    texture_id: u32,
    texture_view: Rect<u32>,
}

pub struct SDFFont {
    textures: Vec<(Texture, TextureViewAllocator)>,
    free_texture_index: u32,
    texture_width: u32,
    texture_height: u32,
    font_size: u8,
    shadow_size: u8,
    font: Font<'static>,
    glyphs: HashMap<char, Option<GlyphInfo>>,
}

impl SDFFont {
    pub fn new(
        texture_width: u32,
        texture_height: u32,
        font_size: u8,
        shadow_size: u8,
        font_path: &str,
    ) -> Result<Self, SDFFontError> {
        let mut font_data = Vec::<u8>::new();
        File::open(font_path)?.read_to_end(&mut font_data)?;
        let font = Font::from_bytes(font_data)?;

        Ok(SDFFont {
            textures: vec![Texture::new(texture_width, texture_height)],
            free_texture_index: 0,
            texture_width,
            texture_height,
            font_size,
            shadow_size,
            font,
            glyphs: HashMap::new(),
        })
    }

    pub fn allocate_glyph(&mut self, c: char) -> Option<AllocatedShape> {
        if self.glyphs.contains_key(&c) {
            return None;
        }

        let glyph = self.font.glyph(c);
        let allocated_shape = if let Some(shape) =
            glyph.scaled(Scale::uniform(self.font_size as f32)).shape()
        {
            loop {
                let allocated_shape = {
                    let texture_allocator = &mut self.textures[self.free_texture_index as usize].1;
                    AllocatedShape::new(
                        shape.as_slice().into(),
                        texture_allocator,
                        self.shadow_size as f32,
                    )
                };

                if let Some(s) = allocated_shape {
                    break Some(s);
                } else {
                    self.textures
                        .push(Texture::new(self.texture_width, self.texture_height));
                    self.free_texture_index += 1;
                }
            }
        } else {
            None
        };

        let glyph_info = allocated_shape.as_ref().map(|s| GlyphInfo {
            texture_id: self.free_texture_index,
            texture_view: s.texture_view.get_view(),
        });

        self.glyphs.insert(c, glyph_info);

        allocated_shape
    }

    pub fn allocate_glyphs(&mut self, text: &str) -> Vec<AllocatedShape> {
        text.chars()
            .filter_map(|c| self.allocate_glyph(c))
            .collect()
    }
}

impl<'a> From<&'a [Contour]> for Shape {
    fn from(contours: &'a [Contour]) -> Shape {
        let mut segments = Vec::new();

        for contour in contours {
            let segments_count = contour.segments.len();
            let mut area = 0.0;
            let mut mask = 0;

            for (index, segment) in contour.segments.iter().enumerate() {
                mask = match mask {
                    0b110 => 0b011,
                    0b011 => 0b101,
                    _ => if index + 1 >= segments_count {
                        0b011
                    } else {
                        0b110
                    },
                };

                match segment {
                    FontSegment::Line(line) => {
                        let line = Line {
                            p0: Point2::new(line.p[0].x, line.p[0].y),
                            p1: Point2::new(line.p[1].x, line.p[1].y),
                        };
                        area += line.area();
                        segments.push(Segment::Line { line, mask });
                    }
                    FontSegment::Curve(curve) => {
                        let curve = Curve {
                            p0: Point2::new(curve.p[0].x, curve.p[0].y),
                            p1: Point2::new(curve.p[1].x, curve.p[1].y),
                            p2: Point2::new(curve.p[2].x, curve.p[2].y),
                        };
                        area += curve.area();
                        segments.push(Segment::Curve { curve, mask });
                    }
                }
            }
            segments.push(Segment::End {
                clock_wise: area < 0.0,
            });
        }

        Shape::new(segments)
    }
}
