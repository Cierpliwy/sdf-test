use super::geometry::Rect;
use std::marker::PhantomData;

pub struct Texture {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

pub struct TextureViewAllocator {
    data: *mut [u8],
    width: u32,
    height: u32,
    free_space: Vec<Rect<u32>>,
}

pub struct TextureView {
    data: *mut [u8],
    view: Rect<u32>,
}

pub struct LockedTexture<'a> {
    texture: *mut Texture,
    phantom: PhantomData<&'a Texture>,
}

unsafe impl Send for TextureViewAllocator {}
unsafe impl Sync for TextureViewAllocator {}
unsafe impl Send for TextureView {}
unsafe impl Sync for TextureView {}
unsafe impl<'a> Send for LockedTexture<'a> {}
unsafe impl<'a> Sync for LockedTexture<'a> {}

impl TextureView {
    pub fn get_view(&self) -> Rect<u32> {
        self.view
    }
}

impl Texture {
    pub fn new(width: u32, height: u32) -> (Self, TextureViewAllocator) {
        let mut texture = Texture {
            data: vec![0; (width * height * 3) as usize],
            width,
            height,
        };
        let allocator = TextureViewAllocator {
            data: texture.data.as_mut_slice(),
            width: width,
            height: height,
            free_space: vec![Rect::new(0, 0, width, height)],
        };
        (texture, allocator)
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn lock(&mut self) -> LockedTexture {
        LockedTexture {
            texture: self,
            phantom: PhantomData,
        }
    }
}

impl TextureViewAllocator {
    pub fn get_free_space(&self) -> f32 {
        let free_space_area: f32 = self
            .free_space
            .iter()
            .map(|s| (s.width() * s.height()) as f32)
            .sum();

        free_space_area / (self.width * self.height) as f32
    }

    pub fn allocate(&mut self, width: u32, height: u32) -> Option<TextureView> {
        if width == 0 || height == 0 {
            return None;
        }

        let pos = self
            .free_space
            .iter()
            .position(|space| width <= space.width() && height <= space.height())?;

        let slot = self.free_space.swap_remove(pos);
        let free_width = slot.width() - width;
        let free_height = slot.height() - height;

        if free_width < free_height {
            if free_width > 0 {
                self.free_space.push(Rect::new(
                    slot.min.x + width,
                    slot.min.y,
                    slot.max.x,
                    slot.min.y + height,
                ));
            }
            if free_height > 0 {
                self.free_space.push(Rect::new(
                    slot.min.x,
                    slot.min.y + height,
                    slot.max.x,
                    slot.max.y,
                ));
            }
        } else {
            if free_height > 0 {
                self.free_space.push(Rect::new(
                    slot.min.x,
                    slot.min.y + height,
                    slot.min.x + width,
                    slot.max.y,
                ));
            }
            if free_width > 0 {
                self.free_space.push(Rect::new(
                    slot.min.x + width,
                    slot.min.y,
                    slot.max.x,
                    slot.max.y,
                ));
            }
        }

        self.free_space
            .sort_by(|x, y| (x.width() * x.height()).cmp(&(y.width() * y.height())));

        Some(TextureView {
            data: self.data,
            view: Rect::new(
                slot.min.x,
                slot.min.y,
                slot.min.x + width,
                slot.min.y + height,
            ),
        })
    }
}

pub struct PixelView {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub top_pixel: [u8; 3],
    pub left_pixel: [u8; 3],
    pub top_left_pixel: [u8; 3],
    pub top_right_pixel: [u8; 3],
}

impl<'a> LockedTexture<'a> {
    pub fn modify_view<F: Fn(PixelView) -> [u8; 3]>(&self, view: &mut TextureView, func: F) {
        let texture = unsafe { &mut *self.texture };
        assert!(view.data == texture.data.as_mut_slice());

        let mut top_pixel = [0, 0, 0];
        let mut left_pixel = [0, 0, 0];
        let mut top_left_pixel = [0, 0, 0];
        let mut top_right_pixel = [0, 0, 0];

        for y in view.view.min.y..view.view.max.y {
            for x in view.view.min.x..view.view.max.x {
                if y > view.view.min.y {
                    let top_offset = 3 * ((y - 1) * texture.width + x) as usize;
                    top_pixel[0] = texture.data[top_offset];
                    top_pixel[1] = texture.data[top_offset + 1];
                    top_pixel[2] = texture.data[top_offset + 2];

                    if x >= view.view.max.x {
                        top_right_pixel = [0, 0, 0];
                    } else {
                        top_right_pixel[0] = texture.data[top_offset + 3];
                        top_right_pixel[1] = texture.data[top_offset + 4];
                        top_right_pixel[2] = texture.data[top_offset + 5];
                    }
                }

                let mut pixel = func(PixelView {
                    x: x - view.view.min.x,
                    y: y - view.view.min.y,
                    width: view.view.width(),
                    height: view.view.height(),
                    top_pixel,
                    left_pixel,
                    top_left_pixel,
                    top_right_pixel,
                });

                let offset = 3 * (y * texture.width + x) as usize;
                texture.data[offset] = pixel[0];
                texture.data[offset + 1] = pixel[1];
                texture.data[offset + 2] = pixel[2];

                left_pixel = pixel;
                top_left_pixel = top_pixel;
            }
            left_pixel = [0, 0, 0];
            top_left_pixel = [0, 0, 0];
        }
    }
}
