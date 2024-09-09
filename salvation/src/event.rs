#![allow(clippy::new_without_default)]

pub use crate::draw::DrawEvent;

use std::{cell::Cell, rc::Rc};

use accesskit::{Action, ActionData};
use typed_builder::TypedBuilder;
use winit::{
    event::{DeviceId, ElementState, Ime, KeyEvent, MouseButton},
    keyboard::ModifiersState,
};

use crate::{
    types::{Point, Rect},
    widgets::{RawWidgetId, WidgetAddress, WidgetScope},
};

use derive_more::From;

#[derive(Debug, Clone, From)]
pub enum Event {
    MouseInput(MouseInputEvent),
    MouseEnter(MouseEnterEvent),
    MouseMove(MouseMoveEvent),
    MouseLeave(MouseLeaveEvent),
    KeyboardInput(KeyboardInputEvent),
    Ime(ImeEvent),
    Draw(DrawEvent),
    Layout(LayoutEvent),
    FocusIn(FocusInEvent),
    FocusOut(FocusOutEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    Accessible(AccessibleActionEvent),
    WidgetScopeChange(WidgetScopeChangeEvent),
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct MouseInputEvent {
    device_id: DeviceId,
    state: ElementState,
    button: MouseButton,
    num_clicks: u32,
    // pos in current widget coordinates
    pos: Point,
    pos_in_window: Point,
    pub(crate) accepted_by: Rc<Cell<Option<RawWidgetId>>>,
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

    pub fn pos_in_window(&self) -> Point {
        self.pos_in_window
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
pub struct MouseMoveEvent {
    device_id: DeviceId,
    pos: Point,
    pos_in_window: Point,
    // TODO: avoid?
    pub(crate) accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

impl MouseMoveEvent {
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn pos_in_window(&self) -> Point {
        self.pos_in_window
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

    pub fn create_enter_event(&self) -> MouseEnterEvent {
        MouseEnterEvent {
            device_id: self.device_id,
            pos: self.pos,
            accepted_by: self.accepted_by.clone(),
        }
    }
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct MouseEnterEvent {
    device_id: DeviceId,
    pos: Point,
    pub(crate) accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

impl MouseEnterEvent {
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn pos(&self) -> Point {
        self.pos
    }
}

#[derive(Debug, Clone)]
pub struct MouseLeaveEvent(());

impl MouseLeaveEvent {
    pub fn new() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone, TypedBuilder)]
pub struct KeyboardInputEvent {
    device_id: DeviceId,
    info: KeyEvent,
    is_synthetic: bool,
    modifiers: ModifiersState,
}

impl KeyboardInputEvent {
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn info(&self) -> &KeyEvent {
        &self.info
    }

    pub fn is_synthetic(&self) -> bool {
        self.is_synthetic
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }
}

#[derive(Debug, Clone)]
pub struct ImeEvent {
    info: Ime,
}

impl ImeEvent {
    pub fn new(info: Ime) -> Self {
        Self { info }
    }

    pub fn info(&self) -> &Ime {
        &self.info
    }
}

#[derive(Debug, Clone)]
pub struct LayoutEvent {
    // None means widget is hidden
    pub(crate) new_rect_in_window: Option<Rect>,
    pub(crate) changed_size_hints: Vec<WidgetAddress>,
}

impl LayoutEvent {
    pub(crate) fn new(
        new_rect_in_window: Option<Rect>,
        changed_size_hints: Vec<WidgetAddress>,
    ) -> Self {
        Self {
            new_rect_in_window,
            changed_size_hints,
        }
    }

    pub(crate) fn size_hints_changed_within(&self, addr: &WidgetAddress) -> bool {
        self.changed_size_hints
            .iter()
            .any(|changed| changed.starts_with(addr))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusReason {
    Mouse,
    Tab,
    /// A widget was automatically focused because there was no focused widget previously.
    Auto,
}

#[derive(Debug, Clone)]
pub struct FocusInEvent {
    reason: FocusReason,
}

impl FocusInEvent {
    pub fn new(reason: FocusReason) -> Self {
        Self { reason }
    }

    pub fn reason(&self) -> FocusReason {
        self.reason
    }
}

#[derive(Debug, Clone)]
pub struct FocusOutEvent(());

impl FocusOutEvent {
    pub fn new() -> Self {
        Self(())
    }
}

#[derive(Debug, Clone)]
pub struct WindowFocusChangeEvent {
    is_focused: bool,
}

impl WindowFocusChangeEvent {
    pub fn new(focused: bool) -> Self {
        Self {
            is_focused: focused,
        }
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused
    }
}

#[derive(Debug, Clone)]
pub struct AccessibleActionEvent {
    action: Action,
    data: Option<ActionData>,
}

impl AccessibleActionEvent {
    pub fn new(action: Action, data: Option<ActionData>) -> Self {
        Self { action, data }
    }

    pub fn action(&self) -> Action {
        self.action
    }

    pub fn data(&self) -> Option<&ActionData> {
        self.data.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct WidgetScopeChangeEvent {
    previous_scope: WidgetScope,
}

impl WidgetScopeChangeEvent {
    pub fn new(previous_scope: WidgetScope) -> Self {
        Self { previous_scope }
    }

    pub fn previous_scope(&self) -> &WidgetScope {
        &self.previous_scope
    }
}
