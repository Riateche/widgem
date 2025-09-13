use std::{
    cell::RefCell,
    collections::HashMap,
    mem,
    rc::Rc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _};
use cosmic_text::{FontSystem, SwashCache};
use tracing::warn;
use winit::{event_loop::EventLoopProxy, monitor::MonitorHandle};

use crate::{
    app_builder::AppBuilder,
    callback::{Callback, CallbackId, WidgetCallbackData},
    event::{FocusReason, KeyboardInputEvent},
    event_loop::{with_active_event_loop, UserEvent},
    shared_window::{
        ScrollToRectRequest, SetFocusRequest, SharedWindow, WindowId, WindowInfo, WindowRequest,
    },
    shortcut::{Shortcut, ShortcutId},
    style::Style,
    system::{LayoutState, SharedSystemDataInner, SystemConfig},
    timer::{Timer, TimerId},
    types::Rect,
    RawWidgetId, Widget, WidgetAddress, WidgetId,
};

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
))]
use arboard::{GetExtLinux, LinuxClipboardKind, SetExtLinux};

pub struct App {
    data: Rc<RefCell<SharedSystemDataInner>>,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish()
    }
}

impl App {
    pub fn builder() -> AppBuilder {
        AppBuilder::new()
    }

    pub(crate) fn init(data: SharedSystemDataInner) -> Self {
        App {
            data: Rc::new(RefCell::new(data)),
        }
    }

    // Note: App intentionally doesn't implement `Clone` so that the user can only get a `&App`,
    // not an owned version.
    pub(crate) fn create_app_handle(&self) -> Self {
        App {
            data: Rc::clone(&self.data),
        }
    }

    pub(crate) fn address(&self, id: RawWidgetId) -> Option<WidgetAddress> {
        let data = self.data.borrow();
        data.address_book.get(&id).cloned()
    }

    pub(crate) fn register_address(
        &self,
        id: RawWidgetId,
        address: WidgetAddress,
    ) -> Option<WidgetAddress> {
        let mut data = self.data.borrow_mut();
        data.address_book.insert(id, address)
    }

    pub(crate) fn unregister_address(&self, id: RawWidgetId) -> Option<WidgetAddress> {
        let mut data = self.data.borrow_mut();
        data.address_book.remove(&id)
    }

    fn send_window_request(&self, window_id: WindowId, request: impl Into<WindowRequest>) {
        let data = self.data.borrow();
        let _ = data
            .event_loop_proxy
            .send_event(UserEvent::WindowRequest(window_id, request.into()));
    }

    // rect is in widget coordinates.
    pub(crate) fn scroll_to_rect(&self, window_id: WindowId, widget_id: RawWidgetId, rect: Rect) {
        self.send_window_request(
            window_id,
            WindowRequest::ScrollToRect(ScrollToRectRequest { widget_id, rect }),
        );
    }

    // rect is in widget coordinates.
    pub(crate) fn set_focus(
        &self,
        window_id: WindowId,
        widget_id: RawWidgetId,
        reason: FocusReason,
    ) {
        self.send_window_request(
            window_id,
            WindowRequest::SetFocus(SetFocusRequest { widget_id, reason }),
        );
    }

    pub fn add_timer(&self, duration: Duration, callback: Callback<Instant>) -> TimerId {
        self.add_timer_or_interval(duration, None, callback)
    }

    pub fn add_interval(&self, interval: Duration, callback: Callback<Instant>) -> TimerId {
        self.add_timer_or_interval(interval, Some(interval), callback)
    }

    fn add_timer_or_interval(
        &self,
        duration: Duration,
        interval: Option<Duration>,
        callback: Callback<Instant>,
    ) -> TimerId {
        let mut data = self.data.borrow_mut();
        data.timers
            .add(Instant::now() + duration, Timer { interval, callback })
    }

    pub fn cancel_timer(&self, id: TimerId) {
        let mut data = self.data.borrow_mut();
        data.timers.remove(id);
    }

    pub(crate) fn next_ready_timer(&self) -> Option<Timer> {
        let mut data = self.data.borrow_mut();
        data.timers.next_ready_timer()
    }

    pub(crate) fn next_timer_instant(&self) -> Option<Instant> {
        let data = self.data.borrow();
        data.timers.next_instant()
    }

    pub(crate) fn request_children_update(&self, addr: WidgetAddress) {
        let mut data = self.data.borrow_mut();
        data.pending_children_updates.push(addr);
    }

    pub(crate) fn take_pending_children_updates(&self) -> Vec<WidgetAddress> {
        let mut data = self.data.borrow_mut();
        mem::take(&mut data.pending_children_updates)
    }

    /// Creates a callback that will call `func` on the receiver widget with ID `self`.
    ///
    /// The callback will only be invoked after you register it by passing it to a
    /// `.on_*()` function of the sender widget.
    ///
    /// It's only possible to register a single callback for a given sender-signal-receiver
    /// triplet. If another callback with the same receiver is supplied to the same
    /// `.on_*()` function of the same sender widget, it will replace the previous callback.
    /// Thus, it is save to call `.on_*()` functions within `handle_declare_children_request`,
    /// as the new callbacks will overwrite the old ones instead of creating new copies.
    ///
    /// The callback will be automatically deregistered when the sender or the receiver is deleted.
    pub(crate) fn create_widget_callback<W, E, F>(
        &self,
        widget_id: WidgetId<W>,
        func: F,
    ) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> anyhow::Result<()> + 'static,
        E: 'static,
    {
        let callback_id = CallbackId::new();
        let callback_data = WidgetCallbackData {
            widget_id: widget_id.raw(),
            func: Rc::new(move |widget, any_event| {
                let widget = widget
                    .downcast_mut::<W>()
                    .context("widget downcast failed")?;
                let event = any_event
                    .downcast::<E>()
                    .map_err(|_| anyhow!("event downcast failed"))?;
                func(widget, *event)
            }),
        };
        let mut data = self.data.borrow_mut();
        data.widget_callbacks.insert(callback_id, callback_data);
        Callback::new(data.event_loop_proxy.clone(), callback_id, widget_id.raw())
    }

    pub(crate) fn windows(&self) -> HashMap<WindowId, WindowInfo> {
        let data = self.data.borrow();
        data.windows.clone()
    }

    pub(crate) fn add_window(&self, window: &SharedWindow) {
        let info = WindowInfo {
            id: window.id(),
            root_widget_id: window.root_widget_id(),
            shared_window: window.clone(),
        };
        let mut data = self.data.borrow_mut();
        data.windows.insert(info.id, info);
        data.had_any_windows = true;
    }

    pub(crate) fn add_winit_window(&self, winit_id: winit::window::WindowId, id: WindowId) {
        let mut data = self.data.borrow_mut();
        let Some(info) = data.windows.get(&id).cloned() else {
            warn!("add_winit_window: unknown window id");
            return;
        };
        data.windows_by_winit_id.insert(winit_id, info);
    }

    pub(crate) fn remove_window(&self, window: &SharedWindow) {
        let id = window.id();
        let winit_id = window.winit_id();

        let mut data = self.data.borrow_mut();

        data.windows.remove(&id);
        if let Some(winit_id) = winit_id {
            data.windows_by_winit_id.remove(&winit_id);
        }
    }

    pub(crate) fn window(&self, id: WindowId) -> Option<WindowInfo> {
        let data = self.data.borrow();
        data.windows.get(&id).cloned()
    }

    pub(crate) fn window_for_winit_id(&self, id: winit::window::WindowId) -> Option<WindowInfo> {
        let data = self.data.borrow();
        data.windows_by_winit_id.get(&id).cloned()
    }

    pub(crate) fn widget_callback(&self, id: CallbackId) -> Option<WidgetCallbackData> {
        let data = self.data.borrow();
        data.widget_callbacks.get(&id).cloned()
    }

    pub(crate) fn should_exit(&self) -> bool {
        let data = self.data.borrow();
        data.had_any_windows && data.windows.is_empty() && data.config.exit_after_last_window_closes
    }

    /// Trigger shutdown of the application.
    pub fn exit(&self) {
        with_active_event_loop(|event_loop| event_loop.exit());
    }

    pub(crate) fn event_loop_proxy(&self) -> EventLoopProxy<UserEvent> {
        let data = self.data.borrow();
        data.event_loop_proxy.clone()
    }

    pub(crate) fn config(&self) -> Rc<SystemConfig> {
        let data = self.data.borrow();
        data.config.clone()
    }

    pub(crate) fn style(&self) -> Style {
        let data = self.data.borrow();
        data.style.clone()
    }

    pub(crate) fn with_font_system<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut FontSystem) -> R,
    {
        let mut data = self.data.borrow_mut();
        f(&mut data.font_system)
    }

    pub(crate) fn with_font_system_and_swash_cache<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut FontSystem, &mut SwashCache) -> R,
    {
        let data = &mut *self.data.borrow_mut();
        f(&mut data.font_system, &mut data.swash_cache)
    }

    pub fn clipboard_text(&self) -> anyhow::Result<String> {
        let mut data = self.data.borrow_mut();
        Ok(data.clipboard.get_text()?)
    }

    pub fn set_clipboard_text(&self, text: &str) -> anyhow::Result<()> {
        let mut data = self.data.borrow_mut();
        data.clipboard.set_text(text)?;
        Ok(())
    }

    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
    ))]
    pub fn linux_primary_selection(&self) -> anyhow::Result<String> {
        let mut data = self.data.borrow_mut();

        Ok(data
            .clipboard
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()?)
    }

    #[cfg(all(
        unix,
        not(any(target_os = "macos", target_os = "android", target_os = "emscripten"))
    ))]
    pub fn set_linux_primary_selection(&self, text: &str) -> anyhow::Result<()> {
        {
            let mut data = self.data.borrow_mut();

            data.clipboard
                .set()
                .clipboard(LinuxClipboardKind::Primary)
                .text(text)?;
            Ok(())
        }
    }

    pub(crate) fn remove_shortcut(&self, id: ShortcutId) {
        let mut data = self.data.borrow_mut();
        // TODO: HashMap/BTreeMap?
        data.application_shortcuts.retain(|s| s.id != id);
    }

    pub(crate) fn add_shortcut(&self, shortcut: Shortcut) {
        let mut data = self.data.borrow_mut();
        data.application_shortcuts.push(shortcut);
    }

    pub(crate) fn trigger_shortcuts(&self, event: &KeyboardInputEvent) {
        let mut triggered_callbacks = Vec::new();
        {
            let data = self.data.borrow();
            for shortcut in &data.application_shortcuts {
                if shortcut.key_combinations.matches(event) {
                    triggered_callbacks.push(shortcut.callback.clone());
                }
            }
        }
        for callback in triggered_callbacks {
            callback.invoke(());
        }
    }

    // TODO: remove
    pub(crate) fn with_current_layout_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Option<LayoutState>) -> R,
    {
        let mut data = self.data.borrow_mut();
        f(&mut data.current_layout_state)
    }

    pub fn available_monitors(&self) -> impl Iterator<Item = MonitorHandle> {
        with_active_event_loop(|e| e.available_monitors())
    }

    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        with_active_event_loop(|e| e.primary_monitor())
    }
}
