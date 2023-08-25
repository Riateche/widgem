use draw::DrawContext;
use types::Rect;

pub mod draw;
pub mod event_loop;
pub mod types;
pub mod widgets;

pub struct WidgetInfo {
    pub rect: Rect,
}

pub trait Widget {
    fn draw(&mut self, ctx: &mut DrawContext<'_>);
}

pub struct WidgetContainer {
    pub info: WidgetInfo,
    pub widget: Box<dyn Widget>,
}
