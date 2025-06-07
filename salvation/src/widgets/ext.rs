use {
    super::{common::WidgetGeometry, Widget, WidgetAddress, WidgetId},
    crate::{
        callback::Callback,
        event::{Event, ScrollToRectRequest},
        layout::SizeHints,
        style::{css::MyPseudoClass, Style},
        types::PhysicalPixels,
    },
    anyhow::Result,
    std::{borrow::Cow, rc::Rc},
};

pub trait WidgetExt {
    fn id(&self) -> WidgetId<Self>
    where
        Self: Sized;

    fn callback<F, E>(&self, func: F) -> Callback<E>
    where
        F: Fn(&mut Self, E) -> Result<()> + 'static,
        E: 'static,
        Self: Sized;

    fn set_no_padding(&mut self, no_padding: bool) -> &mut Self;
    fn set_visible(&mut self, value: bool) -> &mut Self;
    fn set_focusable(&mut self, value: bool) -> &mut Self;
    fn set_accessible(&mut self, value: bool) -> &mut Self;
    fn set_row(&mut self, row: i32) -> &mut Self;
    fn set_column(&mut self, column: i32) -> &mut Self;
    fn set_size_x_fixed(&mut self, fixed: bool) -> &mut Self;
    fn set_size_y_fixed(&mut self, fixed: bool) -> &mut Self;

    fn dispatch(&mut self, event: Event) -> bool;
    fn update_accessible(&mut self);

    fn update_children(&mut self);
    fn size_hint_x(&mut self) -> SizeHints;
    fn size_hint_y(&mut self, size_x: PhysicalPixels) -> SizeHints;

    // TODO: private
    fn set_enabled(&mut self, enabled: bool) -> &mut Self;
    fn set_style(&mut self, style: Option<Rc<Style>>) -> Result<()>;

    fn add_class(&mut self, class: Cow<'static, str>) -> &mut Self;
    fn remove_class(&mut self, class: Cow<'static, str>) -> &mut Self;
    fn has_class(&self, class: &str) -> bool;
    fn set_class(&mut self, class: Cow<'static, str>, present: bool) -> &mut Self;
    fn add_pseudo_class(&mut self, class: MyPseudoClass) -> &mut Self;
    fn remove_pseudo_class(&mut self, class: MyPseudoClass) -> &mut Self;
    fn has_pseudo_class(&self, class: MyPseudoClass) -> bool;
    fn set_pseudo_class(&mut self, class: MyPseudoClass, present: bool) -> &mut Self;

    fn set_geometry(
        &mut self,
        geometry: Option<WidgetGeometry>,
        changed_size_hints: &[WidgetAddress],
    );

    fn scroll_to_rect(&mut self, request: ScrollToRectRequest) -> bool;

    fn boxed(self) -> Box<dyn Widget>
    where
        Self: Sized;
}
