use crate::ui::layout::{UILayoutId, UILayoutManager};
use crate::ui::widget::{UIWidgetId, UIWidgetManager};
use glium::Frame;

pub mod block;
pub mod button;
pub mod label;
pub mod layout;
pub mod slider;
pub mod widget;

#[derive(Copy, Clone)]
pub struct UIFrameInput {
    mouse_pos: [f32; 2],
    left_mouse_button_pressed: bool,
    right_mouse_button_pressed: bool,
}

pub struct UIState {
    layout_manager: UILayoutManager,
    widget_manager: UIWidgetManager,
    frame_input: UIFrameInput,
    pinned_widgets: Vec<(UILayoutId, UIWidgetId)>,
}

impl UIState {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            frame_input: UIFrameInput {
                mouse_pos: [0.0, 0.0],
                left_mouse_button_pressed: false,
                right_mouse_button_pressed: false,
            },
            layout_manager: UILayoutManager::new(width, height),
            widget_manager: UIWidgetManager::new(),
            pinned_widgets: Vec::new(),
        }
    }

    pub fn set_mouse_pos(&mut self, mouse_pos: [f32; 2]) {
        self.frame_input.mouse_pos = mouse_pos;
    }

    pub fn set_left_mouse_button_pressed(&mut self, pressed: bool) {
        self.frame_input.left_mouse_button_pressed = pressed;
    }

    pub fn set_right_mouse_button_pressed(&mut self, pressed: bool) {
        self.frame_input.right_mouse_button_pressed = pressed;
    }

    pub fn pin_widget(&mut self, widget: UIWidgetId, layout: UILayoutId) {
        self.pinned_widgets.push((layout, widget))
    }

    pub fn render(&mut self, frame: &mut Frame) {
        for (layout, view) in &self.pinned_widgets {
            let layout_result = self.layout_manager.layout(Some(*layout));
            self.widget_manager
                .update_input(*view, layout_result, self.frame_input);
            self.widget_manager.render(
                frame,
                *view,
                layout_result,
                self.layout_manager.get_screen(),
            );
        }
    }

    pub fn layout<R, F: Fn(&UILayoutManager) -> R>(&self, func: F) -> R {
        func(&self.layout_manager)
    }

    pub fn update_layout<R, F: Fn(&mut UILayoutManager) -> R>(&mut self, func: F) -> R {
        func(&mut self.layout_manager)
    }

    pub fn widget<R, F: Fn(&UIWidgetManager) -> R>(&self, func: F) -> R {
        func(&self.widget_manager)
    }

    pub fn update_widget<R, F: Fn(&mut UIWidgetManager) -> R>(&mut self, func: F) -> R {
        func(&mut self.widget_manager)
    }
}
