use std::f32::consts::PI;

pub fn solve_quadratic(a: f32, b: f32, c: f32) -> (Option<f32>, Option<f32>) {
    if a == 0.0 {
        if b == 0.0 {
            return (None, None);
        }
        return (Some(-c / b), None);
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return (None, None);
    }
    if discriminant == 0.0 {
        return (Some(-0.5 * b / a), None);
    }

    let sqrt_delta = discriminant.sqrt();
    let t1 = (-b + sqrt_delta) / (2.0 * a);
    let t2 = (-b - sqrt_delta) / (2.0 * a);

    (Some(t1), Some(t2))
}

#[allow(clippy::many_single_char_names)]
pub fn solve_cubic(a: f32, b: f32, c: f32, d: f32) -> (Option<f32>, Option<f32>, Option<f32>) {
    if a == 0.0 {
        let (r1, r2) = solve_quadratic(b, c, d);
        return (r1, r2, None);
    }

    let b = b / a;
    let c = c / a;
    let d = d / a;

    let q = (b * b - 3.0 * c) / 9.0;
    let r = (2.0 * b * b * b - 9.0 * b * c + 27.0 * d) / 54.0;
    let qqq = q * q * q;
    let discriminant = qqq - r * r;
    let third = 1.0 / 3.0;

    if discriminant >= 0.0 {
        let twopi = 2.0 * PI;
        let theta = (r / qqq.sqrt()).acos();
        let mult = -2.0 * q.sqrt();
        let add = -b * third;
        let r0 = mult * (third * theta).cos() + add;
        let r1 = mult * (third * (theta + twopi)).cos() + add;
        let r2 = mult * (third * (theta + twopi + twopi)).cos() + add;
        return (Some(r0), Some(r1), Some(r2));
    }

    let temp = ((-discriminant).sqrt() + r.abs()).powf(third);
    let sign = r.signum();
    let r = -sign * (temp + q / temp) - third * b;
    return (Some(r), None, None);
}

pub fn median<T: Ord + Copy>(c: [T; 3]) -> T {
    c[0].min(c[1]).max(c[0].max(c[1]).min(c[2]))
}

pub fn median_f32(c: [f32; 3]) -> f32 {
    c[0].min(c[1]).max(c[0].max(c[1]).min(c[2]))
}

pub fn clamp<T: Ord + Copy>(x: T, x_min: T, x_max: T) -> T {
    x.max(x_min).min(x_max)
}

pub fn clamp_f32(x: f32, x_min: f32, x_max: f32) -> f32 {
    x.max(x_min).min(x_max)
}

pub fn min<T: Ord + Copy>(c: [T; 3]) -> T {
    c[0].min(c[1]).min(c[2])
}

pub fn max<T: Ord + Copy>(c: [T; 3]) -> T {
    c[0].max(c[1]).max(c[2])
}
