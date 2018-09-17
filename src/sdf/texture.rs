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

impl<'a> LockedTexture<'a> {
    pub fn modify_view<F: Fn(u32, u32, u32, u32) -> (u8, u8, u8)>(
        &self,
        view: &mut TextureView,
        func: F,
    ) {
        let texture = unsafe { &mut *self.texture };
        assert!(view.data == texture.data.as_mut_slice());

        for y in view.view.min.y..view.view.max.y {
            for x in view.view.min.x..view.view.max.x {
                let (r, g, b) = func(
                    x - view.view.min.x,
                    y - view.view.min.y,
                    view.view.width(),
                    view.view.height(),
                );
                let offset = 3 * (y * texture.width + x) as usize;
                texture.data[offset] = r;
                texture.data[offset + 1] = g;
                texture.data[offset + 2] = b;
            }
        }
    }
}
