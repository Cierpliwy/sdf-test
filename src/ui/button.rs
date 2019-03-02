use crate::ui::block::{UIBlock, UIBlockContext, UIBlockStyle};
use crate::ui::label::{UILabel, UILabelAlignment, UILabelContext, UILabelStyle};
use crate::ui::layout::{UILayout, UILayoutResult, UIScaleLayout, UIScreen};
use crate::ui::widget::UIWidget;
use crate::ui::UIFrameInput;
use crate::utils::*;
use glium::Frame;
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
    block: UIBlock,
    label: UILabel,
    hover: bool,
    pressed: bool,
    active: bool,
    toggled: bool,
    hover_from: f32,
    hover_to: f32,
    hover_time: Instant,
}

impl UIButton {
    pub fn new(context: &Rc<UIButtonContext>, title: &str) -> Self {
        let block = UIBlock::new(
            context.block_context.clone(),
            UIBlockStyle {
                alpha: 0.95,
                sharpness: 1.0,
                left_offset: 0.0,
                left_color: [0.0, 0.0, 0.0],
                right_offset: 3.0,
                right_color: [0.6, 0.1, 0.9],
                radius: 4.0,
                inner_shadow: 10.0,
                shade_color: [0.0, 0.0, 0.0],
            },
        );

        let label = UILabel::new(
            context.label_context.clone(),
            title,
            UILabelStyle {
                size: 0.0,
                align: UILabelAlignment::Center,
                color: [0.0, 0.0, 0.0, 1.0],
                shadow_color: [0.0, 0.0, 0.0, 1.0],
            },
        );

        Self {
            block,
            label,
            hover: false,
            pressed: false,
            active: false,
            toggled: false,
            hover_from: 0.0,
            hover_to: 0.0,
            hover_time: Instant::now(),
        }
    }

    fn calc_layout(&self, layout: UILayoutResult) -> UILayoutResult {
        let scale = 1.0 + 0.1 * self.hover_value();
        let scale_layout = UIScaleLayout {
            scale: [scale, scale],
            anchor: [0.5, 0.5],
        };
        scale_layout.layout(layout)
    }

    fn hover_value(&self) -> f32 {
        let animation = (self.hover_time.elapsed_seconds() * 8.0).min(1.0) as f32;
        let t = (self.hover_to - self.hover_from) * animation + self.hover_from;
        1.0 - (t - 1.0).powf(2.0)
    }
}

pub enum UIButtonEvent {
    Toggled(bool),
}

impl UIWidget for UIButton {
    type Event = UIButtonEvent;

    fn render(&self, frame: &mut Frame, layout: UILayoutResult, screen: UIScreen) {
        let scale = 1.0 + 0.1 * self.hover_value();
        let hover_value = self.hover_value();
        let pressed_value = if self.active { 1.0 } else { 0.0 };
        let toggle_value = if self.toggled { 1.0 } else { 0.1 };

        let scale_layout = self.calc_layout(layout);
        let size = scale_layout.size;

        let style = UIBlockStyle {
            alpha: 0.95,
            sharpness: 1.0,
            left_offset: 0.0,
            left_color: [
                0.016 * toggle_value,
                0.404 * toggle_value,
                0.557 * toggle_value,
            ],
            right_offset: size[0] * 3.0,
            right_color: [0.6, 0.1, 0.9],
            radius: 4.0 + 2.0 * hover_value,
            inner_shadow: 10.0 + 10.0 * pressed_value,
            shade_color: [pressed_value, pressed_value, pressed_value],
        };

        self.block.render_styled(frame, scale_layout, style, screen);
        let label_style = UILabelStyle {
            size: 25.0 * scale,
            color: [
                0.07 * hover_value + 0.07 / toggle_value,
                0.05 * hover_value + 0.05 / toggle_value,
                0.11 * hover_value + 0.11 / toggle_value,
                1.0,
            ],
            ..self.label.get_style()
        };
        self.label
            .render_styled(frame, scale_layout, label_style, screen);
    }

    fn update_input(
        &mut self,
        layout: UILayoutResult,
        frame_input: UIFrameInput,
        events: &mut Vec<UIButtonEvent>,
    ) {
        let scale_layout = self.calc_layout(layout);
        let hover = scale_layout.is_inside(frame_input.mouse_pos);
        let pressed = frame_input.left_mouse_button_pressed;

        if self.hover {
            if !hover {
                self.hover_from = self.hover_value();
                self.hover_to = 0.0;
                self.hover_time = Instant::now();
            }
        } else if hover {
            self.hover_from = self.hover_value();
            self.hover_to = 1.0;
            self.hover_time = Instant::now();
        }

        if !self.active && !self.pressed && pressed && hover {
            self.active = true;
        }

        if self.active && self.pressed && !pressed && hover {
            let toggled = !self.toggled;
            events.push(UIButtonEvent::Toggled(toggled));
            self.toggled = toggled;
        }

        if self.active && !(hover && pressed) {
            self.active = false;
        }

        self.pressed = pressed;
        self.hover = hover;
    }
}
