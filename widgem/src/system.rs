use {
    crate::{
        callback::{CallbackId, WidgetCallbackData},
        event_loop::UserEvent,
        shared_window::{WindowId, WindowInfo},
        shortcut::Shortcut,
        style::Style,
        timer::Timers,
        widgets::{RawWidgetId, WidgetAddress},
    },
    anyhow::Result,
    arboard::Clipboard,
    cosmic_text::{FontSystem, SwashCache},
    std::{
        collections::{HashMap, HashSet},
        rc::Rc,
        time::Duration,
    },
    tracing::warn,
    winit::event_loop::EventLoopProxy,
};

pub struct SystemConfig {
    pub auto_repeat_delay: Duration,
    pub auto_repeat_interval: Duration,
    pub exit_after_last_window_closes: bool,
    pub fixed_scale: Option<f32>,
}

pub struct SharedSystemDataInner {
    pub config: Rc<SystemConfig>,
    pub address_book: HashMap<RawWidgetId, WidgetAddress>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,

    pub style: Style,
    pub(crate) event_loop_proxy: EventLoopProxy<UserEvent>,
    pub timers: Timers,
    pub clipboard: Clipboard,
    pub had_any_windows: bool,
    pub windows: HashMap<WindowId, WindowInfo>,
    pub windows_by_winit_id: HashMap<winit::window::WindowId, WindowInfo>,
    pub widget_callbacks: HashMap<CallbackId, WidgetCallbackData>,
    pub application_shortcuts: Vec<Shortcut>,
    pub pending_children_updates: Vec<WidgetAddress>,
    pub current_children_update: Option<ChildrenUpdateState>,
    pub current_layout_state: Option<LayoutState>,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutState {
    pub changed_size_hints: Vec<WidgetAddress>,
}

#[derive(Debug, Default)]
pub struct ChildrenUpdateState {
    // hashmap key is parent id
    pub num_declared_children: HashMap<RawWidgetId, u32>,
    pub declared_children: HashSet<RawWidgetId>,
}

pub fn report_error(error: impl Into<anyhow::Error>) {
    // TODO: display popup error message or custom hook
    warn!("{:?}", error.into());
}

pub trait ReportError {
    type Output;
    fn or_report_err(self) -> Option<Self::Output>;
}

impl<T, E> ReportError for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    type Output = T;

    fn or_report_err(self) -> Option<Self::Output> {
        self.map_err(|err| report_error(err)).ok()
    }
}
