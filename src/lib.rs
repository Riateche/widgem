use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use widgets::{RawWidgetId, WidgetAddress};
use winit::window::WindowId;

pub mod callback;
pub mod draw;
pub mod event;
pub mod event_loop;
pub mod types;
pub mod widgets;
pub mod window;

pub struct SharedSystemDataInner {
    pub address_book: HashMap<RawWidgetId, WidgetAddress>,
    pub widget_tree_changed_flags: HashSet<WindowId>,
}

#[derive(Clone)]
pub struct SharedSystemData(pub Rc<RefCell<SharedSystemDataInner>>);
