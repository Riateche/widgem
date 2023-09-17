use std::{cell::Cell, rc::Rc};

use accesskit::{Action, ActionData};
use winit::{
    event::{DeviceId, ElementState, Ime, KeyEvent, MouseButton},
    keyboard::ModifiersState,
};

use crate::{
    draw::DrawEvent,
    types::Point,
    widgets::{Geometry, MountPoint, RawWidgetId},
};

use derive_more::From;

#[derive(From)]
pub enum Event {
    MouseInput(MouseInputEvent),
    CursorMoved(CursorMovedEvent),
    KeyboardInput(KeyboardInputEvent),
    Ime(ImeEvent),
    Draw(DrawEvent),
    GeometryChanged(GeometryChangedEvent),
    Mount(MountEvent),
    Unmount(UnmountEvent),
    FocusIn(FocusInEvent),
    FocusOut(FocusOutEvent),
    WindowFocusChanged(WindowFocusChangedEvent),
    Accessible(AccessibleEvent),
}

pub struct MouseInputEvent {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub num_clicks: u32,
    pub pos: Point,
    pub accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

pub struct CursorMovedEvent {
    pub device_id: DeviceId,
    pub pos: Point,
    pub accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

#[derive(Debug)]
pub struct KeyboardInputEvent {
    pub device_id: DeviceId,
    pub event: KeyEvent,
    pub is_synthetic: bool,
    pub modifiers: ModifiersState,
}

pub struct ImeEvent(pub Ime);

#[derive(Clone, Copy)]
pub struct GeometryChangedEvent {
    pub new_geometry: Option<Geometry>,
}

pub struct MountEvent(pub MountPoint);

pub struct UnmountEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum FocusReason {
    Mouse,
    Tab,
    /// A widget was automatically focused because there was no focused widget previously.
    Auto,
}

pub struct FocusInEvent {
    pub reason: FocusReason,
}

pub struct FocusOutEvent;

#[derive(Clone)]
pub struct WindowFocusChangedEvent {
    pub focused: bool,
}

pub struct AccessibleEvent {
    pub action: Action,
    pub data: Option<ActionData>,
}
