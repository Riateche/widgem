use {
    super::{scroll_bar::ScrollBar, Widget, WidgetCommon, WidgetCommonTyped, WidgetExt},
    crate::{
        event::{LayoutEvent, MouseScrollEvent},
        impl_widget_common,
        layout::{
            grid::{self, GridOptions},
            SizeHintMode,
        },
        types::{Axis, Rect},
    },
    anyhow::Result,
    salvation_macros::impl_with,
    std::cmp::max,
};

pub struct ScrollArea {
    common: WidgetCommonTyped<Self>,
}

const INDEX_SCROLL_BAR_X: u64 = 0;
const INDEX_SCROLL_BAR_Y: u64 = 1;
const INDEX_VIEWPORT: u64 = 2;

#[impl_with]
impl ScrollArea {
    fn has_content(&self) -> bool {
        !self
            .common
            .children
            .get(&INDEX_VIEWPORT)
            .unwrap()
            .widget
            .common()
            .children
            .is_empty()
    }

    // TODO: naming?
    // TODO: remove old content
    pub fn add_content<T: Widget>(&mut self) -> &mut T {
        assert!(!self.has_content());
        self.common
            .children
            .get_mut(&INDEX_VIEWPORT)
            .unwrap()
            .widget
            .common_mut()
            .add_child::<T>(0)
    }

    // pub fn set_content(&mut self, content: Box<dyn Widget>) {
    //     if self.has_content() {
    //         self.common.children[INDEX_VIEWPORT]
    //             .widget
    //             .common_mut()
    //             .remove_child(0)
    //             .unwrap();
    //     }
    //     self.common.children[INDEX_VIEWPORT]
    //         .widget
    //         .common_mut()
    //         .add_child(content, LayoutItemOptions::default());
    // }
    // TODO: take_content; default impl for empty scroll area

    // pub fn on_value_changed(&mut self, callback: Callback<i32>) {
    //     self.value_changed = Some(callback);
    // }

    // fn size_hints(&mut self) -> SizeHints {
    //     let xscroll_x = self.common.children[0].widget.cached_size_hint_x();
    //     let yscroll_x = self.common.children[1].widget.cached_size_hint_x();
    //     let content_x = if let Some(child) = self.common.children.get(2) {
    //         widget.cached_size_hint_x()
    //     } else {
    //         SizeHint::new_fallback()
    //     };

    //     let xscroll_y = self.common.children[0]
    //         .widget
    //         .cached_size_hint_y(xscroll_x.preferred);
    //     let yscroll_y = self.common.children[1]
    //         .widget
    //         .cached_size_hint_y(yscroll_x.preferred);
    //     let content_y = self.common.children[2]
    //         .widget
    //         .cached_size_hint_y(content_x.preferred);
    //     SizeHints {
    //         xscroll_x,
    //         yscroll_x,
    //         content_x,
    //         xscroll_y: xscroll_y,
    //         yscroll_y,
    //         content_y,
    //     }
    // }

    fn relayout(&mut self) -> Result<()> {
        let options = self.common.grid_options();
        let size = self.common.size_or_err()?;
        let rects = grid::layout(&mut self.common.children, &options, size)?;
        self.common.set_child_rects(&rects)?;

        if self.has_content() {
            let value_x = self
                .common
                .children
                .get(&INDEX_SCROLL_BAR_X)
                .unwrap()
                .widget
                .downcast_ref::<ScrollBar>()
                .unwrap()
                .value();
            let value_y = self
                .common
                .children
                .get(&INDEX_SCROLL_BAR_Y)
                .unwrap()
                .widget
                .downcast_ref::<ScrollBar>()
                .unwrap()
                .value();

            let viewport_rect = *rects.get(&INDEX_VIEWPORT).unwrap();
            let content_size_x = self
                .common
                .children
                .get_mut(&INDEX_VIEWPORT)
                .unwrap()
                .widget
                .common_mut()
                .children
                .get_mut(&0)
                .unwrap()
                .widget
                .size_hint_x(SizeHintMode::Preferred);
            let content_size_y = self
                .common
                .children
                .get_mut(&INDEX_VIEWPORT)
                .unwrap()
                .widget
                .common_mut()
                .children
                .get_mut(&0)
                .unwrap()
                .widget
                .size_hint_y(content_size_x, SizeHintMode::Preferred);
            let content_rect = Rect::from_xywh(-value_x, -value_y, content_size_x, content_size_y);
            self.common
                .children
                .get_mut(&INDEX_VIEWPORT)
                .unwrap()
                .widget
                .common_mut()
                .set_child_rect(0, Some(content_rect))?;

            let max_value_x = max(0, content_size_x - viewport_rect.size.x);
            let max_value_y = max(0, content_size_y - viewport_rect.size.y);
            self.common
                .children
                .get_mut(&INDEX_SCROLL_BAR_X)
                .unwrap()
                .widget
                .downcast_mut::<ScrollBar>()
                .unwrap()
                .set_value_range(0..=max_value_x);
            self.common
                .children
                .get_mut(&INDEX_SCROLL_BAR_Y)
                .unwrap()
                .widget
                .downcast_mut::<ScrollBar>()
                .unwrap()
                .set_value_range(0..=max_value_y);
        }
        Ok(())
    }
}

impl Widget for ScrollArea {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let relayout = common.callback(|this, _| this.relayout());

        // TODO: icons, localized name
        common
            .add_child::<ScrollBar>(INDEX_SCROLL_BAR_X)
            .set_column(0)
            .set_row(1)
            .on_value_changed(relayout.clone());
        common
            .add_child::<ScrollBar>(INDEX_SCROLL_BAR_Y)
            .set_column(1)
            .set_row(0)
            .set_axis(Axis::Y)
            .on_value_changed(relayout);
        common
            .add_child::<Viewport>(INDEX_VIEWPORT)
            .set_column(0)
            .set_row(0);
        common.set_grid_options(Some(GridOptions::ZERO));
        Self { common }
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        self.relayout()
    }

    fn handle_mouse_scroll(&mut self, event: MouseScrollEvent) -> Result<bool> {
        let delta = event.unified_delta(&self.common);

        let scroll_x = self
            .common
            .children
            .get_mut(&INDEX_SCROLL_BAR_X)
            .unwrap()
            .widget
            .downcast_mut::<ScrollBar>()
            .unwrap();
        let new_value_x = scroll_x.value() - delta.x.round() as i32;
        scroll_x.set_value(new_value_x.clamp(
            *scroll_x.value_range().start(),
            *scroll_x.value_range().end(),
        ));

        let scroll_y = self
            .common
            .children
            .get_mut(&INDEX_SCROLL_BAR_Y)
            .unwrap()
            .widget
            .downcast_mut::<ScrollBar>()
            .unwrap();
        let new_value_y = scroll_y.value() - delta.y.round() as i32;
        scroll_y.set_value(new_value_y.clamp(
            *scroll_y.value_range().start(),
            *scroll_y.value_range().end(),
        ));
        Ok(true)
    }
}

pub struct Viewport {
    common: WidgetCommonTyped<Self>,
}

impl Viewport {}

impl Widget for Viewport {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self { common }
    }

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        false
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        false
    }
}
