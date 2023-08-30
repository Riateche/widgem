use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use cosmic_text::{FontSystem, SwashCache};
use draw::Palette;
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
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,

    // TODO: per-widget font metrics and palette (as part of the style)
    pub font_metrics: cosmic_text::Metrics,
    pub palette: Palette,
}

#[derive(Clone)]
pub struct SharedSystemData(pub Rc<RefCell<SharedSystemDataInner>>);
