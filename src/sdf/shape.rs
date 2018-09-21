use super::geometry::{Curve, Line, Rect, SignedDistance};
use super::texture::{LockedTexture, TextureView, TextureViewAllocator};
use cgmath::Point2;
use std::cell::RefCell;
use std::f32;

#[derive(Debug, Clone, Copy)]
pub enum SegmentPrimitive {
    Line(Line),
    Curve(Curve),
    End { clock_wise: bool },
}

impl SegmentPrimitive {
    pub fn bounding_box(&self) -> Option<Rect<f32>> {
        match self {
            SegmentPrimitive::Line(line) => Some(line.bounding_box()),
            SegmentPrimitive::Curve(curve) => Some(curve.bounding_box()),
            SegmentPrimitive::End { .. } => None,
        }
    }
}

pub struct Shape {
    segments: Vec<SegmentPrimitive>,
    segments_bb: Rect<f32>,
    texture_view: RefCell<TextureView>,
    max_distance: f32,
}

impl Shape {
    pub fn new(
        segments: Vec<SegmentPrimitive>,
        texture_allocator: &mut TextureViewAllocator,
        max_distance: f32,
    ) -> Option<Self> {
        let mut max_bb: Option<Rect<f32>> = None;
        for segment in &segments {
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

        Some(Shape {
            segments,
            segments_bb: max_bb,
            texture_view: RefCell::new(texture_view),
            max_distance,
        })
    }

    pub fn render(&mut self, locked_texture: &LockedTexture) {
        let mut texture_view = self.texture_view.borrow_mut();
        locked_texture.modify_view(&mut *texture_view, |x, y, _, _| {
            let pixel = Point2::new(
                self.segments_bb.min.x + x as f32,
                self.segments_bb.min.y + y as f32,
            );

            let (rd, bd, gd) = self.render_pixel(pixel);

            ((rd * 255.0) as u8, (gd * 255.0) as u8, (bd * 255.0) as u8)
        });
    }

    fn render_pixel(&self, pixel: Point2<f32>) -> (f32, f32, f32) {
        let mut mask = 0b101;
        let mut distance = [f32::MAX, f32::MAX, f32::MAX];
        let mut orthogonality = [0.0, 0.0, 0.0];
        let mut pseudo_distance = [f32::MAX, f32::MAX, f32::MAX];
        let mut final_distance = [f32::MAX, f32::MAX, f32::MAX];
        let mut segment_count = 0;

        for p in &self.segments {
            let sd = match p {
                SegmentPrimitive::Line(line) => Some(line.signed_distance(pixel)),
                SegmentPrimitive::Curve(curve) => Some(curve.signed_distance(pixel)),
                SegmentPrimitive::End { clock_wise } => {
                    for i in 0..3 {
                        if (1 << i) & mask == 0 {
                            continue;
                        }

                        distance[i] = f32::MAX;
                        orthogonality[i] = 0.0;
                        if segment_count == 0 {
                            final_distance[i] = pseudo_distance[i];
                            continue;
                        }

                        if *clock_wise {
                            final_distance[i] = pseudo_distance[i].max(final_distance[i]);
                        } else {
                            final_distance[i] = pseudo_distance[i].min(final_distance[i]);
                        }
                    }
                    segment_count += 1;
                    None
                }
            };

            if let Some(sd) = sd {
                for i in 0..3 {
                    if (1 << i) & mask == 0 {
                        continue;
                    }

                    if !self.is_closer_to_segment(&sd, distance[i], orthogonality[i]) {
                        continue;
                    }

                    distance[i] = sd.real_dist;
                    orthogonality[i] = sd.orthogonality;
                    pseudo_distance[i] = -sd.sign * sd.extended_dist;
                }

                // mask = match mask {
                //     0b101 => 0b011,
                //     0b011 => 0b110,
                //     _ => 0b101,
                // }
            }
        }

        (
            (final_distance[0] / self.max_distance).max(-1.0).min(1.0) * 0.5 + 0.5,
            (final_distance[1] / self.max_distance).max(-1.0).min(1.0) * 0.5 + 0.5,
            (final_distance[2] / self.max_distance).max(-1.0).min(1.0) * 0.5 + 0.5,
        )
    }

    fn is_closer_to_segment(&self, sd: &SignedDistance, distance: f32, orthogonality: f32) -> bool {
        if (sd.real_dist - distance).abs() <= 0.01 {
            if sd.orthogonality > orthogonality {
                true
            } else {
                false
            }
        } else if sd.real_dist < distance {
            true
        } else {
            false
        }
    }
}
