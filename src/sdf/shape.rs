use super::geometry::{Curve, Line, Rect, SignedDistance};
use super::texture::{LockedTexture, PixelView, TextureView, TextureViewAllocator};
use super::utils::{clamp_f32, median, median_f32};
use cgmath::Point2;
use std::cell::RefCell;
use std::f32;

#[derive(Debug, Clone, Copy)]
pub enum SegmentPrimitive {
    Line { line: Line, mask: u8 },
    Curve { curve: Curve, mask: u8 },
    End { clock_wise: bool },
}

impl SegmentPrimitive {
    pub fn bounding_box(&self) -> Option<Rect<f32>> {
        match self {
            SegmentPrimitive::Line { line, .. } => Some(line.bounding_box()),
            SegmentPrimitive::Curve { curve, .. } => Some(curve.bounding_box()),
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
        locked_texture.modify_view(&mut *texture_view, |pixel_view| {
            let pixel = Point2::new(
                self.segments_bb.min.x + pixel_view.x as f32,
                self.segments_bb.min.y + (pixel_view.height - 1 - pixel_view.y) as f32,
            );

            let (rd, bd, gd) = self.render_pixel(pixel);
            let mut current_pixel = [(rd * 255.0) as u8, (gd * 255.0) as u8, (bd * 255.0) as u8];

            if self.is_pixel_clashing(pixel_view, current_pixel) {
                let m = median(current_pixel);
                current_pixel[0] = m;
                current_pixel[1] = m;
                current_pixel[2] = m;
            }

            current_pixel
        });
    }

    fn render_pixel(&self, pixel: Point2<f32>) -> (f32, f32, f32) {
        const MAX: [f32; 3] = [f32::MAX, f32::MAX, f32::MAX];
        const ZERO: [f32; 3] = [0.0, 0.0, 0.0];

        let mut distance = MAX;
        let mut pseudo_distance = MAX;
        let mut final_distance = MAX;
        let mut orthogonality = ZERO;
        let mut segment_count = 0;
        let mut current_mask = 0;

        for p in &self.segments {
            let sd = match p {
                SegmentPrimitive::Line { line, mask } => {
                    current_mask = *mask;
                    Some(line.signed_distance(pixel))
                }
                SegmentPrimitive::Curve { curve, mask } => {
                    current_mask = *mask;
                    Some(curve.signed_distance(pixel))
                }
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
                    if (1 << i) & current_mask == 0 {
                        continue;
                    }

                    if !self.is_closer_to_segment(&sd, distance[i], orthogonality[i]) {
                        continue;
                    }

                    distance[i] = sd.real_dist;
                    orthogonality[i] = sd.orthogonality;
                    pseudo_distance[i] = -sd.sign * sd.extended_dist;
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

    fn is_pixel_clashing(&self, pixel_view: PixelView, current_pixel: [u8; 3]) -> bool {
        if pixel_view.x == pixel_view.width - 1 || pixel_view.y == pixel_view.height - 1 {
            return true;
        }
        self.is_pixel_pair_clashing(pixel_view.top_pixel, current_pixel)
            || self.is_pixel_pair_clashing(pixel_view.left_pixel, current_pixel)
            || self.is_pixel_pair_clashing(pixel_view.top_left_pixel, current_pixel)
            || self.is_pixel_pair_clashing(pixel_view.top_right_pixel, current_pixel)
    }

    fn is_pixel_pair_clashing(&self, p1: [u8; 3], p2: [u8; 3]) -> bool {
        const INSIDE_THRESHOLD: u8 = 127;
        const CLASHING_TRESHOLD: i16 = 16;

        let p1_bits = (p1[0] / INSIDE_THRESHOLD) << 0
            | (p1[1] / INSIDE_THRESHOLD) << 1
            | (p1[2] / INSIDE_THRESHOLD) << 2;

        let p2_bits = (p2[0] / INSIDE_THRESHOLD) << 0
            | (p2[1] / INSIDE_THRESHOLD) << 1
            | (p2[2] / INSIDE_THRESHOLD) << 2;

        if p1_bits == 0b000 || p1_bits == 0b111 || p2_bits == 0b000 || p2_bits == 0b111 {
            return false;
        }

        let xor_bits = p1_bits ^ p2_bits;
        if xor_bits.count_ones() != 2 {
            return false;
        }

        let mut clashing = true;
        for i in 0..3 {
            if 1 << i & xor_bits != 0 && (p1[i] as i16 - p2[i] as i16).abs() < CLASHING_TRESHOLD {
                clashing = false;
            }
        }

        clashing
    }
}
