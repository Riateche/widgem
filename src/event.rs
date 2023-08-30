use std::collections::HashSet;

use winit::event::{DeviceId, ElementState, Ime, KeyboardInput, ModifiersState, MouseButton};

use crate::types::Point;

pub struct MouseInputEvent<'a> {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub pos: Point,

    // TODO: move to shared data
    pub modifiers: ModifiersState,
    pub pressed_mouse_buttons: &'a HashSet<MouseButton>,
}

pub struct CursorMovedEvent<'a> {
    pub device_id: DeviceId,
    pub pos: Point,

    // TODO: move to shared data
    pub modifiers: ModifiersState,
    pub pressed_mouse_buttons: &'a HashSet<MouseButton>,
}

pub struct KeyboardInputEvent {
    pub device_id: DeviceId,
    pub input: KeyboardInput,
    pub is_synthetic: bool,

    // TODO: move to shared data
    pub modifiers: ModifiersState,
}

pub struct ReceivedCharacterEvent {
    pub char: char,
}

pub struct ImeEvent(pub Ime);
