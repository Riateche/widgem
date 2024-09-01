use std::{
    any::Any, collections::HashMap, fmt::Debug, marker::PhantomData, path::PathBuf, rc::Rc,
    sync::mpsc::SyncSender, time::Instant,
};

use anyhow::{anyhow, Result};
use arboard::Clipboard;
use cosmic_text::{fontdb, FontSystem, SwashCache};
use derive_more::From;
use log::warn;
use scoped_tls::scoped_thread_local;
use tiny_skia::Pixmap;
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::WindowId,
};

use linked_hash_map::LinkedHashMap;

use crate::{
    callback::{
        Callback, CallbackDataFn, CallbackId, CallbackKind, CallbackMaker, Callbacks,
        InvokeCallbackEvent,
    },
    style::{computed::ComputedStyle, defaults::default_style},
    system::{address, with_system, ReportError, SharedSystemDataInner, SYSTEM},
    timer::Timers,
    widgets::{
        get_widget_by_address_mut, RawWidgetId, Widget, WidgetExt, WidgetId, WidgetNotFound,
    },
    window::{Window, WindowRequest},
};

pub struct CallbackContext<'a, State> {
    windows: &'a mut LinkedHashMap<WindowId, Window>,
    add_callback: Box<dyn FnMut(Box<CallbackDataFn<State>>) -> CallbackId + 'a>,
    marker: PhantomData<State>,
}

impl<'a, State> CallbackContext<'a, State> {
    pub fn map_state<AnotherState: 'static>(
        &mut self,
        mapper: impl Fn(&mut State) -> Option<&mut AnotherState> + Clone + 'static,
    ) -> CallbackContext<'_, AnotherState> {
        let add_callback = &mut self.add_callback;
        CallbackContext {
            windows: self.windows,
            marker: PhantomData,
            add_callback: Box::new(move |mut f| -> CallbackId {
                let mapper = mapper.clone();
                (add_callback)(Box::new(move |state, ctx, any_event| {
                    if let Some(another_state) = mapper(state) {
                        let mut new_ctx = ctx.map_state::<AnotherState>(mapper.clone());
                        f(another_state, &mut new_ctx, any_event)
                    } else {
                        Ok(())
                    }
                }))
            }),
        }
    }

    pub fn callback<Event: 'static>(
        &mut self,
        mut callback: impl FnMut(&mut State, &mut CallbackContext<State>, Event) -> Result<()> + 'static,
    ) -> Callback<Event> {
        let callback_id = (self.add_callback)(Box::new(move |state, ctx, any_event| {
            let event = *any_event
                .downcast::<Event>()
                .map_err(|_| anyhow!("event downcast failed"))?;
            callback(state, ctx, event)
        }));
        let event_loop_proxy = with_system(|s| s.event_loop_proxy.clone());
        Callback::new(event_loop_proxy, callback_id, CallbackKind::State)
    }

    pub fn widget<W: Widget>(&mut self, id: WidgetId<W>) -> Result<&mut W, WidgetNotFound> {
        let w = self.widget_raw(id.0)?;
        Ok(w.downcast_mut::<W>().expect("widget downcast failed"))
    }

    pub fn widget_raw(&mut self, id: RawWidgetId) -> Result<&mut dyn Widget, WidgetNotFound> {
        let address = address(id).ok_or(WidgetNotFound)?;
        let window = self
            .windows
            .get_mut(&address.window_id)
            .ok_or(WidgetNotFound)?;
        let widget = window.root_widget.as_mut().ok_or(WidgetNotFound)?;
        get_widget_by_address_mut(widget.as_mut(), &address)
    }
}

#[derive(Debug)]
pub struct Snapshot(pub Vec<Pixmap>);

#[derive(Debug, From)]
pub enum UserEvent {
    InvokeCallback(InvokeCallbackEvent),
    WindowRequest(WindowId, WindowRequest),
    WindowClosed(WindowId),
    Accesskit(accesskit_winit::Event),
    SnapshotRequest(SyncSender<Snapshot>),
    DispatchWindowEvent(usize, WindowEvent),
}

scoped_thread_local!(static ACTIVE_EVENT_LOOP: ActiveEventLoop);

pub fn with_active_event_loop<F, R>(f: F) -> R
where
    F: FnOnce(&ActiveEventLoop) -> R,
{
    ACTIVE_EVENT_LOOP.with(f)
}

// pub fn with_window_target<F, R>(f: F) -> R
// where
//     F: FnOnce(&EventLoopWindowTarget<UserEvent>) -> R,
// {
//     WINDOW_TARGET.with(f)
// }

fn dispatch_widget_callback(
    windows: &mut LinkedHashMap<WindowId, Window>,
    callback_id: CallbackId,
    event: Box<dyn Any + Send>,
) {
    let Some(callback) = with_system(|s| s.widget_callbacks.get(&callback_id).cloned()) else {
        warn!("unknown widget callback id");
        return;
    };
    let Some(address) = address(callback.widget_id) else {
        return;
    };
    let Some(window) = windows.get_mut(&address.window_id) else {
        return;
    };
    let Some(root_widget) = window.root_widget.as_mut() else {
        return;
    };
    let Ok(widget) = get_widget_by_address_mut(root_widget.as_mut(), &address) else {
        return;
    };
    (callback.func)(widget, event).or_report_err();
    widget.update_accessible();
    window.after_widget_activity();
}

fn fetch_new_windows(windows: &mut LinkedHashMap<WindowId, Window>) {
    with_system(|system| {
        for window in system.new_windows.drain(..) {
            windows.insert(window.id, window);
        }
    });
}

fn default_scale(event_loop: &ActiveEventLoop) -> f32 {
    let monitor = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next());
    if let Some(monitor) = monitor {
        monitor.scale_factor() as f32
    } else {
        warn!("unable to find any monitors");
        1.0
    }
}

pub struct App {
    system_fonts: bool,
    custom_font_paths: Vec<PathBuf>,
    fixed_scale: Option<f32>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> App {
        App {
            system_fonts: true,
            custom_font_paths: vec![],
            fixed_scale: None,
        }
    }

    pub fn with_system_fonts(mut self, enable: bool) -> App {
        self.system_fonts = enable;
        self
    }

    pub fn with_font(mut self, path: PathBuf) -> App {
        self.custom_font_paths.push(path);
        self
    }

    pub fn with_scale(mut self, scale: f32) -> App {
        self.fixed_scale = Some(scale);
        self
    }

    pub fn run<State: 'static>(
        self,
        make_state: impl FnOnce(&mut CallbackContext<State>) -> State + 'static,
    ) -> Result<(), EventLoopError> {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
        let mut handler = Handler::new(self, make_state, &event_loop);
        event_loop.run_app(&mut handler)
    }
}

/*
let mut windows = LinkedHashMap::<WindowId, Window>::new();

        let mut snapshot_sender = None;

        let mut callback_maker = CallbackMaker::<State>::new();
        let mut callbacks = Callbacks::<State>::new();
        let mut state = None;
        let mut make_state = Some(make_state);
        let mut event_loop_proxy = Some(event_loop.create_proxy()); */
struct Handler<State: 'static> {
    app: App,
    // TODO: back to normal hashmap?
    windows: LinkedHashMap<WindowId, Window>,
    callback_maker: CallbackMaker<State>,
    callbacks: Callbacks<State>,
    make_state: Option<Box<dyn FnOnce(&mut CallbackContext<State>) -> State>>,
    event_loop_proxy: Option<EventLoopProxy<UserEvent>>,
    state: Option<State>,
    snapshot_sender: Option<SyncSender<Snapshot>>,
}

impl<State: 'static> Handler<State> {
    fn new(
        app: App,
        make_state: impl FnOnce(&mut CallbackContext<State>) -> State + 'static,
        event_loop: &EventLoop<UserEvent>,
    ) -> Self {
        Self {
            app,
            windows: Default::default(),
            callback_maker: CallbackMaker::new(),
            callbacks: Callbacks::new(),
            make_state: Some(Box::new(make_state)),
            event_loop_proxy: Some(event_loop.create_proxy()),
            state: None,
            snapshot_sender: None,
        }
    }

    fn before_handler(&mut self) {
        // If initialized.
        if self.make_state.is_none() {
            fetch_new_windows(&mut self.windows);
            while let Some(timer) = with_system(|system| system.timers.pop()) {
                timer.callback.invoke(Instant::now());
            }
        }
    }

    fn after_handler(&mut self) {
        fetch_new_windows(&mut self.windows);
    }
}

impl<State> ApplicationHandler<UserEvent> for Handler<State> {
    // TODO: It's recommended that applications should only initialize their graphics context
    // and create a window after they have received their first `Resumed` event.
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.handle_event(event);
            }
            self.after_handler();
        })
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            if cause == StartCause::Init {
                let mut db = fontdb::Database::new();
                for custom_font_path in &self.app.custom_font_paths {
                    db.load_font_file(custom_font_path)
                        .expect("failed to initialize custom font");
                }
                if self.app.system_fonts {
                    db.load_system_fonts();
                }
                let font_system =
                    FontSystem::new_with_locale_and_db(FontSystem::new().locale().to_string(), db);
                let scale = match self.app.fixed_scale {
                    None => default_scale(&event_loop),
                    Some(fixed_scale) => fixed_scale,
                };

                let shared_system_data = SharedSystemDataInner {
                    address_book: HashMap::new(),
                    font_system,
                    swash_cache: SwashCache::new(),
                    event_loop_proxy: self.event_loop_proxy.take().expect("only happens once"),
                    // TODO: how to detect monitor scale change?
                    default_style: Rc::new(ComputedStyle::new(&default_style(), scale).unwrap()),
                    timers: Timers::new(),
                    clipboard: Clipboard::new().expect("failed to initialize clipboard"),
                    new_windows: Vec::new(),
                    exit_after_last_window_closes: true,
                    widget_callbacks: HashMap::new(),
                };
                SYSTEM.with(|system| {
                    *system.0.borrow_mut() = Some(shared_system_data);
                });

                self.state = {
                    let mut ctx = CallbackContext {
                        windows: &mut self.windows,
                        add_callback: Box::new(|f| self.callback_maker.add(f)),
                        marker: PhantomData,
                    };
                    let make_state = self.make_state.take().expect("only happens once");
                    Some(make_state(&mut ctx))
                };
                self.callbacks.add_all(&mut self.callback_maker);
            }
            self.after_handler();
        })
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            match event {
                UserEvent::WindowRequest(window_id, request) => {
                    if let Some(window) = self.windows.get_mut(&window_id) {
                        window.handle_request(request);
                    }
                }
                UserEvent::WindowClosed(window_id) => {
                    self.windows.remove(&window_id);
                    if self.windows.is_empty() {
                        let exit = with_system(|s| s.exit_after_last_window_closes);
                        if exit {
                            event_loop.exit();
                        }
                    }
                }
                UserEvent::InvokeCallback(event) => match event.kind {
                    CallbackKind::State => {
                        {
                            let mut ctx = CallbackContext {
                                windows: &mut self.windows,
                                add_callback: Box::new(|f| self.callback_maker.add(f)),
                                marker: PhantomData,
                            };

                            self.callbacks
                                .call(self.state.as_mut().unwrap(), &mut ctx, event);
                        }
                        self.callbacks.add_all(&mut self.callback_maker);
                        for mut entry in self.windows.entries() {
                            entry.get_mut().after_widget_activity();
                        }
                    }
                    CallbackKind::Widget => {
                        dispatch_widget_callback(&mut self.windows, event.callback_id, event.event);
                    }
                },
                UserEvent::Accesskit(event) => {
                    if let Some(window) = self.windows.get_mut(&event.window_id) {
                        window.handle_accesskit_event(event);
                    } else {
                        warn!("accesskit event for unknown window: {:?}", event);
                    }
                }
                UserEvent::SnapshotRequest(sender) => {
                    self.snapshot_sender = Some(sender);
                }
                UserEvent::DispatchWindowEvent(window_index, window_event) => {
                    let elem = self.windows.entries().nth(window_index);
                    if let Some(mut elem) = elem {
                        elem.get_mut().handle_event(window_event);
                    } else {
                        warn!(
                            "event dispatch request for unknown window index: {:?}",
                            window_index
                        );
                    }
                }
            }
            self.after_handler();
        })
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            let snapshot_sender = self.snapshot_sender.take();
            if let Some(sender) = snapshot_sender {
                let snapshots_vec: Vec<Pixmap> = self
                    .windows
                    .iter()
                    .map(|(_, w)| w.pixmap.borrow().clone())
                    .collect();
                let result = sender.send(Snapshot(snapshots_vec));
                if result.is_err() {
                    warn!("Failed to send snapshot");
                }
            }

            let next_timer = with_system(|system| system.timers.next_instant());
            if let Some(next_timer) = next_timer {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_timer));
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            self.after_handler();
        })
    }
}
