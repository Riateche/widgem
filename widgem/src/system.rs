use {
    crate::{
        callback::{CallbackId, WidgetCallbackData},
        event_loop::UserEvent,
        shared_window::{WindowId, WindowInfo},
        shortcut::Shortcut,
        style::Style,
        timer::Timers,
        RawWidgetId, WidgetAddress,
    },
    anyhow::Result,
    arboard::Clipboard,
    cosmic_text::{FontSystem, SwashCache},
    std::{collections::HashMap, fmt::Debug, rc::Rc, time::Duration},
    tracing::warn,
    winit::event_loop::EventLoopProxy,
};

#[derive(Debug)]
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
    pub current_layout_state: Option<LayoutState>,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutState {
    pub changed_size_hints: Vec<WidgetAddress>,
}

/// Provides `.or_warn()` method on `Result`.
pub trait OrWarn {
    type Output;

    /// If `self` is `Err`, logs the warning and returns `None`.
    /// Otherwise, returns `Some(value)`.
    fn or_warn(self) -> Option<Self::Output>;
}

impl<T, E> OrWarn for Result<T, E>
where
    E: Debug,
{
    type Output = T;

    fn or_warn(self) -> Option<Self::Output> {
        self.map_err(|error| warn!(?error)).ok()
    }
}
