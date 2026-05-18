#![allow(dead_code)]

mod app;
mod app_handler;
mod render;
mod widget_context;
mod widget_ext;
mod widget_trait;
mod widget_tree_item;
pub mod widgets;

pub use self::{
    app::{run, App},
    render::WidgetRenderContext,
    widget_context::WidgetContext,
    widget_ext::WidgetExt,
    widget_trait::{BoxWidget, Widget, WidgetAuto},
};

pub(crate) use self::widget_tree_item::WidgetTreeItem;
