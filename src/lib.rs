use draw::DrawContext;
use event::MouseInputEvent;
use types::Rect;

pub mod callback;
pub mod draw;
pub mod event;
pub mod event_loop;
pub mod types;
pub mod widgets;
pub mod window;

pub struct WidgetCommon {
    //...
}

pub struct Child {
    pub rect: Rect,
    pub widget: Box<dyn Widget>,
}

pub trait Widget {
    fn draw(&mut self, ctx: &mut DrawContext<'_>);
    fn mouse_input(&mut self, event: &mut MouseInputEvent<'_>) {
        let _ = event;
    }
}
