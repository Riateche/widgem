use {
    super::{Widget, WidgetBaseOf, WidgetExt, WidgetGeometry},
    crate::{
        event::LayoutEvent,
        impl_widget_base,
        key::Key,
        layout::SizeHints,
        types::{PhysicalPixels, PpxSuffix, Rect},
        widgets::NewWidget,
    },
    anyhow::Result,
};

pub struct Stack {
    base: WidgetBaseOf<Self>,
}

impl Stack {
    // TODO: impl explicit rect setting for universal grid layout?
    pub fn add<T: NewWidget>(&mut self, key: Key, rect: Rect, arg: T::Arg) -> &mut T {
        let geometry = self.base.geometry().cloned();
        let widget = self.base.add_child_with_key::<T>(key.clone(), arg);
        if let Some(geometry) = geometry {
            widget.set_geometry(Some(WidgetGeometry::new(&geometry, rect)), &[]);
        }
        self.base.update();
        self.base
            .children
            .get_mut(&key)
            .unwrap()
            .downcast_mut::<T>()
            .unwrap()
    }
}

impl NewWidget for Stack {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Stack {
    impl_widget_base!();

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        Ok(())
    }

    fn handle_size_hint_x_request(&mut self) -> Result<crate::layout::SizeHints> {
        let max = self
            .base
            .children
            .values()
            .filter_map(|c| c.base().rect_in_parent())
            .map(|rect| rect.bottom_right().x())
            .max()
            .unwrap_or(0.ppx());
        Ok(SizeHints {
            min: max,
            preferred: max,
            is_fixed: true,
        })
    }

    fn handle_size_hint_y_request(&mut self, _size_x: PhysicalPixels) -> Result<SizeHints> {
        let max = self
            .base
            .children
            .values()
            .filter_map(|c| c.base().rect_in_parent())
            .map(|rect| rect.bottom_right().y())
            .max()
            .unwrap_or(0.ppx());
        Ok(SizeHints {
            min: max,
            preferred: max,
            is_fixed: true,
        })
    }
}
