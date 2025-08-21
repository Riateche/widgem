#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::comparison_chain)]

mod accessibility;
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

use crate::types::Rect;
use event_loop::with_active_event_loop;

pub use {
    crate::{
        accessibility::new_accessibility_node_id,
        child_key::ChildKey,
        event_loop::{run, App},
        monitor::MonitorExt,
        pixmap::Pixmap,
    },
    widgets::{
        RawWidgetId, Widget, WidgetAddress, WidgetBase, WidgetBaseOf, WidgetExt, WidgetGeometry,
        WidgetId, WidgetNotFound, Window,
    },
};

pub fn exit() {
    with_active_event_loop(|event_loop| event_loop.exit());
}

#[derive(Debug, Clone)]
pub struct ScrollToRectRequest {
    pub address: WidgetAddress,
    pub rect: Rect,
}
