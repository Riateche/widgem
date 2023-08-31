use winit::event::{DeviceId, ElementState, Ime, KeyboardInput, MouseButton};

use crate::types::Point;

pub struct MouseInputEvent {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub pos: Point,
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
