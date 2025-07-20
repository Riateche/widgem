mod address;
mod base;
mod button;
mod column;
mod ext;
mod id;
mod image;
mod label;
mod menu;
mod padding_box;
mod root;
mod row;
mod scroll_area;
mod scroll_bar;
mod stack;
mod text_input;
mod widget_trait;
mod window;

pub use self::{
    button::Button, column::Column, image::Image, label::Label, menu::Menu,
    padding_box::PaddingBox, root::RootWidget, row::Row, scroll_area::ScrollArea,
    scroll_bar::ScrollBar, stack::Stack, text_input::TextInput, window::Window,
};

// TODO: move out of this module?
pub use self::{
    address::WidgetAddress,
    base::{EventFilterFn, WidgetBase, WidgetBaseOf, WidgetCreationContext, WidgetGeometry},
    ext::WidgetExt,
    id::{RawWidgetId, WidgetId},
    widget_trait::{NewWidget, Widget},
};

use {crate::system::address, anyhow::Result, log::warn, std::fmt::Debug, thiserror::Error};

#[derive(Debug, Error)]
#[error("widget not found")]
pub struct WidgetNotFound;

pub fn get_widget_by_address_mut<'a>(
    root_widget: &'a mut dyn Widget,
    address: &WidgetAddress,
) -> Result<&'a mut dyn Widget, WidgetNotFound> {
    let root_address = root_widget.base().address();

    if !address.starts_with(root_address) {
        warn!("get_widget_by_address_mut: address is not within root widget");
        return Err(WidgetNotFound);
    }
    let root_address_len = root_address.path.len();
    let mut current_widget = root_widget;
    for (key, _id) in &address.path[root_address_len..] {
        current_widget = current_widget
            .base_mut()
            .children
            .get_mut(key)
            .ok_or(WidgetNotFound)?
            .as_mut();
    }
    Ok(current_widget)
}

pub fn get_widget_by_id_mut(
    root_widget: &mut dyn Widget,
    id: RawWidgetId,
) -> Result<&mut dyn Widget, WidgetNotFound> {
    let address = address(id).ok_or(WidgetNotFound)?;
    get_widget_by_address_mut(root_widget, &address)
}

pub fn invalidate_size_hint_cache(widget: &mut dyn Widget, pending: &[WidgetAddress]) {
    let common = widget.base_mut();
    for pending_addr in pending {
        if pending_addr.starts_with(common.address()) {
            common.clear_size_hint_cache();
            for child in common.children.values_mut() {
                invalidate_size_hint_cache(child.as_mut(), pending);
            }
            return;
        }
    }
}

#[macro_export]
macro_rules! impl_widget_base {
    () => {
        fn type_name() -> &'static str {
            std::any::type_name::<Self>()
        }

        fn base(&self) -> &$crate::WidgetBase {
            self.base.untyped()
        }

        fn base_mut(&mut self) -> &mut $crate::WidgetBase {
            self.base.untyped_mut()
        }
    };
}
