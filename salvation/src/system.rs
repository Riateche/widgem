use {
    crate::{
        callback::{Callback, CallbackId, WidgetCallbackData},
        event_loop::UserEvent,
        shortcut::Shortcut,
        style::computed::ComputedStyle,
        timer::{Timer, TimerId, Timers},
        widgets::{RawWidgetId, WidgetAddress},
        window::WindowRequest,
        window_handler::WindowInfo,
    },
    anyhow::Result,
    arboard::Clipboard,
    cosmic_text::{FontSystem, SwashCache},
    log::warn,
    std::{
        cell::RefCell,
        collections::HashMap,
        time::{Duration, Instant},
    },
    winit::{event_loop::EventLoopProxy, window::WindowId},
};

thread_local! {
    pub static SYSTEM: SharedSystemData = const { SharedSystemData(RefCell::new(None)) };
}

pub struct SystemConfig {
    pub auto_repeat_delay: Duration,
    pub auto_repeat_interval: Duration,
    pub exit_after_last_window_closes: bool,
}

pub struct SharedSystemDataInner {
    pub config: SystemConfig,
    pub address_book: HashMap<RawWidgetId, WidgetAddress>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,

    pub default_style: ComputedStyle,
    pub(crate) event_loop_proxy: EventLoopProxy<UserEvent>,
    pub timers: Timers,
    pub clipboard: Clipboard,
    pub windows: HashMap<WindowId, WindowInfo>,
    pub widget_callbacks: HashMap<CallbackId, WidgetCallbackData>,
    pub application_shortcuts: Vec<Shortcut>,
}

pub struct SharedSystemData(pub RefCell<Option<SharedSystemDataInner>>);

const EMPTY_ERR: &str = "system not initialized yet";

pub fn address(id: RawWidgetId) -> Option<WidgetAddress> {
    with_system(|system| system.address_book.get(&id).cloned())
}

pub fn register_address(id: RawWidgetId, address: WidgetAddress) -> Option<WidgetAddress> {
    with_system(|system| system.address_book.insert(id, address))
}

pub fn unregister_address(id: RawWidgetId) -> Option<WidgetAddress> {
    with_system(|system| system.address_book.remove(&id))
}

pub fn with_system<R>(f: impl FnOnce(&mut SharedSystemDataInner) -> R) -> R {
    SYSTEM.with(|system| f(system.0.borrow_mut().as_mut().expect(EMPTY_ERR)))
}

pub fn send_window_request(window_id: WindowId, request: impl Into<WindowRequest>) {
    with_system(|system| {
        let _ = system
            .event_loop_proxy
            .send_event(UserEvent::WindowRequest(window_id, request.into()));
    });
}

pub fn add_timer(duration: Duration, callback: Callback<Instant>) -> TimerId {
    add_timer_or_interval(duration, None, callback)
}

pub fn add_interval(interval: Duration, callback: Callback<Instant>) -> TimerId {
    add_timer_or_interval(interval, Some(interval), callback)
}

pub fn add_timer_or_interval(
    duration: Duration,
    interval: Option<Duration>,
    callback: Callback<Instant>,
) -> TimerId {
    with_system(|system| {
        system
            .timers
            .add(Instant::now() + duration, Timer { interval, callback })
    })
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
