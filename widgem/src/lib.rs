#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::comparison_chain)]

mod accessibility;
mod app;
mod app_builder;
mod callback;
mod child_key;
mod draw;
pub mod event;
mod event_loop;
pub mod layout;
mod monitor;
mod pixmap;
pub mod shared_window;
pub mod shortcut;
pub mod style;
pub mod system;
mod text;
pub mod text_editor;
pub mod timer;
pub mod types;
pub mod widgets;
mod window_handler;

use winit::monitor::MonitorHandle;

use crate::types::{Point, Rect, Size};

pub use {
    crate::{
        accessibility::new_accessibility_node_id,
        app::App,
        app_builder::{run, AppBuilder},
        child_key::ChildKey,
        monitor::MonitorExt,
        pixmap::Pixmap,
    },
    widgets::{
        RawWidgetId, Widget, WidgetAddress, WidgetBase, WidgetBaseOf, WidgetExt, WidgetGeometry,
        WidgetId, WidgetNotFound, Window,
    },
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
