use crate::ui::block::{UIBlock, UIBlockContext, UIBlockStyle};
use crate::ui::label::{UILabel, UILabelAlignment, UILabelContext, UILabelStyle};
use crate::ui::layout::{UIAbsoluteLayout, UILayout, UILayoutResult, UIScaleLayout};
use crate::ui::widget::UIWidget;
use crate::ui::UIFrameInput;
use crate::utils::*;
use glium::Frame;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

pub struct UISliderContext {
    block_context: Rc<UIBlockContext>,
    label_context: Rc<RefCell<UILabelContext>>,
}

impl UISliderContext {
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

pub struct UISlider {
    context: Rc<UISliderContext>,
    block: UIBlock,
    dot: UIBlock,
    label: UILabel,
    hover: bool,
    pressed: bool,
    hover_from: f32,
    hover_to: f32,
    hover_time: Instant,
    min_value: f32,
    max_value: f32,
    step_value: f32,
    value: f32,
    drag_value: Option<f32>,
}

impl UISlider {
    pub fn new(
        context: Rc<UISliderContext>,
        min_value: f32,
        max_value: f32,
        step_value: f32,
        value: f32,
    ) -> Self {
        let block = UIBlock::new(
            context.block_context.clone(),
            UIBlockStyle {
                alpha: 0.95,
                sharpness: 1.0,
                radius: 4.0,
                left_offset: 0.0,
                left_color: [0.016, 0.404, 0.557],
                right_offset: 0.0,
                right_color: [0.05, 0.05, 0.05],
                inner_shadow: 2.0,
                shade_color: [0.02, 0.02, 0.02],
            },
        );

        let dot = UIBlock::new(
            context.block_context.clone(),
            UIBlockStyle {
                alpha: 0.95,
                sharpness: 1.0,
                radius: 8.0,
                left_offset: -10.0,
                left_color: [0.016, 0.404, 0.557],
                right_offset: 20.0,
                right_color: [0.6, 0.1, 0.9],
                inner_shadow: 20.0,
                shade_color: [0.0, 0.0, 0.0],
            },
        );

        let label = UILabel::new(
            context.label_context.clone(),
            &format!("{}", value),
            UILabelStyle {
                size: 15.0,
                align: UILabelAlignment::Center,
                color: [0.7, 0.7, 0.7, 1.0],
                shadow_color: [0.0, 0.0, 0.0, 1.0],
            },
        );

        Self {
            context,
            block,
            dot,
            label,
            hover: false,
            pressed: false,
            hover_from: 0.0,
            hover_to: 0.0,
            hover_time: Instant::now(),
            min_value,
            max_value,
            step_value,
            value,
            drag_value: None,
        }
    }

    fn hover_value(&self) -> f32 {
        let animation = (self.hover_time.elapsed_seconds() * 8.0).min(1.0) as f32;
        let t = (self.hover_to - self.hover_from) * animation + self.hover_from;
        1.0 - (t - 1.0).powf(2.0)
    }

    fn value_from_pos(&self, pos: f32, layout: UILayoutResult) -> f32 {
        let value = ((pos - layout.pos[0]) / layout.size[0]).max(0.0).min(1.0);
        (value * (self.max_value - self.min_value) / self.step_value + 0.5).floor()
            * self.step_value
    }

    fn value_to_pos(&self, value: f32, layout: UILayoutResult) -> f32 {
        let value = (value / self.step_value + 0.5).floor() * self.step_value;
        (value - self.min_value) / (self.max_value - self.min_value) * layout.size[0]
    }

    fn calc_dot_layout(&self, layout: UILayoutResult) -> UILayoutResult {
        let dot_size = self.dot.get_style().radius * 2.0;
        let mut value = if let Some(drag_value) = self.drag_value {
            drag_value
        } else {
            self.value
        };
        value = self.value_to_pos(value, layout);

        let dot_layout = UIAbsoluteLayout {
            size: [dot_size, dot_size],
            pos: [value - dot_size / 2.0, (layout.size[1] - dot_size) / 2.0],
        };

        let scale = 1.0 + 0.3 * self.hover_value();
        let scale_layout = UIScaleLayout {
            scale: [scale, scale],
            anchor: [0.5, 0.5],
        };

        scale_layout.layout(dot_layout.layout(layout))
    }
}

pub enum UISliderEvent {
    ValueChanged(f32),
    ValueFinished(f32),
}

impl UIWidget for UISlider {
    type Event = UISliderEvent;

    fn render(&self, frame: &mut Frame, layout: UILayoutResult) {
        let UILayoutResult { size, .. } = layout;

        // Dot layout
        let dot_layout = self.calc_dot_layout(layout);
        let center = dot_layout.pos[0] + dot_layout.size[0] / 2.0 - layout.pos[0];

        // Background
        let background_style = UIBlockStyle {
            left_offset: center - 2.0,
            right_offset: center + 2.0,
            ..self.block.get_style()
        };
        let background_height = background_style.radius * 2.0;
        let background_layout = UIAbsoluteLayout {
            size: [size[0], background_height],
            pos: [0.0, (size[1] - background_height) / 2.0],
        };
        let background_layout = background_layout.layout(layout);
        self.block
            .render_styled(frame, background_layout, background_style);

        // Dot
        let pressed_value = if self.drag_value.is_some() { 1.0 } else { 0.0 };
        let dot_style = UIBlockStyle {
            shade_color: [pressed_value, pressed_value, pressed_value],
            radius: 8.0 * (1.0 + 0.3 * self.hover_value()),
            ..self.dot.get_style()
        };
        self.dot.render_styled(frame, dot_layout, dot_style);

        // Label
        let label_layout = UIAbsoluteLayout {
            pos: [0.0, 20.0],
            size: dot_layout.size,
        };

        self.label.render(frame, label_layout.layout(dot_layout));
    }

    fn update_input(
        &mut self,
        layout: UILayoutResult,
        frame_input: UIFrameInput,
        events: &mut Vec<UISliderEvent>,
    ) {
        let dot_layout = self.calc_dot_layout(layout);
        let hover = dot_layout.is_inside(frame_input.mouse_pos);
        let pressed = frame_input.left_mouse_button_pressed;

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

        if !self.pressed && pressed && hover && self.drag_value.is_none() {
            self.drag_value = Some(self.value);
        }

        if let Some(old_value) = self.drag_value {
            let new_value = self.value_from_pos(frame_input.mouse_pos[0], layout);
            if !pressed {
                self.value = new_value;
                self.label.set_text(&self.value.to_string());
                self.drag_value = None;
                events.push(UISliderEvent::ValueFinished(new_value));
            } else {
                if old_value != new_value {
                    events.push(UISliderEvent::ValueChanged(new_value));
                    self.label.set_text(&new_value.to_string());
                }
                self.drag_value = Some(new_value);
            }
        }

        self.pressed = pressed;
        self.hover = hover;
    }
}
