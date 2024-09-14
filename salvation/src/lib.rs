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

pub use crate::accessible::new_accessible_node_id;
pub use crate::event_loop::{run, App};
pub use tiny_skia;
pub use winit::window::WindowAttributes;
