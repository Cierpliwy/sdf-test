use super::geometry::{Curve, Line, Rect, SignedDistance};
use super::texture::{Color, LockedTexture, TextureView, TextureViewAllocator};
use super::utils::{clamp_f32, median, median_f32};
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
        locked_texture.modify_view(&mut *texture_view, |x, y, _, h| {
            let pixel = Point2::new(
                self.segments_bb.min.x + x as f32,
                self.segments_bb.min.y + (h - 1 - y) as f32,
            );

            let (rd, bd, gd) = self.render_pixel(pixel);

            Color {
                r: (rd * 255.0) as u8,
                g: (gd * 255.0) as u8,
                b: (bd * 255.0) as u8,
            }
        });

        locked_texture.correct_view(&mut *texture_view, |_, _, _, _, left, top, current| {
            fn color_clashing(c1: Color, c2: Color) -> bool {
                if c1.is_black() || c2.is_black() {
                    return false;
                }

                let c1r = (c1.r >= 128) as u8;
                let c1g = (c1.g >= 128) as u8;
                let c1b = (c1.b >= 128) as u8;

                let c2r = (c2.r >= 128) as u8;
                let c2g = (c2.g >= 128) as u8;
                let c2b = (c2.b >= 128) as u8;

                let c1sum = c1r + c1g + c1b;
                let c2sum = c2r + c2g + c2b;

                let c1_inside = c1sum >= 2;
                let c2_inside = c2sum >= 2;
                if c1_inside != c2_inside {
                    return false;
                }

                if c1sum == 0 || c1sum == 3 || c2sum == 0 || c2sum == 3 {
                    return false;
                }

                let d1a;
                let d1b;
                let d2a;
                let d2b;

                if c1r == c2r {
                    if c1b == c2b && c1g == c2g {
                        return false;
                    }
                    d1b = c1.g as i16;
                    d2b = c2.g as i16;
                    d1a = c1.b as i16;
                    d2a = c2.b as i16;
                } else {
                    if c1g == c2g {
                        d1b = c1.r as i16;
                        d2b = c2.r as i16;
                        d1a = c1.b as i16;
                        d2a = c2.b as i16;
                    } else {
                        d1b = c1.g as i16;
                        d2b = c2.g as i16;
                        d1a = c1.r as i16;
                        d2a = c2.r as i16;
                    }
                }
                (d1b - d2b).abs() > 10 && (d1a - d2a).abs() > 2
            }

            if color_clashing(left, current) || color_clashing(top, current) {
                let m = median([current.r, current.g, current.b]);
                Color { r: m, g: m, b: m }
            } else {
                current
            }
        });
    }

    fn render_pixel(&self, pixel: Point2<f32>) -> (f32, f32, f32) {
        const MAX: [f32; 3] = [f32::MAX, f32::MAX, f32::MAX];
        const ZERO: [f32; 3] = [0.0, 0.0, 0.0];

        let mut mask = 0b101;
        let mut distance = MAX;
        let mut pseudo_distance = MAX;
        let mut final_distance = MAX;
        let mut orthogonality = ZERO;
        let mut segment_count = 0;

        for p in &self.segments {
            let sd = match p {
                SegmentPrimitive::Line(line) => Some(line.signed_distance(pixel)),
                SegmentPrimitive::Curve(curve) => Some(curve.signed_distance(pixel)),
                SegmentPrimitive::End { clock_wise } => {
                    distance = MAX;
                    orthogonality = ZERO;
                    if segment_count == 0 {
                        final_distance = pseudo_distance;
                    }

                    let pseudo_median = median_f32(pseudo_distance);
                    let final_median = median_f32(final_distance);

                    if (pseudo_median > final_median) ^ !*clock_wise {
                        final_distance = pseudo_distance;
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

                mask = match mask {
                    0b101 => 0b011,
                    0b011 => 0b110,
                    _ => 0b101,
                }
            }
        }

        (
            clamp_f32(final_distance[0] / self.max_distance, -1.0, 1.0) * 0.5 + 0.5,
            clamp_f32(final_distance[1] / self.max_distance, -1.0, 1.0) * 0.5 + 0.5,
            clamp_f32(final_distance[2] / self.max_distance, -1.0, 1.0) * 0.5 + 0.5,
        )
    }

    fn is_closer_to_segment(&self, sd: &SignedDistance, distance: f32, orthogonality: f32) -> bool {
        if (sd.real_dist - distance).abs() <= 0.01 {
            sd.orthogonality > orthogonality
        } else {
            sd.real_dist < distance
        }
    }
}
