#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::comparison_chain)]

mod accessible;
mod callback;
mod draw;
pub mod event;
mod event_loop;
pub mod layout;
pub mod shortcut;
pub mod style;
pub mod system;
pub mod text_editor;
pub mod timer;
pub mod types;
pub mod widgets;
pub mod window;
mod window_handler;

pub use {
    crate::accessible::new_accessible_node_id,
    crate::event_loop::{run, App},
    crate::window_handler::create_window,
    tiny_skia, winit,
    winit::window::WindowAttributes,
};

use event_loop::with_active_event_loop;

pub fn exit() {
    with_active_event_loop(|event_loop| event_loop.exit());
}
