use crate::ui::block::{UIBlock, UIBlockContext, UIBlockStyle};
use crate::ui::label::{UILabel, UILabelAlignment, UILabelContext};
use crate::ui::{UILayout, UIScaleLayout};
use crate::utils::*;
use glium::Surface;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

pub struct UIButtonContext {
    block_context: Rc<UIBlockContext>,
    label_context: Rc<RefCell<UILabelContext>>,
}

impl UIButtonContext {
    pub fn new(
        block_context: Rc<UIBlockContext>,
        label_context: Rc<RefCell<UILabelContext>>,
    ) -> Self {
        Self {
            block_context,
            label_context,
        }
    }
}

pub struct UIButton {
    context: Rc<UIButtonContext>,
    block: UIBlock,
    label: UILabel,
    style: UIBlockStyle,
    hover: bool,
    pressed: bool,
    toggled: bool,
    hover_from: f32,
    hover_to: f32,
    hover_time: Instant,
}

impl UIButton {
    pub fn new(context: Rc<UIButtonContext>, title: &str) -> Self {
        let block = UIBlock::new(context.block_context.clone());
        let style = UIBlockStyle {
            alpha: 0.95,
            sharpness: 1.0,
            radius: 5.0,
            left_offset: 0.0,
            left_color: [0.0, 0.0, 0.0],
            right_offset: 3.0,
            right_color: [0.6, 0.1, 0.9],
            inner_shadow: 10.0,
            shade_color: [0.0, 0.0, 0.0],
        };

        let label = UILabel::new(
            context.label_context.clone(),
            title,
            0.0,
            UILabelAlignment::Center,
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        );

        Self {
            context,
            block,
            style,
            label,
            hover: false,
            pressed: false,
            toggled: false,
            hover_from: 0.0,
            hover_to: 0.0,
            hover_time: Instant::now(),
        }
    }

    pub fn render<S: ?Sized + Surface>(&mut self, surface: &mut S, layout: &UILayout) {
        let hover_value = self.hover_value();
        let pressed_value = if self.pressed { 1.0 } else { 0.0 };
        let toggle_value = if self.toggled { 1.0 } else { 0.2 };

        let scale = 1.0 + 0.1 * hover_value;
        let layout = UIScaleLayout::new(layout, [scale, scale], [0.5, 0.5]);

        let size = layout.get_size();
        let style = UIBlockStyle {
            left_offset: size[0] * self.style.left_offset,
            left_color: [
                0.016 * toggle_value,
                0.404 * toggle_value,
                0.557 * toggle_value,
            ],
            right_offset: size[0] * self.style.right_offset,
            radius: self.style.radius - 2.0 * hover_value,
            shade_color: [pressed_value, pressed_value, pressed_value],
            ..self.style
        };

        self.block.render(surface, &style, &layout);
        self.label.set_size(25.0 * scale);
        self.label.set_color([
            0.14 * hover_value + 0.07 / toggle_value,
            0.1 * hover_value + 0.05 / toggle_value,
            0.22 * hover_value + 0.11 / toggle_value,
            1.0,
        ]);
        self.label.render(surface, &layout);
    }

    pub fn set_hover(&mut self, hover: bool) {
        if self.hover {
            if !hover {
                self.hover_from = self.hover_value();
                self.hover_to = 0.0;
                self.hover_time = Instant::now();
            }
        } else {
            if hover {
                self.hover_from = self.hover_value();
                self.hover_to = 1.0;
                self.hover_time = Instant::now();
            }
        }
        self.hover = hover;
    }

    pub fn get_hover(&self) -> bool {
        self.hover
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    pub fn get_toggled(&self) -> bool {
        self.toggled
    }

    pub fn set_pressed(&mut self, pressed: bool) {
        self.pressed = pressed;
    }

    pub fn get_pressed(&self) -> bool {
        self.pressed
    }

    fn hover_value(&self) -> f32 {
        let animation = (self.hover_time.elapsed_seconds() * 8.0).min(1.0) as f32;
        let t = (self.hover_to - self.hover_from) * animation + self.hover_from;
        1.0 - (t - 1.0).powf(2.0)
    }
}
