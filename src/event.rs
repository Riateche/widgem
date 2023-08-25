use winit::event::{DeviceId, ElementState, ModifiersState, MouseButton};

use crate::{draw::Palette, types::Point};

pub struct MouseInputEvent<'a> {
    pub device_id: DeviceId,
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub pos: Point,

    pub font_metrics: cosmic_text::Metrics,
    pub palette: &'a mut Palette,
}
