use crate::ui::block::{UIBlock, UIBlockContext, UIBlockStyle};
use crate::ui::label::{UILabel, UILabelAlignment, UILabelContext};
use crate::ui::{UIAbsoluteLayout, UILayout};
use glium::Surface;
use std::cell::RefCell;
use std::rc::Rc;

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
    block_style: UIBlockStyle,
    dot: UIBlock,
    dot_style: UIBlockStyle,
    label: UILabel,
    hover: bool,
    min_value: f32,
    max_value: f32,
    step_value: f32,
    value: f32,
}

impl UISlider {
    pub fn new(
        context: Rc<UISliderContext>,
        min_value: f32,
        max_value: f32,
        step_value: f32,
        value: f32,
    ) -> Self {
        let block = UIBlock::new(context.block_context.clone());
        let block_style = UIBlockStyle {
            alpha: 0.95,
            sharpness: 1.0,
            radius: 5.0,
            left_offset: 0.0,
            left_color: [0.016, 0.404, 0.557],
            right_offset: 0.0,
            right_color: [0.05, 0.05, 0.05],
            inner_shadow: 4.0,
            shade_color: [0.02, 0.02, 0.02],
        };

        let dot = UIBlock::new(context.block_context.clone());
        let dot_style = UIBlockStyle {
            alpha: 0.95,
            sharpness: 1.0,
            radius: 10.0,
            left_offset: -10.0,
            left_color: [0.016, 0.404, 0.557],
            right_offset: 20.0,
            right_color: [0.6, 0.1, 0.9],
            inner_shadow: 20.0,
            shade_color: [0.0, 0.0, 0.0],
        };

        let label = UILabel::new(
            context.label_context.clone(),
            "",
            10.0,
            UILabelAlignment::Center,
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        );
        Self {
            context,
            block,
            block_style,
            dot,
            dot_style,
            label,
            hover: false,
            min_value,
            max_value,
            step_value,
            value,
        }
    }

    pub fn render<S: ?Sized + Surface>(&mut self, surface: &mut S, layout: &UILayout) {
        let size = layout.get_size();
        let value = size[0] * 0.5;

        // Background
        let background_style = UIBlockStyle {
            left_offset: value - 2.0,
            right_offset: value + 2.0,
            ..self.block_style
        };
        let background_height = background_style.radius * 2.0;
        let background_layout = UIAbsoluteLayout::new(
            layout,
            [size[0], background_height],
            [0.0, (size[1] - background_height) / 2.0],
        );
        self.block
            .render(surface, &background_style, &background_layout);

        // Dot
        let dot_style = UIBlockStyle { ..self.dot_style };
        let dot_size = dot_style.radius * 2.0;
        let dot_layout = UIAbsoluteLayout::new(
            layout,
            [dot_size, dot_size],
            [value - dot_size / 2.0, (size[1] - dot_size) / 2.0],
        );
        self.dot.render(surface, &dot_style, &dot_layout);

        self.label.render(surface, layout);
    }

    pub fn set_hover(&mut self, hover: bool) {
        self.hover = hover;
    }

    pub fn get_hover(&self) -> bool {
        self.hover
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }
}
