use std::{cell::Cell, rc::Rc};

use accesskit::{Action, ActionData};
use typed_builder::TypedBuilder;
use winit::{
    event::{DeviceId, ElementState, Ime, KeyEvent, MouseButton},
    keyboard::ModifiersState,
};

use crate::{
    draw::DrawEvent,
    types::{Point, Rect},
    widgets::{MountPoint, RawWidgetId},
    window::SharedWindowData,
};

use derive_more::From;

#[derive(Debug, Clone, From)]
pub enum Event {
    MouseInput(MouseInputEvent),
    CursorMove(CursorMoveEvent),
    CursorLeave(CursorLeaveEvent),
    KeyboardInput(KeyboardInputEvent),
    Ime(ImeEvent),
    Draw(DrawEvent),
    GeometryChange(GeometryChangeEvent),
    Mount(MountEvent),
    Unmount(UnmountEvent),
    FocusIn(FocusInEvent),
    FocusOut(FocusOutEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    Accessible(AccessibleEvent),
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct MouseInputEvent {
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    num_clicks: u32,
    pos: Point,
    accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

impl MouseInputEvent {
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn state(&self) -> ElementState {
        self.state
    }

    pub fn button(&self) -> MouseButton {
        self.button
    }

    pub fn num_clicks(&self) -> u32 {
        self.num_clicks
    }

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub(crate) fn accepted_by(&self) -> Option<RawWidgetId> {
        self.accepted_by.get()
    }

    pub(crate) fn set_accepted_by(&self, id: RawWidgetId) {
        self.accepted_by.set(Some(id));
    }

    pub fn map_to_child(&self, rect_in_parent: Rect) -> Option<Self> {
        if rect_in_parent.contains(self.pos) {
            let mut event = self.clone();
            event.pos -= rect_in_parent.top_left;
            Some(event)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct CursorMoveEvent {
    pub device_id: DeviceId,
    pub pos: Point,
    pub accepted_by: Rc<Cell<Option<RawWidgetId>>>,
    pub widget_id: RawWidgetId,
    pub window: SharedWindowData,
}

impl CursorMoveEvent {
    pub(crate) fn accepted_by(&self) -> Option<RawWidgetId> {
        self.accepted_by.get()
    }

    pub(crate) fn set_accepted_by(&self, id: RawWidgetId, rect_in_window: Rect) {
        self.accepted_by.set(Some(id));
        if self.is_enter() {
            self.window
                .0
                .borrow_mut()
                .mouse_entered_widgets
                .push((rect_in_window, id));
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn is_enter(&self) -> bool {
        !self
            .window
            .0
            .borrow()
            .mouse_entered_widgets
            .iter()
            .any(|(_, id)| *id == self.widget_id)
    }
}

#[derive(Debug, Clone)]
pub struct CursorLeaveEvent;

#[derive(Debug, Clone)]
pub struct KeyboardInputEvent {
    pub device_id: DeviceId,
    pub event: KeyEvent,
    pub is_synthetic: bool,
    pub modifiers: ModifiersState,
}

#[derive(Debug, Clone)]
pub struct ImeEvent(pub Ime);

#[derive(Debug, Clone)]
pub struct GeometryChangeEvent {
    pub new_rect_in_window: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct MountEvent(pub MountPoint);

#[derive(Debug, Clone)]
pub struct UnmountEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusReason {
    Mouse,
    Tab,
    /// A widget was automatically focused because there was no focused widget previously.
    Auto,
}

#[derive(Debug, Clone)]
pub struct FocusInEvent {
    pub reason: FocusReason,
}

#[derive(Debug, Clone)]
pub struct FocusOutEvent;

#[derive(Debug, Clone)]
pub struct WindowFocusChangeEvent {
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct AccessibleEvent {
    pub action: Action,
    pub data: Option<ActionData>,
}
