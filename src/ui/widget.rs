use glium::Frame;

// Helper structures ----------------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct UIPoint {
    pub left: f32,
    pub top: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct UISize {
    pub width: f32,
    pub height: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct UILayout {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct UIFrameInput {
    pub mouse_pos: UIPoint,
    pub left_mouse_button_pressed: bool,
    pub right_mouse_button_pressed: bool,
    pub mouse_wheel_delta: Option<f32>,
}

impl UIPoint {
    pub fn zero() -> Self {
        UIPoint {
            left: 0.0,
            top: 0.0,
        }
    }
}

impl UISize {
    pub fn zero() -> Self {
        UISize {
            width: 0.0,
            height: 0.0,
        }
    }
}

impl UILayout {
    pub fn zero() -> Self {
        UILayout {
            left: 0.0,
            top: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn from_size(size: UISize) -> Self {
        UILayout {
            left: 0.0,
            top: 0.0,
            width: size.width,
            height: size.height,
        }
    }

    pub fn is_inside(&self, point: UIPoint) -> bool {
        point.left >= self.left
            && point.left <= self.left + self.width
            && point.top >= self.top
            && point.top <= self.top + self.height
    }

    pub fn extend(&self, padding: f32) -> UILayout {
        UILayout {
            left: self.left - padding,
            top: self.top - padding,
            width: self.width + padding * 2.0,
            height: self.height + padding * 2.0,
        }
    }
}

impl UIFrameInput {
    fn new() -> Self {
        Self {
            mouse_pos: UIPoint::zero(),
            left_mouse_button_pressed: false,
            right_mouse_button_pressed: false,
            mouse_wheel_delta: None,
        }
    }
}

// Widget definition and IDs --------------------------------------------------

#[derive(Copy, Clone, Debug)]
pub struct UIWidgetId {
    id: usize,
}

pub struct UITypedWidgetId<T: UIWidget> {
    id: usize,
    ptr: *mut UITypedWidgetData<T>,
    _marker: std::marker::PhantomData<T>,
}

pub trait UIWidget {
    type Event;

    fn measure(&self, _children: &[UISize]) -> UISize {
        UISize::zero()
    }

    fn layout(&self, _layout: UILayout, _children: &mut [UILayout]) {}

    fn render(&self, _frame: &mut Frame, _layout: UILayout, _screen: UISize) {}

    fn update_input(
        &mut self,
        _layout: UILayout,
        _frame_input: UIFrameInput,
        _events: &mut Vec<Self::Event>,
    ) {
    }
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

// Widget manager -------------------------------------------------------------

trait UIWidgetData {
    fn add_child(&mut self, child: UIWidgetId);
    fn get_children(&self) -> &[UIWidgetId];
    fn set_layout(&mut self, layout: UILayout);
    fn get_layout(&self) -> UILayout;
    fn set_size(&mut self, size: UISize);
    fn get_size(&self) -> UISize;

    fn measure(&self, children: &[UISize]) -> UISize;
    fn layout(&self, children: &mut [UILayout]);
    fn render(&self, frame: &mut Frame, screen: UISize);
    fn update_input(&mut self, frame_input: UIFrameInput);
}

struct UITypedWidgetData<T: UIWidget> {
    layout: UILayout,
    size: UISize,
    children: Vec<UIWidgetId>,
    widget: T,
    events: Vec<T::Event>,
}

impl<T: UIWidget> UIWidgetData for UITypedWidgetData<T> {
    fn add_child(&mut self, child: UIWidgetId) {
        self.children.push(child);
    }
    fn get_children(&self) -> &[UIWidgetId] {
        &self.children
    }
    fn set_layout(&mut self, layout: UILayout) {
        self.layout = layout;
    }
    fn get_layout(&self) -> UILayout {
        self.layout
    }
    fn set_size(&mut self, size: UISize) {
        self.size = size;
    }
    fn get_size(&self) -> UISize {
        self.size
    }
    fn measure(&self, children: &[UISize]) -> UISize {
        self.widget.measure(children)
    }
    fn layout(&self, children: &mut [UILayout]) {
        self.widget.layout(self.layout, children);
    }
    fn render(&self, frame: &mut Frame, screen: UISize) {
        self.widget.render(frame, self.layout, screen);
    }
    fn update_input(&mut self, frame_input: UIFrameInput) {
        self.widget
            .update_input(self.layout, frame_input, &mut self.events);
    }
}

pub struct UIWidgetManager {
    screen: UISize,
    widgets: Vec<Box<UIWidgetData>>,
    root: Option<UIWidgetId>,
    frame_input: UIFrameInput,
}

impl UIWidgetManager {
    pub fn new(screen: UISize) -> Self {
        UIWidgetManager {
            screen,
            widgets: Vec::new(),
            root: None,
            frame_input: UIFrameInput::new(),
        }
    }

    pub fn set_screen(&mut self, screen: UISize) {
        self.screen = screen
    }

    pub fn get_screen(&self) -> UISize {
        self.screen
    }

    pub fn set_mouse_pos(&mut self, pos: UIPoint) {
        self.frame_input.mouse_pos = pos;
    }

    pub fn set_left_mouse_button_pressed(&mut self, pressed: bool) {
        self.frame_input.left_mouse_button_pressed = pressed;
    }

    pub fn set_right_mouse_button_pressed(&mut self, pressed: bool) {
        self.frame_input.right_mouse_button_pressed = pressed;
    }

    pub fn set_mouse_wheel_delta(&mut self, delta: Option<f32>) {
        self.frame_input.mouse_wheel_delta = delta;
    }

    pub fn create<T: UIWidget + 'static>(&mut self, widget: T) -> UITypedWidgetId<T> {
        let id = self.widgets.len();
        let mut data = Box::new(UITypedWidgetData {
            layout: UILayout::zero(),
            size: UISize::zero(),
            children: Vec::new(),
            events: Vec::new(),
            widget,
        });
        let ptr = &mut *data as *mut UITypedWidgetData<T>;
        self.widgets.push(data);
        UITypedWidgetId {
            id,
            ptr,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn update<T: UIWidget, F: FnMut(&mut T)>(&mut self, id: UITypedWidgetId<T>, mut func: F) {
        func(unsafe { &mut (*id.ptr).widget });
    }

    pub fn poll_events<T: UIWidget, F: FnMut(&T::Event)>(
        &mut self,
        id: UITypedWidgetId<T>,
        mut func: F,
    ) {
        let state: &mut UITypedWidgetData<T> = unsafe { &mut *id.ptr };
        for e in &state.events {
            func(&e);
        }
        state.events.clear();
    }

    pub fn root<T: Into<UIWidgetId>>(&mut self, widget: T) {
        self.root = Some(widget.into())
    }

    pub fn add_child<T1: Into<UIWidgetId>, T2: Into<UIWidgetId>>(&mut self, parent: T1, child: T2) {
        let widget_data = &mut self.widgets[parent.into().id];
        widget_data.add_child(child.into());
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let mut index = 0;
        let mut widgets = Vec::with_capacity(self.widgets.len());

        if let Some(root) = self.root {
            let widget_data = &mut self.widgets[root.id];
            widget_data.set_layout(UILayout::from_size(self.screen));
            widgets.push(root);
        }

        while index < widgets.len() {
            let widget = widgets[index];
            let widget_data = &self.widgets[widget.id];
            widgets.extend(widget_data.get_children());
            index += 1;
        }

        for widget in widgets.iter().rev() {
            let widget_data = &self.widgets[widget.id];
            let children: Vec<UISize> = widget_data
                .get_children()
                .iter()
                .map(|child| self.widgets[child.id].get_size())
                .collect();

            let size = widget_data.measure(&children);
            self.widgets[widget.id].set_size(size);
        }

        for widget in widgets {
            let widget_data = &self.widgets[widget.id];
            let mut children_layouts: Vec<UILayout> = widget_data
                .get_children()
                .iter()
                .map(|child| {
                    let child = &self.widgets[child.id];
                    let size = child.get_size();
                    UILayout::from_size(size)
                })
                .collect();

            widget_data.layout(&mut children_layouts);

            let children_ids = widget_data.get_children().to_vec();
            for (index, child) in children_ids.iter().enumerate() {
                let child = &mut self.widgets[child.id];
                child.set_layout(children_layouts[index]);
            }

            let widget_data = &mut self.widgets[widget.id];
            widget_data.update_input(self.frame_input);
            widget_data.render(frame, self.screen);
        }
    }
}
