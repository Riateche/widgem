#![allow(dead_code)]

mod render;
mod widget_ext;
mod widget_trait;
pub mod widgets;

pub use self::{
    widget_ext::WidgetExt,
    widget_trait::{BoxWidget, Widget},
};
