use super::geometry::SignedDistance;
use super::shape::{AllocatedShape, Shape, ShapeSegment};
use super::texture::{LockedTexture, PixelView};
use super::utils::{clamp_f32, max, median, median_f32, min};
use cgmath::Point2;
use std::f32;

pub fn render_shape(allocated_shape: &mut AllocatedShape, locked_texture: &LockedTexture) {
    let bb = allocated_shape.shape_bb;
    let shape = &allocated_shape.shape;
    let max_distance = allocated_shape.max_distance;
    let mut texture_view = &mut allocated_shape.texture_view;

    locked_texture.modify_view(&mut texture_view, |pixel_view| {
        let pixel = Point2::new(
            bb.min.x + pixel_view.x as f32,
            // bb.min.y + (pixel_view.height - 1 - pixel_view.y) as f32,
            bb.min.y + pixel_view.y as f32,
        );

        let (rd, bd, gd) = render_shape_pixel(shape, max_distance, pixel);
        let mut current_pixel = [(rd * 255.0) as u8, (gd * 255.0) as u8, (bd * 255.0) as u8];

        if is_pixel_clashing(max_distance, pixel_view, current_pixel) {
            let m = median(current_pixel);
            current_pixel[0] = m;
            current_pixel[1] = m;
            current_pixel[2] = m;
        }

        current_pixel
    });
}

fn render_shape_pixel(shape: &Shape, max_distance: f32, pixel: Point2<f32>) -> (f32, f32, f32) {
    const MAX: [f32; 3] = [f32::MAX, f32::MAX, f32::MAX];
    const ZERO: [f32; 3] = [0.0, 0.0, 0.0];

    let mut distance = MAX;
    let mut pseudo_distance = MAX;
    let mut final_distance = MAX;
    let mut orthogonality = ZERO;
    let mut segment_count = 0;
    let mut current_mask = 0;

    for p in shape.get_segments() {
        let sd = match p {
            ShapeSegment::Line { line, mask } => {
                current_mask = *mask;
                Some(line.signed_distance(pixel))
            }
            ShapeSegment::Curve { curve, mask } => {
                current_mask = *mask;
                Some(curve.signed_distance(pixel))
            }
            ShapeSegment::End { clock_wise } => {
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

                if !is_closer_to_segment(&sd, distance[i], orthogonality[i]) {
                    continue;
                }

                distance[i] = sd.real_dist;
                orthogonality[i] = sd.orthogonality;

                const START_THRESHOLD: f32 = 0.3;
                const END_THRESHOLD: f32 = 0.5;

                let mut rd = (sd.real_dist / max_distance - START_THRESHOLD) / END_THRESHOLD;
                rd = clamp_f32(rd, 0.0, 1.0);

                pseudo_distance[i] = -sd.sign * ((1.0 - rd) * sd.extended_dist + rd * sd.real_dist);
            }
        }
    }

    (
        clamp_f32(final_distance[0] / max_distance, -1.0, 1.0) * 0.5 + 0.5,
        clamp_f32(final_distance[1] / max_distance, -1.0, 1.0) * 0.5 + 0.5,
        clamp_f32(final_distance[2] / max_distance, -1.0, 1.0) * 0.5 + 0.5,
    )
}

fn is_closer_to_segment(sd: &SignedDistance, distance: f32, orthogonality: f32) -> bool {
    if (sd.real_dist - distance).abs() <= 0.01 {
        sd.orthogonality > orthogonality
    } else {
        sd.real_dist < distance
    }
}

fn is_pixel_clashing(max_distance: f32, pixel_view: PixelView, current_pixel: [u8; 3]) -> bool {
    if pixel_view.x == pixel_view.width - 1 || pixel_view.y == pixel_view.height - 1 {
        return true;
    }

    let clashing_threshold = (128.0 / max_distance) as i16 + 1;

    is_pixel_pair_clashing(clashing_threshold, pixel_view.top_pixel, current_pixel)
        || is_pixel_pair_clashing(clashing_threshold, pixel_view.left_pixel, current_pixel)
        || is_pixel_pair_clashing(clashing_threshold, pixel_view.top_left_pixel, current_pixel)
        || is_pixel_pair_clashing(
            clashing_threshold,
            pixel_view.top_right_pixel,
            current_pixel,
        )
}

fn is_pixel_pair_clashing(clashing_threshold: i16, p1: [u8; 3], p2: [u8; 3]) -> bool {
    let p1_min = min(p1);
    let p1_threshold = (max(p1) - p1_min) / 2 + 1;

    let p1_bits = (p1[0] - p1_min) / p1_threshold << 0
        | (p1[1] - p1_min) / p1_threshold << 1
        | (p1[2] - p1_min) / p1_threshold << 2;

    let p2_min = min(p2);
    let p2_threshold = (max(p2) - p2_min) / 2 + 1;

    let p2_bits = (p2[0] - p2_min) / p2_threshold << 0
        | (p2[1] - p2_min) / p2_threshold << 1
        | (p2[2] - p2_min) / p2_threshold << 2;

    if p1_bits == 0b000 || p1_bits == 0b111 || p2_bits == 0b000 || p2_bits == 0b111 {
        return false;
    }

    let xor_bits = p1_bits ^ p2_bits;
    if xor_bits.count_ones() != 2 {
        return false;
    }

    let mut clashing = true;
    for i in 0..3 {
        if 1 << i & xor_bits != 0 && (p1[i] as i16 - p2[i] as i16).abs() < clashing_threshold {
            clashing = false;
        }
    }

    clashing
}
