#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::comparison_chain)]

mod accessibility;
mod address;
mod app;
mod app_builder;
mod callback;
mod child_key;
mod draw;
pub mod event;
mod event_loop;
mod id;
pub mod items;
pub mod layout;
mod monitor;
mod pixmap;
pub mod shared_window;
pub mod shortcut;
pub mod style;
pub mod system;
mod text;
pub mod timer;
pub mod types;
mod widget_base;
mod widget_ext;
pub mod widget_initializer;
mod widget_trait;
pub mod widgets;
mod window_handler;

use {
    crate::types::{Point, Rect, Size},
    winit::monitor::MonitorHandle,
};

pub use {
    crate::{
        accessibility::new_accessibility_node_id,
        address::WidgetAddress,
        app::App,
        app_builder::{run, AppBuilder},
        callback::Callback,
        child_key::ChildKey,
        id::{RawWidgetId, WidgetId},
        monitor::MonitorExt,
        pixmap::Pixmap,
        widget_base::{EventFilterFn, WidgetBase, WidgetBaseOf, WidgetGeometry},
        widget_ext::WidgetExt,
        widget_trait::Widget,
    },
    widget_initializer::WidgetInitializer,
    widgets::{WidgetNotFound, Window},
};

#[derive(Debug, Clone)]
pub struct ScrollToRectRequest {
    pub address: WidgetAddress,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct WindowRectRequest {
    pub suggested_position: Option<Point>,
    pub suggested_size: Size,
    pub monitor: Option<MonitorHandle>,
}

#[derive(Debug, Clone)]
pub struct WindowRectResponse {
    pub position: Option<Point>,
    pub size: Option<Size>,
}
