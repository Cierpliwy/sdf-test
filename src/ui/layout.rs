#[derive(Clone, Copy)]
pub struct UIScreen {
    pub width: f32,
    pub height: f32,
}

pub struct UILayoutManager {
    screen: UIScreen,
    layouts: Vec<UILayoutSlot>,
}

impl UILayoutManager {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            screen: UIScreen { width, height },
            layouts: Vec::new(),
        }
    }

    pub fn get_screen(&self) -> UIScreen {
        self.screen
    }

    pub fn set_screen(&mut self, screen: UIScreen) {
        self.screen = screen;
    }

    pub fn root<T: UILayout + 'static>(&mut self, layout: T) -> UITypedLayoutId<T> {
        self.create(layout, None)
    }

    pub fn attach<T: UILayout + 'static, ID: Into<UILayoutId>>(
        &mut self,
        layout: T,
        parent_id: ID,
    ) -> UITypedLayoutId<T> {
        self.create(layout, Some(parent_id.into()))
    }

    pub fn update<T: UILayout, F: Fn(&mut T)>(&mut self, id: UITypedLayoutId<T>, func: F) {
        func(unsafe { &mut *id.ptr });
    }

    pub fn layout(&self, id: Option<UILayoutId>) -> UILayoutResult {
        match id {
            Some(id) => {
                let slot = &self.layouts[id.id];
                let result = self.layout(slot.parent);
                slot.layout.layout(result)
            }
            None => UILayoutResult {
                pos: [0.0, 0.0],
                size: [self.screen.width, self.screen.height],
            },
        }
    }

    fn create<T: UILayout + 'static>(
        &mut self,
        layout: T,
        parent: Option<UILayoutId>,
    ) -> UITypedLayoutId<T> {
        let id = self.layouts.len();
        let mut layout = Box::new(layout);
        let ptr = &mut *layout as *mut T;
        self.layouts.push(UILayoutSlot { parent, layout });
        UITypedLayoutId {
            id,
            ptr,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Copy, Clone)]
pub struct UILayoutId {
    id: usize,
}

pub struct UITypedLayoutId<T: UILayout> {
    id: usize,
    ptr: *mut T,
    _marker: std::marker::PhantomData<T>,
}

impl<T: UILayout> Clone for UITypedLayoutId<T> {
    fn clone(&self) -> Self {
        UITypedLayoutId {
            id: self.id,
            ptr: self.ptr,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: UILayout> Copy for UITypedLayoutId<T> {}

impl<T: UILayout> From<UITypedLayoutId<T>> for UILayoutId {
    fn from(id: UITypedLayoutId<T>) -> Self {
        UILayoutId { id: id.id }
    }
}

struct UILayoutSlot {
    parent: Option<UILayoutId>,
    layout: Box<UILayout>,
}

// ============ Layout ==================================================================

#[derive(Copy, Clone, Debug)]
pub struct UILayoutResult {
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

impl UILayoutResult {
    pub fn is_inside(&self, point: [f32; 2]) -> bool {
        let pos = self.pos;
        let size = self.size;
        point[0] >= pos[0]
            && point[0] <= pos[0] + size[0]
            && point[1] >= pos[1]
            && point[1] <= pos[1] + size[1]
    }
}

impl PartialEq for UILayoutResult {
    fn eq(&self, rhs: &Self) -> bool {
        self.pos[0] == rhs.pos[0]
            && self.pos[1] == rhs.pos[1]
            && self.size[0] == rhs.size[0]
            && self.size[1] == rhs.size[1]
    }
}

pub trait UILayout {
    fn layout(&self, parent_result: UILayoutResult) -> UILayoutResult;
}

// ============ Absolute Layout =========================================================

#[derive(Copy, Clone)]
pub struct UIAbsoluteLayout {
    pub size: [f32; 2],
    pub pos: [f32; 2],
}

impl UILayout for UIAbsoluteLayout {
    fn layout(&self, parent_result: UILayoutResult) -> UILayoutResult {
        let UILayoutResult { pos, .. } = parent_result;
        UILayoutResult {
            pos: [pos[0] + self.pos[0], pos[1] + self.pos[1]],
            size: self.size,
        }
    }
}

// ============ Relative Layout =========================================================

pub struct UIRelativeLayout {
    pub size: [f32; 2],
    pub pos: [f32; 2],
}

impl UILayout for UIRelativeLayout {
    fn layout(&self, parent_result: UILayoutResult) -> UILayoutResult {
        let UILayoutResult { pos, size } = parent_result;
        UILayoutResult {
            pos: [
                pos[0] + size[0] * self.pos[0],
                pos[1] + size[1] * self.pos[1],
            ],
            size: [size[0] * self.size[0], size[1] * self.size[1]],
        }
    }
}

// ============ Scale Layout =========================================================

pub struct UIScaleLayout {
    pub scale: [f32; 2],
    pub anchor: [f32; 2],
}

impl UILayout for UIScaleLayout {
    fn layout(&self, parent_result: UILayoutResult) -> UILayoutResult {
        let UILayoutResult { pos, size } = parent_result;
        let origin = [
            self.anchor[0] * size[0] + pos[0],
            self.anchor[1] * size[1] + pos[1],
        ];
        UILayoutResult {
            pos: [
                (pos[0] - origin[0]) * self.scale[0] + origin[0],
                (pos[1] - origin[1]) * self.scale[1] + origin[1],
            ],
            size: [size[0] * self.scale[0], size[1] * self.scale[1]],
        }
    }
}

// ============ Test ====================================================================

#[test]
fn layout_manager_test() {
    let mut lm = UILayoutManager::new(200.0, 100.0);
    let upper_right = lm.root(UIRelativeLayout {
        pos: [0.5, 0.5],
        size: [0.5, 0.5],
    });
    let upper_right_padding = lm.attach(
        UIAbsoluteLayout {
            pos: [10.0, 10.0],
            size: [30.0, 20.0],
        },
        upper_right,
    );

    assert_eq!(
        lm.layout(Some(upper_right.into())),
        UILayoutResult {
            pos: [100.0, 50.0],
            size: [100.0, 50.0]
        }
    );
    assert_eq!(
        lm.layout(Some(upper_right_padding.into())),
        UILayoutResult {
            pos: [110.0, 60.0],
            size: [30.0, 20.0]
        }
    );

    lm.set_screen(UIScreen {
        width: 50.0,
        height: 50.0,
    });

    assert_eq!(
        lm.layout(Some(upper_right.into())),
        UILayoutResult {
            pos: [25.0, 25.0],
            size: [25.0, 25.0]
        }
    );
    assert_eq!(
        lm.layout(Some(upper_right_padding.into())),
        UILayoutResult {
            pos: [35.0, 35.0],
            size: [30.0, 20.0]
        }
    );

    lm.update(upper_right, |l| {
        l.pos = [0.0, 0.0];
    });

    assert_eq!(
        lm.layout(Some(upper_right.into())),
        UILayoutResult {
            pos: [0.0, 0.0],
            size: [25.0, 25.0]
        }
    );
    assert_eq!(
        lm.layout(Some(upper_right_padding.into())),
        UILayoutResult {
            pos: [10.0, 10.0],
            size: [30.0, 20.0]
        }
    );
}
