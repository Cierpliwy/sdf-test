use crate::ui::block::{UIBlock, UIBlockContext, UIBlockStyle};
use crate::ui::UILayout;
use glium::Surface;
use std::rc::Rc;

pub struct UIButtonContext {
    block_context: Rc<UIBlockContext>,
}

impl UIButtonContext {
    fn new(block_context: Rc<UIBlockContext>) -> Self {
        Self { block_context }
    }
}

pub struct UIButton {
    context: Rc<UIButtonContext>,
    block: UIBlock,
    style: UIBlockStyle,
}

impl UIButton {
    pub fn new(context: Rc<UIButtonContext>) -> Self {
        let block = UIBlock::new(context.block_context.clone());
        let style = UIBlockStyle {
            alpha: 0.95,
            sharpness: 1.0,
            radius: 50.0,
            left_offset: -10.0,
            left_color: [1.0, 0.7, 8.0],
            right_offset: 450.0,
            right_color: [6.0, 1.0, 9.0],
            inner_shadow: 60.0,
            shade_color: [0.0, 0.0, 0.0],
        };

        Self {
            context,
            block,
            style,
        }
    }

    pub fn render<S: ?Sized + Surface>(&self, surface: &mut S, layout: &UILayout) {
        self.block.render(surface, &self.style, layout);
    }
}
