// ============ Absolute Layout =========================================================

use super::widget::{UILayout, UIPoint, UISize, UIWidget};

#[derive(Copy, Clone)]
pub struct UIAbsoluteLayout {
    pub size: UISize,
    pub pos: UIPoint,
}

impl UIWidget for UIAbsoluteLayout {
    type Event = ();
    fn layout(&self, layout: UILayout, children: &mut [UILayout]) {
        for child in children {
            child.left = layout.left + self.pos.left;
            child.top = layout.top + self.pos.top;
            child.width = self.size.width;
            child.height = self.size.height;
        }
    }
}

// ============ Relative Layout =========================================================

pub struct UIRelativeLayout {
    pub size: UISize,
    pub pos: UIPoint,
}

impl UIWidget for UIRelativeLayout {
    type Event = ();
    fn layout(&self, layout: UILayout, children: &mut [UILayout]) {
        for child in children {
            child.left = layout.left + layout.width * self.pos.left;
            child.top = layout.top + layout.height * self.pos.top;
            child.width = layout.width * self.size.width;
            child.height = layout.height * self.size.height;
        }
    }
}

// ============ Scale Layout =========================================================

pub struct UIScaleLayout {
    pub scale: UISize,
    pub anchor: UIPoint,
}

impl UIWidget for UIScaleLayout {
    type Event = ();
    fn layout(&self, layout: UILayout, children: &mut [UILayout]) {
        let origin_left = self.anchor.left * layout.width + layout.left;
        let origin_top = self.anchor.top * layout.height + layout.top;

        for child in children {
            child.left = (layout.left - origin_left) * self.scale.width + origin_left;
            child.top = (layout.top - origin_top) * self.scale.height + origin_top;
            child.width = layout.width * self.scale.width;
            child.height = layout.height * self.scale.height;
        }
    }
}
