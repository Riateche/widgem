#![allow(dead_code)]

mod app;
mod app_handler;
mod render;
mod widget_ext;
mod widget_trait;
pub mod widgets;

pub use self::{
    app::{run, App},
    render::WidgetRenderContext,
    widget_ext::WidgetExt,
    widget_trait::{BoxWidget, Widget},
};
