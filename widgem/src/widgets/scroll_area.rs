use {
    super::{scroll_bar::ScrollBar, Widget, WidgetBaseOf, WidgetExt, WidgetGeometry},
    crate::{
        event::{LayoutEvent, MouseScrollEvent},
        impl_widget_base,
        layout::{default_layout, Layout, SizeHint},
        types::{Axis, PhysicalPixels, PpxSuffix, Rect},
        widget_initializer::{self, WidgetInitializer},
    },
    anyhow::Result,
    std::cmp::{max, min},
    widgem_macros::impl_with,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ScrollBarPolicy {
    #[default]
    AsNeeded,
    AlwaysOn,
    AlwaysOff,
}

pub struct ScrollArea {
    base: WidgetBaseOf<Self>,
    x_policy: ScrollBarPolicy,
    y_policy: ScrollBarPolicy,
}

const INDEX_SCROLL_BAR_X: u64 = 0;
const INDEX_SCROLL_BAR_Y: u64 = 1;
const INDEX_VIEWPORT: u64 = 2;

const KEY_CONTENT_IN_VIEWPORT: u64 = 0;

#[impl_with]
impl ScrollArea {
    fn new(mut base: WidgetBaseOf<Self>) -> anyhow::Result<Self> {
        let relayout = base.callback(|this, _| this.relayout());
        base.set_layout(Layout::ExplicitGrid);

        // TODO: icons, localized name
        base.set_child(INDEX_SCROLL_BAR_X, ScrollBar::init(Axis::X))?
            .set_grid_cell(0, 1)
            .on_value_changed(relayout.clone());
        base.set_child(INDEX_SCROLL_BAR_Y, ScrollBar::init(Axis::Y))?
            .set_grid_cell(1, 0)
            .on_value_changed(relayout);
        base.set_child(INDEX_VIEWPORT, Viewport::init())?
            .set_grid_cell(0, 0)
            .set_layout(Layout::ExplicitGrid);
        Ok(ScrollArea {
            base,
            x_policy: ScrollBarPolicy::default(),
            y_policy: ScrollBarPolicy::default(),
        })
    }

    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_fallible_new(Self::new)
    }

    fn has_content(&self) -> bool {
        self.base
            .get_dyn_child(INDEX_VIEWPORT)
            .unwrap()
            .base()
            .has_child(KEY_CONTENT_IN_VIEWPORT)
    }

    pub fn set_content<WI: WidgetInitializer>(
        &mut self,
        initializer: WI,
    ) -> anyhow::Result<&mut WI::Output> {
        self.base
            .get_dyn_child_mut(INDEX_VIEWPORT)
            .unwrap()
            .base_mut()
            .set_child(KEY_CONTENT_IN_VIEWPORT, initializer)
    }

    pub fn remove_content(&mut self) -> &mut Self {
        let _ = self
            .base
            .get_dyn_child_mut(INDEX_VIEWPORT)
            .unwrap()
            .base_mut()
            .remove_child(KEY_CONTENT_IN_VIEWPORT);
        self
    }

    pub fn content<T: Widget>(&self) -> anyhow::Result<&T> {
        self.base
            .get_dyn_child(INDEX_VIEWPORT)
            .unwrap()
            .base()
            .get_child(KEY_CONTENT_IN_VIEWPORT)
    }

    pub fn dyn_content(&self) -> anyhow::Result<&dyn Widget> {
        self.base
            .get_dyn_child(INDEX_VIEWPORT)
            .unwrap()
            .base()
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
    }

    pub fn dyn_content_mut(&mut self) -> anyhow::Result<&mut dyn Widget> {
        self.base
            .get_dyn_child_mut(INDEX_VIEWPORT)
            .unwrap()
            .base_mut()
            .get_dyn_child_mut(KEY_CONTENT_IN_VIEWPORT)
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
        let geometry = self.base.geometry_or_err()?.clone();

        let scroll_x_hint_x = self
            .base
            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?
            .size_hint_x(None);
        let scroll_y_hint_x = self
            .base
            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_Y)?
            .size_hint_x(None);

        let content_hint_x;
        let content_hint_y;
        if let Ok(content) = self
            .base
            .get_dyn_child(INDEX_VIEWPORT)?
            .base()
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
        {
            content_hint_x = content.size_hint_x(None);
            content_hint_y = content.size_hint_y(content_hint_x.preferred());
        } else {
            content_hint_x = SizeHint::new_fixed(0.ppx(), 0.ppx());
            content_hint_y = SizeHint::new_fixed(0.ppx(), 0.ppx());
        };

        let scroll_y_visible = match self.y_policy {
            ScrollBarPolicy::AsNeeded => match self.x_policy {
                ScrollBarPolicy::AlwaysOn | ScrollBarPolicy::AlwaysOff => {
                    let available_size_y = match self.x_policy {
                        ScrollBarPolicy::AsNeeded => unreachable!(),
                        ScrollBarPolicy::AlwaysOn => {
                            let scroll_x_size_y = self
                                .base
                                .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?
                                .size_hint_y(scroll_x_hint_x.preferred())
                                .preferred();
                            min(0.ppx(), geometry.size_y() - scroll_x_size_y)
                        }
                        ScrollBarPolicy::AlwaysOff => 0.ppx(),
                    };
                    content_hint_y.preferred() > available_size_y
                }
                // both x and y are `AsNeeded`
                ScrollBarPolicy::AsNeeded => {
                    // If it fits in both axes, no scroll bars are needed.
                    if content_hint_x.preferred() <= geometry.size_x()
                        && content_hint_y.preferred() <= geometry.size_y()
                    {
                        false
                    } else {
                        // Assuming X scrollbar visible, is Y scrollbar needed?
                        let scroll_x_size_y = self
                            .base
                            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?
                            .size_hint_y(scroll_x_hint_x.preferred())
                            .preferred();
                        let available_size_y = max(0.ppx(), geometry.size_y() - scroll_x_size_y);
                        content_hint_y.preferred() > available_size_y
                    }
                }
            },
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AlwaysOn => true,
        };
        let scroll_x_visible = match self.x_policy {
            ScrollBarPolicy::AsNeeded => {
                let available_size_x = if scroll_y_visible {
                    max(0.ppx(), geometry.size_x() - scroll_y_hint_x.preferred())
                } else {
                    geometry.size_x()
                };
                content_hint_x.preferred() > available_size_x
            }
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
        };

        self.base
            .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_Y)?
            .set_visible(scroll_y_visible);
        self.base
            .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_X)?
            .set_visible(scroll_x_visible);

        default_layout(self);

        if self.has_content() {
            let value_x = self
                .base
                .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)
                .unwrap()
                .value();
            let value_y = self
                .base
                .get_child::<ScrollBar>(INDEX_SCROLL_BAR_Y)
                .unwrap()
                .value();

            let Some(viewport_rect) = self
                .base
                .get_dyn_child_mut(INDEX_VIEWPORT)
                .unwrap()
                .base_mut()
                .rect_in_parent()
            else {
                return Ok(());
            };
            let content_size_hint_x = self
                .base
                .get_dyn_child_mut(INDEX_VIEWPORT)
                .unwrap()
                .base_mut()
                .get_dyn_child_mut(KEY_CONTENT_IN_VIEWPORT)
                .unwrap()
                .size_hint_x(None);
            let content_size_x = if !content_size_hint_x.is_fixed()
                && viewport_rect.size_x() > content_size_hint_x.preferred()
            {
                viewport_rect.size_x()
            } else {
                content_size_hint_x.preferred()
            };
            let content_size_hint_y = self
                .base
                .get_dyn_child_mut(INDEX_VIEWPORT)
                .unwrap()
                .base_mut()
                .get_dyn_child_mut(KEY_CONTENT_IN_VIEWPORT)
                .unwrap()
                .size_hint_y(content_size_x);
            let content_size_y = if !content_size_hint_y.is_fixed()
                && viewport_rect.size_y() > content_size_hint_y.preferred()
            {
                viewport_rect.size_y()
            } else {
                content_size_hint_y.preferred()
            };
            let content_rect = Rect::from_xywh(
                PhysicalPixels::from_i32(-value_x),
                PhysicalPixels::from_i32(-value_y),
                content_size_x,
                content_size_y,
            );
            self.base
                .get_dyn_child_mut(INDEX_VIEWPORT)
                .unwrap()
                .base_mut()
                .get_dyn_child_mut(KEY_CONTENT_IN_VIEWPORT)
                .unwrap()
                .set_geometry(Some(WidgetGeometry::new(&geometry, content_rect)));

            let max_value_x = max(0.ppx(), content_size_x - viewport_rect.size_x());
            let max_value_y = max(0.ppx(), content_size_y - viewport_rect.size_y());
            self.base
                .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_X)
                .unwrap()
                .set_value_range(0..=max_value_x.to_i32());
            self.base
                .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_Y)
                .unwrap()
                .set_value_range(0..=max_value_y.to_i32());
        }
        Ok(())
    }
}

impl Widget for ScrollArea {
    impl_widget_base!();

    fn handle_size_hint_x_request(&self, size_y: Option<PhysicalPixels>) -> Result<SizeHint> {
        let content_hint_x = self
            .base
            .get_dyn_child(INDEX_VIEWPORT)?
            .base()
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
            .map(|content| content.size_hint_x(None))
            .unwrap_or_else(|_| SizeHint::new_fixed(0.ppx(), 0.ppx()));

        let content_hint_y_preferred = self
            .base
            .get_dyn_child(INDEX_VIEWPORT)?
            .base()
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
            .map(|content| content.size_hint_y(content_hint_x.preferred()).preferred())
            .unwrap_or_else(|_| 0.ppx());

        let scroll_x_hint_x = self
            .base
            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?
            .size_hint_x(None);
        let scroll_x_hint_y_preferred = self
            .base
            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?
            .size_hint_y(scroll_x_hint_x.preferred())
            .preferred();

        let scroll_y_hint_x = self
            .base
            .get_child::<ScrollBar>(INDEX_SCROLL_BAR_Y)?
            .size_hint_x(None);

        let scroll_x_min_x = match self.x_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOn => scroll_x_hint_x.min(),
            ScrollBarPolicy::AlwaysOff => 0.ppx(),
        };
        let scroll_y_min_x = match self.y_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOn => scroll_y_hint_x.min(),
            ScrollBarPolicy::AlwaysOff => 0.ppx(),
        };

        let min = scroll_x_min_x + scroll_y_min_x;

        let scroll_x_preferred_x = match self.x_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOff => 0.ppx(),
            ScrollBarPolicy::AlwaysOn => scroll_x_hint_x.preferred(),
        };
        let scroll_y_preferred_x = match self.y_policy {
            ScrollBarPolicy::AsNeeded => {
                if let Some(size_y) = size_y {
                    let scroll_x_preferred_y = match self.x_policy {
                        ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOff => 0.ppx(),
                        ScrollBarPolicy::AlwaysOn => scroll_x_hint_y_preferred,
                    };
                    if content_hint_y_preferred + scroll_x_preferred_y <= size_y {
                        0.ppx()
                    } else {
                        scroll_y_hint_x.preferred()
                    }
                } else {
                    0.ppx()
                }
            }
            ScrollBarPolicy::AlwaysOff => 0.ppx(),
            ScrollBarPolicy::AlwaysOn => scroll_y_hint_x.preferred(),
        };

        let preferred = max(
            content_hint_x.preferred() + scroll_y_preferred_x,
            scroll_x_preferred_x,
        );

        Ok(SizeHint::new_fixed(min, preferred))
    }

    fn handle_size_hint_y_request(&self, size_x: PhysicalPixels) -> Result<SizeHint> {
        let scroll_x = self.base.get_child::<ScrollBar>(INDEX_SCROLL_BAR_X)?;
        let scroll_x_hint_x = scroll_x.size_hint_x(None);

        let scroll_y = self.base.get_child::<ScrollBar>(INDEX_SCROLL_BAR_Y)?;
        let scroll_y_hint_x = scroll_y.size_hint_x(None);

        let scroll_x_min = match self.x_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOn => {
                scroll_x.size_hint_y(scroll_x_hint_x.min()).min()
            }
            ScrollBarPolicy::AlwaysOff => 0.ppx(),
        };
        let scroll_y_min = match self.y_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOn => {
                scroll_y.size_hint_y(scroll_y_hint_x.min()).min()
            }
            ScrollBarPolicy::AlwaysOff => 0.ppx(),
        };
        let min = scroll_x_min + scroll_y_min;

        let content_hint_x;
        let content_hint_y;
        if let Ok(content) = self
            .base
            .get_dyn_child(INDEX_VIEWPORT)?
            .base()
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
        {
            content_hint_x = content.size_hint_x(None);
            content_hint_y = content.size_hint_y(content_hint_x.preferred());
        } else {
            content_hint_x = SizeHint::new_fixed(0.ppx(), 0.ppx());
            content_hint_y = SizeHint::new_fixed(0.ppx(), 0.ppx());
        };

        let scroll_y_visible = match self.y_policy {
            ScrollBarPolicy::AsNeeded | ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AlwaysOn => true,
        };
        let scroll_x_visible = match self.x_policy {
            ScrollBarPolicy::AsNeeded => {
                let available_size_x = if scroll_y_visible {
                    max(0.ppx(), size_x - scroll_y_hint_x.preferred())
                } else {
                    size_x
                };
                content_hint_x.preferred() > available_size_x
            }
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
        };

        let preferred_row1 = if scroll_y_visible {
            max(
                content_hint_y.preferred(),
                scroll_y
                    .size_hint_y(scroll_y_hint_x.preferred())
                    .preferred(),
            )
        } else {
            content_hint_y.preferred()
        };

        let preferred_row2 = if scroll_x_visible {
            scroll_x
                .size_hint_y(scroll_x_hint_x.preferred())
                .preferred()
        } else {
            0.ppx()
        };
        let preferred = preferred_row1 + preferred_row2;

        Ok(SizeHint::new_fixed(min, preferred))
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        self.relayout()
    }

    fn handle_mouse_scroll(&mut self, event: MouseScrollEvent) -> Result<bool> {
        let delta = event.unified_delta(&self.base);

        let scroll_x = self
            .base
            .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_X)
            .unwrap();
        let new_value_x = scroll_x.value() - delta.x.round() as i32;
        scroll_x.set_value(new_value_x.clamp(
            *scroll_x.value_range().start(),
            *scroll_x.value_range().end(),
        ));

        let scroll_y = self
            .base
            .get_child_mut::<ScrollBar>(INDEX_SCROLL_BAR_Y)
            .unwrap();
        let new_value_y = scroll_y.value() - delta.y.round() as i32;
        scroll_y.set_value(new_value_y.clamp(
            *scroll_y.value_range().start(),
            *scroll_y.value_range().end(),
        ));
        self.relayout()?;
        self.base.update();
        Ok(true)
    }
}

pub struct Viewport {
    base: WidgetBaseOf<Self>,
}

impl Viewport {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_new(|base| Viewport { base })
    }
}

impl Widget for Viewport {
    impl_widget_base!();

    fn handle_size_hint_x_request(&self, _size_y: Option<PhysicalPixels>) -> Result<SizeHint> {
        let preferred = self
            .base
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
            .ok()
            .map(|content| content.size_hint_x(None).preferred())
            .unwrap_or(0.ppx());
        Ok(SizeHint::new_expanding(0.ppx(), preferred))
    }
    fn handle_size_hint_y_request(&self, size_x: PhysicalPixels) -> Result<SizeHint> {
        let preferred = self
            .base
            .get_dyn_child(KEY_CONTENT_IN_VIEWPORT)
            .ok()
            .map(|content| content.size_hint_y(size_x).preferred())
            .unwrap_or(0.ppx());
        Ok(SizeHint::new_expanding(0.ppx(), preferred))
    }
}
