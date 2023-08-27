use std::collections::HashSet;

use cosmic_text::FontSystem;
use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

use crate::{draw::Palette, types::Point};

pub struct MouseInputEvent<'a> {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub pressed_mouse_buttons: &'a HashSet<MouseButton>,
    pub pos: Point,

    pub font_system: &'a mut FontSystem,
    pub font_metrics: cosmic_text::Metrics,
    pub palette: &'a mut Palette,
}

pub struct CursorMovedEvent<'a> {
    pub device_id: DeviceId,
    pub modifiers: ModifiersState,
    pub pressed_mouse_buttons: &'a HashSet<MouseButton>,
    pub pos: Point,

    pub font_system: &'a mut FontSystem,
    pub font_metrics: cosmic_text::Metrics,
    pub palette: &'a mut Palette,
}
