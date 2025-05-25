use {
    super::{RawWidgetId, Widget, WidgetCommon, WidgetCommonTyped},
    crate::{
        event::LayoutEvent, impl_widget_common, key::Key, layout::SizeHints, system::ReportError,
        types::Rect,
    },
    anyhow::Result,
    std::collections::HashMap,
};

pub struct Stack {
    common: WidgetCommonTyped<Self>,
    rects: HashMap<RawWidgetId, Option<Rect>>,
}

impl Stack {
    // TODO: impl explicit rect setting for universal grid layout?
    pub fn add<T: Widget>(&mut self, key: Key, rect: Rect) -> &mut T {
        let widget = self.common.add_child_with_key::<T>(key.clone());
        let id = widget.common().id;
        self.common
            .set_child_rect(key.clone(), Some(rect))
            .or_report_err();
        self.rects.insert(id, Some(rect));
        self.common.update();
        self.common
            .children
            .get_mut(&key)
            .unwrap()
            .widget
            .downcast_mut::<T>()
            .unwrap()
    }
}

impl Widget for Stack {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common,
            rects: HashMap::new(),
        }
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        Ok(())
    }

    fn recalculate_size_hint_x(&mut self) -> Result<crate::layout::SizeHints> {
        let max = self
            .common
            .children
            .values()
            .filter_map(|c| c.widget.common().rect_in_parent)
            .map(|rect| rect.bottom_right().x)
            .max()
            .unwrap_or(0);
        Ok(SizeHints {
            min: max,
            preferred: max,
            is_fixed: true,
        })
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32) -> Result<SizeHints> {
        let max = self
            .common
            .children
            .values()
            .filter_map(|c| c.widget.common().rect_in_parent)
            .map(|rect| rect.bottom_right().y)
            .max()
            .unwrap_or(0);
        Ok(SizeHints {
            min: max,
            preferred: max,
            is_fixed: true,
        })
    }
}
