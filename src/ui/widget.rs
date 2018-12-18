use crate::ui::layout::UILayoutResult;
use crate::ui::UIFrameInput;
use glium::Frame;

#[derive(Default)]
pub struct UIWidgetManager {
    widgets: Vec<Box<UIWidgetState>>,
}

impl UIWidgetManager {
    pub fn new() -> Self {
        UIWidgetManager {
            widgets: Vec::new(),
        }
    }

    pub fn create<T: UIWidget + 'static>(&mut self, widget: T) -> UITypedWidgetId<T> {
        let id = self.widgets.len();
        let mut state = Box::new(UITypedWidgetState {
            widget,
            events: Vec::new(),
        });
        let ptr = &mut *state as *mut UITypedWidgetState<T>;
        self.widgets.push(state);
        UITypedWidgetId {
            id,
            ptr,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn render(&self, frame: &mut Frame, id: UIWidgetId, layout: UILayoutResult) {
        let state = &self.widgets[id.id];
        state.render(frame, layout);
    }

    pub fn update<T: UIWidget, F: Fn(&mut T)>(&mut self, id: UITypedWidgetId<T>, func: F) {
        let state: &mut UITypedWidgetState<T> = unsafe { &mut *id.ptr };
        func(&mut state.widget);
    }

    pub fn update_input(
        &mut self,
        id: UIWidgetId,
        layout: UILayoutResult,
        frame_input: UIFrameInput,
    ) {
        let state = &mut self.widgets[id.id];
        state.update_input(layout, frame_input);
    }

    pub fn poll_events<T: UIWidget, F: Fn(&T::Event)>(&mut self, id: UITypedWidgetId<T>, func: F) {
        let state: &mut UITypedWidgetState<T> = unsafe { &mut *id.ptr };
        for e in &state.events {
            func(&e);
        }
        state.events.clear();
    }
}

trait UIWidgetState {
    fn render(&self, frame: &mut Frame, layout: UILayoutResult);
    fn update_input(&mut self, layout: UILayoutResult, frame_input: UIFrameInput);
}

struct UITypedWidgetState<T: UIWidget> {
    widget: T,
    events: Vec<T::Event>,
}

impl<T: UIWidget> UIWidgetState for UITypedWidgetState<T> {
    fn render(&self, frame: &mut Frame, layout: UILayoutResult) {
        self.widget.render(frame, layout);
    }

    fn update_input(&mut self, layout: UILayoutResult, frame_input: UIFrameInput) {
        self.widget
            .update_input(layout, frame_input, &mut self.events);
    }
}

pub trait UIWidget {
    type Event;
    fn render(&self, frame: &mut Frame, layout: UILayoutResult);
    fn update_input(
        &mut self,
        _layout: UILayoutResult,
        _frame_input: UIFrameInput,
        _events: &mut Vec<Self::Event>,
    ) {
    }
}

#[derive(Copy, Clone)]
pub struct UIWidgetId {
    id: usize,
}

pub struct UITypedWidgetId<T: UIWidget> {
    id: usize,
    ptr: *mut UITypedWidgetState<T>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: UIWidget> Clone for UITypedWidgetId<T> {
    fn clone(&self) -> Self {
        UITypedWidgetId {
            id: self.id,
            ptr: self.ptr,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: UIWidget> Copy for UITypedWidgetId<T> {}

impl<T: UIWidget> From<UITypedWidgetId<T>> for UIWidgetId {
    fn from(id: UITypedWidgetId<T>) -> Self {
        UIWidgetId { id: id.id }
    }
}
