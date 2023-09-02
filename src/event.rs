use std::{cell::Cell, rc::Rc};

use winit::event::{DeviceId, ElementState, Ime, KeyboardInput, MouseButton};

use crate::{
    draw::DrawEvent,
    types::Point,
    widgets::{Geometry, RawWidgetId},
};

use derive_more::From;

#[derive(From)]
pub enum Event {
    MouseInput(MouseInputEvent),
    CursorMoved(CursorMovedEvent),
    KeyboardInput(KeyboardInputEvent),
    ReceivedCharacter(ReceivedCharacterEvent),
    Ime(ImeEvent),
    Draw(DrawEvent),
    GeometryChanged(GeometryChangedEvent),
}

pub struct MouseInputEvent {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub pos: Point,
    pub accepted_by: Rc<Cell<Option<RawWidgetId>>>,
}

pub struct CursorMovedEvent {
    pub device_id: DeviceId,
    pub pos: Point,
}

pub struct KeyboardInputEvent {
    pub device_id: DeviceId,
    pub input: KeyboardInput,
    pub is_synthetic: bool,
}

pub struct ReceivedCharacterEvent {
    pub char: char,
}

pub struct ImeEvent(pub Ime);

#[derive(Clone, Copy)]
pub struct GeometryChangedEvent {
    pub new_geometry: Option<Geometry>,
}
