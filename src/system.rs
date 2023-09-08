use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use cosmic_text::{FontSystem, SwashCache};
use winit::{event_loop::EventLoopProxy, window::WindowId};

use crate::{
    callback::WidgetCallback,
    draw::Palette,
    event_loop::UserEvent,
    timer::{TimerId, Timers, WidgetTimer},
    widgets::{RawWidgetId, Widget, WidgetAddress, WidgetId},
    window::WindowRequest,
};

thread_local! {
    pub static SYSTEM: SharedSystemData = SharedSystemData(RefCell::new(None));
}

pub struct SharedSystemDataInner {
    pub address_book: HashMap<RawWidgetId, WidgetAddress>,
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,

    // TODO: per-widget font metrics and palette (as part of the style)
    pub font_metrics: cosmic_text::Metrics,
    pub palette: Palette,
    pub event_loop_proxy: EventLoopProxy<UserEvent>,
    pub timers: Timers,
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

pub fn send_window_event(window_id: WindowId, event: impl Into<WindowRequest>) {
    with_system(|system| {
        let _ = system
            .event_loop_proxy
            .send_event(UserEvent::WindowRequest(window_id, event.into()));
    });
}

pub fn add_timer<W: Widget, F>(duration: Duration, widget_id: WidgetId<W>, func: F) -> TimerId
where
    F: Fn(&mut W, Instant) + 'static,
{
    add_timer_or_interval(duration, None, widget_id, func)
}

pub fn add_interval<W: Widget, F>(interval: Duration, widget_id: WidgetId<W>, func: F) -> TimerId
where
    F: Fn(&mut W, Instant) + 'static,
{
    add_timer_or_interval(interval, Some(interval), widget_id, func)
}

pub fn add_timer_or_interval<W: Widget, F>(
    duration: Duration,
    interval: Option<Duration>,
    widget_id: WidgetId<W>,
    func: F,
) -> TimerId
where
    F: Fn(&mut W, Instant) + 'static,
{
    with_system(|system| {
        system.timers.add(
            Instant::now() + duration,
            WidgetTimer {
                interval,
                callback: WidgetCallback {
                    widget_id: widget_id.0,
                    func: Rc::new(move |widget, event| {
                        func(
                            widget.downcast_mut::<W>().expect("widget type mismatch"),
                            event,
                        )
                    }),
                },
            },
        )
    })
}
