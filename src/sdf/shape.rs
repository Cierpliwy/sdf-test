use super::geometry::{Curve, Line, Rect};
use super::texture::{TextureView, TextureViewAllocator};
use std::f32;
use std::iter::FromIterator;

pub struct Shape {
    segments: Vec<ShapeSegment>,
}

impl Shape {
    pub fn new(segments: Vec<ShapeSegment>) -> Self {
        Self { segments }
    }

    pub fn get_segments(&self) -> &[ShapeSegment] {
        &self.segments
    }
}

#[derive(Clone, Copy)]
pub enum ShapeSegment {
    Line { line: Line, mask: u8 },
    Curve { curve: Curve, mask: u8 },
    End { clock_wise: bool },
}

impl ShapeSegment {
    pub fn bounding_box(&self) -> Option<Rect<f32>> {
        match self {
            ShapeSegment::Line { line, .. } => Some(line.bounding_box()),
            ShapeSegment::Curve { curve, .. } => Some(curve.bounding_box()),
            ShapeSegment::End { .. } => None,
        }
    }
}

pub struct AllocatedShape {
    pub shape: Shape,
    pub shape_bb: Rect<f32>,
    pub texture_view: TextureView,
    pub max_distance: f32,
}

impl AllocatedShape {
    pub fn new(
        shape: Shape,
        texture_allocator: &mut TextureViewAllocator,
        max_distance: f32,
    ) -> Option<Self> {
        let mut max_bb: Option<Rect<f32>> = None;
        for segment in &shape.segments {
            if let Some(bb) = segment.bounding_box() {
                if let Some(ref mut max_bb) = max_bb {
                    max_bb.min.x = max_bb.min.x.min(bb.min.x);
                    max_bb.min.y = max_bb.min.y.min(bb.min.y);
                    max_bb.max.x = max_bb.max.x.max(bb.max.x);
                    max_bb.max.y = max_bb.max.y.max(bb.max.y);
                } else {
                    max_bb = Some(bb);
                }
            }
        }

        let mut max_bb = max_bb?;
        max_bb.min.x -= max_distance;
        max_bb.min.y -= max_distance;
        max_bb.max.x += max_distance;
        max_bb.max.y += max_distance;

        let texture_view = texture_allocator
            .allocate(max_bb.width().ceil() as u32, max_bb.height().ceil() as u32)?;

        Some(Self {
            shape,
            shape_bb: max_bb,
            texture_view,
            max_distance,
        })
    }
}

pub enum Segment {
    Start { count: usize },
    Line { line: Line },
    Curve { curve: Curve },
}

impl<'a> FromIterator<Segment> for Shape {
    fn from_iter<T: IntoIterator<Item = Segment>>(segments: T) -> Self {
        let mut shape_segments = Vec::new();
        let mut area = 0.0;
        let mut mask = 0;
        let mut remaining_segments = 0;

        fn next_mask(mask: u8, remaining_segments: usize) -> u8 {
            match mask {
                0b110 => 0b011,
                0b011 => 0b101,
                _ => if remaining_segments == 0 {
                    0b011
                } else {
                    0b110
                },
            }
        };

        let mut iter = segments.into_iter();
        while let Some(segment) = iter.next() {
            match segment {
                Segment::Start { count } => {
                    remaining_segments = count;
                    mask = 0;
                    area = 0.0;
                }
                Segment::Line { line } => {
                    area += line.area();
                    remaining_segments -= 1;
                    mask = next_mask(mask, remaining_segments);
                    shape_segments.push(ShapeSegment::Line { line, mask });
                }
                Segment::Curve { curve } => {
                    area += curve.area();
                    remaining_segments -= 1;
                    mask = next_mask(mask, remaining_segments);
                    shape_segments.push(ShapeSegment::Curve { curve, mask });
                }
            }

            if remaining_segments == 0 {
                shape_segments.push(ShapeSegment::End {
                    clock_wise: area < 0.0,
                });
            }
        }

        Shape::new(shape_segments)
    }
}
