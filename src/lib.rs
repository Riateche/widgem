use draw::DrawContext;
use types::Rect;

pub mod draw;
pub mod event_loop;
pub mod types;
pub mod widgets;

pub struct WidgetCommon {
    //...
}

pub struct Child {
    pub rect: Rect,
    pub widget: Box<dyn Widget>,
}

pub trait Widget {
    fn draw(&mut self, ctx: &mut DrawContext<'_>);
}
