#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::comparison_chain)]

mod accessibility;
mod callback;
mod draw;
pub mod event;
mod event_loop;
pub mod key;
pub mod layout;
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

use event_loop::with_active_event_loop;
pub use {
    crate::{
        accessibility::new_accessibility_node_id,
        event_loop::{run, App},
    },
    tiny_skia,
    widgets::{
        EventFilterFn, RawWidgetId, Widget, WidgetAddress, WidgetBase, WidgetBaseOf, WidgetExt,
        WidgetGeometry, WidgetId, WidgetNotFound, Window,
    },
    winit::{self, window::WindowAttributes},
};

pub fn exit() {
    with_active_event_loop(|event_loop| event_loop.exit());
}
