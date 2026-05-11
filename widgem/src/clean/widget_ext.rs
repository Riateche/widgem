use crate::clean::{BoxWidget, Widget};

pub trait WidgetExt: Widget {
    fn boxed(self) -> BoxWidget
    where
        Self: Sized,
    {
        BoxWidget(Box::new(self))
    }
}

impl<W: Widget> WidgetExt for W {}
