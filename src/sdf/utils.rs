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
