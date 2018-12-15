use glium::backend::{Context, Facade};
use std::cell::Cell;
use std::rc::Rc;

pub mod block;
pub mod button;

#[derive(Clone)]
pub struct UIContext {
    gl_context: Rc<Context>,
}

impl UIContext {
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Self {
        Self {
            gl_context: facade.get_context().clone(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct UIScreenInfo {
    size: [f32; 2],
    ratio: f32,
}

impl UIScreenInfo {
    pub fn new(size: [f32; 2], ratio: f32) -> Self {
        Self { size, ratio }
    }

    pub fn get_ratio(&self) -> f32 {
        self.ratio
    }
}

pub type UIScreen = Rc<Cell<UIScreenInfo>>;

pub trait UILayout {
    fn get_screen(&self) -> UIScreen;
    fn get_pos(&self) -> [f32; 2];
    fn get_size(&self) -> [f32; 2];
}

impl UILayout for UIScreen {
    fn get_screen(&self) -> UIScreen {
        self.clone()
    }

    fn get_pos(&self) -> [f32; 2] {
        [0.0, 0.0]
    }

    fn get_size(&self) -> [f32; 2] {
        self.get().size
    }
}

// ============ Absolute Layout =========================================================

pub struct UIAbsoluteLayout<'a> {
    parent: &'a UILayout,
    size: [f32; 2],
    pos: [f32; 2],
}

impl<'a> UIAbsoluteLayout<'a> {
    pub fn new(parent: &'a UILayout, size: [f32; 2], pos: [f32; 2]) -> Self {
        Self { parent, size, pos }
    }
}

impl UILayout for UIAbsoluteLayout<'_> {
    fn get_screen(&self) -> Rc<Cell<UIScreenInfo>> {
        self.parent.get_screen()
    }

    fn get_pos(&self) -> [f32; 2] {
        let pos = self.parent.get_pos();
        let ratio = self.get_screen().get().ratio;
        [pos[0] + self.pos[0] * ratio, pos[1] + self.pos[1] * ratio]
    }

    fn get_size(&self) -> [f32; 2] {
        self.size
    }
}

// ============ Relative Layout =========================================================

pub struct UIRelativeLayout<'a> {
    parent: &'a UILayout,
    size: [f32; 2],
    pos: [f32; 2],
}

impl<'a> UIRelativeLayout<'a> {
    pub fn new(parent: &'a UILayout, size: [f32; 2], pos: [f32; 2]) -> Self {
        Self { parent, size, pos }
    }
}

impl UILayout for UIRelativeLayout<'_> {
    fn get_screen(&self) -> Rc<Cell<UIScreenInfo>> {
        self.parent.get_screen()
    }

    fn get_pos(&self) -> [f32; 2] {
        let size = self.parent.get_size();
        [size[0] * self.pos[0], size[1] * self.pos[1]]
    }

    fn get_size(&self) -> [f32; 2] {
        let size = self.parent.get_size();
        [size[0] * self.size[0], size[1] * self.size[1]]
    }
}
