#![allow(clippy::new_without_default)]

pub use crate::draw::DrawEvent;

use {
    crate::{
        types::{Point, Rect},
        WidgetBase, WidgetGeometry,
    },
    accesskit::{Action, ActionData},
    derive_more::From,
    winit::{
        dpi::PhysicalPosition,
        event::{DeviceId, ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase},
        keyboard::ModifiersState,
    },
};

#[derive(Debug, Clone, From)]
pub enum Event {
    MouseInput(MouseInputEvent),
    MouseScroll(MouseScrollEvent),
    MouseEnter(MouseEnterEvent),
    MouseMove(MouseMoveEvent),
    MouseLeave(MouseLeaveEvent),
    KeyboardInput(KeyboardInputEvent),
    InputMethod(InputMethodEvent),
    Draw(DrawEvent),
    Layout(LayoutEvent),
    FocusIn(FocusInEvent),
    FocusOut(FocusOutEvent),
    // TODO: use a callback instead
    WindowFocusChange(WindowFocusChangeEvent),
    AccessibilityAction(AccessibilityActionEvent),
    StyleChange(StyleChangeEvent),
    Activate(ActivateEvent),
}

#[derive(Debug, Clone)]
pub struct MouseInputEvent {
    pub(crate) device_id: DeviceId,
    pub(crate) state: ElementState,
    pub(crate) button: MouseButton,
    pub(crate) num_clicks: u32,
    /// Position in widget coordinates
    pub(crate) pos: Point,
    pub(crate) pos_in_window: Point,
}

impl MouseInputEvent {
    pub fn map_to_child(&self, rect_in_parent: Rect, force: bool) -> Option<Self> {
        if force || rect_in_parent.contains(self.pos) {
            let mut event = self.clone();
            event.pos -= rect_in_parent.top_left();
            Some(event)
        } else {
            None
        }
    }

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
}

#[derive(Debug, Clone)]
pub struct MouseScrollEvent {
    pub(crate) device_id: DeviceId,
    pub(crate) delta: MouseScrollDelta,
    pub(crate) touch_phase: TouchPhase,
    /// Position in widget coordinates
    pub(crate) pos: Point,
    pub(crate) pos_in_window: Point,
}

impl MouseScrollEvent {
    pub fn map_to_child(&self, rect_in_parent: Rect, force: bool) -> Option<Self> {
        if force || rect_in_parent.contains(self.pos) {
            let mut event = self.clone();
            event.pos -= rect_in_parent.top_left();
            Some(event)
        } else {
            None
        }
    }

    pub fn unified_delta(&self, widget_common: &WidgetBase) -> PhysicalPosition<f64> {
        match self.delta {
            MouseScrollDelta::LineDelta(dx, dy) => {
                let line_height = widget_common.base_style().font_metrics.line_height;
                PhysicalPosition::new((line_height * dx).into(), (line_height * dy).into())
            }
            MouseScrollDelta::PixelDelta(delta) => delta,
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn delta(&self) -> MouseScrollDelta {
        self.delta
    }

    pub fn touch_phase(&self) -> TouchPhase {
        self.touch_phase
    }

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn pos_in_window(&self) -> Point {
        self.pos_in_window
    }
}

#[derive(Debug, Clone)]
pub struct MouseMoveEvent {
    pub(crate) device_id: DeviceId,
    /// Position in widget coordinates
    pub(crate) pos: Point,
    pub(crate) pos_in_window: Point,
}

impl MouseMoveEvent {
    pub fn map_to_child(&self, rect_in_parent: Rect, force: bool) -> Option<Self> {
        if force || rect_in_parent.contains(self.pos) {
            let mut event = self.clone();
            event.pos -= rect_in_parent.top_left();
            Some(event)
        } else {
            None
        }
    }

    pub fn create_enter_event(&self) -> MouseEnterEvent {
        MouseEnterEvent {
            device_id: self.device_id,
            pos: self.pos,
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn pos(&self) -> Point {
        self.pos
    }

    pub fn pos_in_window(&self) -> Point {
        self.pos_in_window
    }
}

#[derive(Debug, Clone)]
pub struct MouseEnterEvent {
    pub(crate) device_id: DeviceId,
    pub(crate) pos: Point,
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
pub struct MouseLeaveEvent {
    pub(crate) _empty: (),
}

#[derive(Debug, Clone)]
pub struct KeyboardInputEvent {
    pub(crate) device_id: DeviceId,
    pub(crate) info: KeyEvent,
    pub(crate) is_synthetic: bool,
    pub(crate) modifiers: ModifiersState,
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
pub struct InputMethodEvent {
    pub(crate) info: Ime,
}

impl InputMethodEvent {
    pub fn info(&self) -> &Ime {
        &self.info
    }
}

#[derive(Debug, Clone)]
pub struct LayoutEvent {
    // None means widget is hidden
    pub(crate) new_geometry: Option<WidgetGeometry>,
}

impl LayoutEvent {
    pub fn new_geometry(&self) -> Option<&WidgetGeometry> {
        self.new_geometry.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FocusReason {
    Mouse,
    Tab,
    Accessibility,
    /// A widget was automatically focused because there was no focused widget previously.
    Auto,
}

#[derive(Debug, Clone)]
pub struct FocusInEvent {
    pub(crate) reason: FocusReason,
}

impl FocusInEvent {
    pub fn reason(&self) -> FocusReason {
        self.reason
    }
}

#[derive(Debug, Clone)]
pub struct FocusOutEvent {
    pub(crate) _empty: (),
}

#[derive(Debug, Clone)]
pub struct WindowFocusChangeEvent {
    pub(crate) is_window_focused: bool,
}

impl WindowFocusChangeEvent {
    pub fn is_window_focused(&self) -> bool {
        self.is_window_focused
    }
}

#[derive(Debug, Clone)]
pub struct AccessibilityActionEvent {
    pub(crate) action: Action,
    pub(crate) data: Option<ActionData>,
}

impl AccessibilityActionEvent {
    pub fn action(&self) -> Action {
        self.action
    }

    pub fn data(&self) -> Option<&ActionData> {
        self.data.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct StyleChangeEvent {
    pub(crate) _empty: (),
}

#[derive(Debug, Clone)]
pub struct ActivateEvent {
    pub(crate) _empty: (),
}
