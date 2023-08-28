use std::{cell::RefCell, collections::HashMap, rc::Rc};

use widgets::{RawWidgetId, WidgetAddress};

pub mod callback;
pub mod draw;
pub mod event;
pub mod event_loop;
pub mod types;
pub mod widgets;
pub mod window;

pub struct SharedSystemDataInner {
    pub address_book: HashMap<RawWidgetId, WidgetAddress>,
}

#[derive(Clone)]
pub struct SharedSystemData(pub Rc<RefCell<SharedSystemDataInner>>);
