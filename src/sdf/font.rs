use super::geometry::{Curve, Line};
use super::shape::SegmentPrimitive;
use cgmath::Point2;
use rusttype::{Contour, Segment};

pub fn create_segments_from_glyph_contours(contours: &Vec<Contour>) -> Vec<SegmentPrimitive> {
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
                Segment::Line(line) => {
                    let line = Line {
                        p0: Point2::new(line.p[0].x, line.p[0].y),
                        p1: Point2::new(line.p[1].x, line.p[1].y),
                    };
                    area += line.area();
                    segments.push(SegmentPrimitive::Line { line, mask });
                }
                Segment::Curve(curve) => {
                    let curve = Curve {
                        p0: Point2::new(curve.p[0].x, curve.p[0].y),
                        p1: Point2::new(curve.p[1].x, curve.p[1].y),
                        p2: Point2::new(curve.p[2].x, curve.p[2].y),
                    };
                    area += curve.area();
                    segments.push(SegmentPrimitive::Curve { curve, mask });
                }
            }
        }
        segments.push(SegmentPrimitive::End {
            clock_wise: area < 0.0,
        });
    }

    segments
}
