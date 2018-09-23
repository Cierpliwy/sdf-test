use super::geometry::{Curve, Line, Rect};
use super::texture::{TextureView, TextureViewAllocator};
use std::f32;

pub struct Shape {
    segments: Vec<Segment>,
}

impl Shape {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }

    pub fn get_segments(&self) -> &[Segment] {
        &self.segments
    }
}

#[derive(Clone, Copy)]
pub enum Segment {
    Line { line: Line, mask: u8 },
    Curve { curve: Curve, mask: u8 },
    End { clock_wise: bool },
}

impl Segment {
    pub fn bounding_box(&self) -> Option<Rect<f32>> {
        match self {
            Segment::Line { line, .. } => Some(line.bounding_box()),
            Segment::Curve { curve, .. } => Some(curve.bounding_box()),
            Segment::End { .. } => None,
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
