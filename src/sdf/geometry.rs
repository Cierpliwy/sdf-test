use super::solver::solve_cubic;
use cgmath::prelude::*;
use cgmath::{dot, Point2, Vector2};
use std::f32::MAX;
use std::ops::Sub;

#[derive(Debug, Clone, Copy)]
pub struct SignedDistance {
    pub real_dist: f32,
    pub real_pos: f32,
    pub extended_dist: f32,
    pub extended_pos: f32,
    pub orthogonality: f32,
    pub sign: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect<T> {
    pub min: Point2<T>,
    pub max: Point2<T>,
}

impl<T: Sub<Output = T> + Copy> Rect<T> {
    pub fn new(min_x: T, min_y: T, max_x: T, max_y: T) -> Self {
        Rect {
            min: Point2::new(min_x, min_y),
            max: Point2::new(max_x, max_y),
        }
    }

    pub fn width(&self) -> T {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> T {
        self.max.y - self.min.y
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub p0: Point2<f32>,
    pub p1: Point2<f32>,
}

impl Line {
    pub fn new(p0: Point2<f32>, p1: Point2<f32>) -> Self {
        Line { p0, p1 }
    }

    pub fn bounding_box(&self) -> Rect<f32> {
        let (a, b) = (self.p0, self.p1);
        let (min_x, max_x) = if a.x < b.x { (a.x, b.x) } else { (b.x, a.x) };
        let (min_y, max_y) = if a.y < b.y { (a.y, b.y) } else { (b.y, a.y) };
        Rect {
            min: Point2 { x: min_x, y: min_y },
            max: Point2 { x: max_x, y: max_y },
        }
    }

    pub fn signed_distance(&self, p: Point2<f32>) -> SignedDistance {
        let p1_p0 = self.p1 - self.p0;
        let p_p0 = p - self.p0;

        let extended_pos = dot(p_p0, p1_p0) / dot(p1_p0, p1_p0);
        let real_pos = extended_pos.max(0.0).min(1.0);

        let extended_dist = (extended_pos * p1_p0 - p_p0).magnitude();
        let real_dist = (real_pos * p1_p0 - p_p0).magnitude();

        let pt = self.p0 + real_pos * p1_p0;
        let p_pt = p - pt;

        let orthogonality = if p_pt.x == 0.0 && p_pt.y == 0.0 {
            0.0
        } else {
            p1_p0.normalize().perp_dot(p_pt.normalize())
        };

        let sign = orthogonality.signum();
        let orthogonality = orthogonality.abs();

        SignedDistance {
            real_dist,
            real_pos,
            extended_dist,
            extended_pos,
            orthogonality,
            sign,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Curve {
    pub p0: Point2<f32>,
    pub p1: Point2<f32>,
    pub p2: Point2<f32>,
}

impl Curve {
    pub fn new(p0: Point2<f32>, p1: Point2<f32>, p2: Point2<f32>) -> Self {
        Curve { p0, p1, p2 }
    }

    pub fn bounding_box(&self) -> Rect<f32> {
        let p0 = self.p0;
        let p1 = self.p1;
        let p2 = self.p2;

        let (min_x, max_x) = if p0.x <= p1.x && p1.x <= p2.x {
            (p0.x, p2.x)
        } else if p0.x >= p1.x && p1.x >= p2.x {
            (p2.x, p0.x)
        } else {
            let t = (p0.x - p1.x) / (p0.x - 2.0 * p1.x + p2.x);
            let _1mt = 1.0 - t;
            let inflection = _1mt * _1mt * p0.x + 2.0 * _1mt * t * p1.x + t * t * p2.x;
            if p1.x < p0.x {
                (inflection, p0.x.max(p2.x))
            } else {
                (p0.x.min(p2.x), inflection)
            }
        };

        let (min_y, max_y) = if p0.y <= p1.y && p1.y <= p2.y {
            (p0.y, p2.y)
        } else if p0.y >= p1.y && p1.y >= p2.y {
            (p2.y, p0.y)
        } else {
            let t = (p0.y - p1.y) / (p0.y - 2.0 * p1.y + p2.y);
            let _1mt = 1.0 - t;
            let inflection = _1mt * _1mt * p0.y + 2.0 * _1mt * t * p1.y + t * t * p2.y;
            if p1.y < p0.y {
                (inflection, p0.y.max(p2.y))
            } else {
                (p0.y.min(p2.y), inflection)
            }
        };

        Rect {
            min: Point2 { x: min_x, y: min_y },
            max: Point2 { x: max_x, y: max_y },
        }
    }

    pub fn signed_distance(&self, p: Point2<f32>) -> SignedDistance {
        let p = p.to_vec();
        let p0 = self.p0.to_vec();
        let p1 = self.p1.to_vec();
        let p2 = self.p2.to_vec();

        let v = p - p0;
        let v1 = p1 - p0;
        let v2 = p2 - 2.0 * p1 + p0;

        let a = v2.dot(v2);
        let b = 3.0 * v1.dot(v2);
        let c = 2.0 * v1.dot(v1) - v2.dot(v);
        let d = -v1.dot(v);

        let (t1, t2, t3) = solve_cubic(a, b, c, d);

        struct DistResult {
            dist2: f32,
            t: f32,
            pt: Vector2<f32>,
        };

        let mut dist_result = DistResult {
            dist2: MAX,
            pt: Vector2::new(0.0, 0.0),
            t: 0.0,
        };

        {
            let mut update_closest_t = |root: Option<f32>| {
                if let Some(t) = root {
                    let ct = t.max(0.0).min(1.0);
                    let pt = ct * ct * v2 + 2.0 * ct * v1 + p0;
                    let dist2 = (p - pt).magnitude2();
                    if dist2 < dist_result.dist2 {
                        dist_result = DistResult { dist2, pt, t }
                    }
                }
            };

            update_closest_t(t1);
            update_closest_t(t2);
            update_closest_t(t3);
        }

        let extended_pos = dist_result.t;
        let real_pos = extended_pos.max(0.0).min(1.0);

        let dir = 2.0 * real_pos * v2 + 2.0 * v1;
        let p_pt = p - dist_result.pt;
        let orthogonality = if p_pt.is_zero() || dir.is_zero() {
            0.0
        } else {
            dir.normalize().perp_dot(p_pt.normalize())
        };

        let sign = orthogonality.signum();
        let orthogonality = orthogonality.abs();

        let real_dist = dist_result.dist2.sqrt();
        let extended_dist =
            (extended_pos * extended_pos * v2 + 2.0 * extended_pos * v1 - v).magnitude();

        SignedDistance {
            real_dist,
            real_pos,
            extended_dist,
            extended_pos,
            orthogonality,
            sign,
        }
    }
}
