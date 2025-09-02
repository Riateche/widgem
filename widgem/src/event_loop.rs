use {
    crate::{
        app_builder::AppBuilder,
        callback::{CallbackId, InvokeCallbackEvent},
        shared_window::{WindowId, WindowRequest},
        style::defaults::default_style,
        system::{ReportError, SharedSystemDataInner, SystemConfig},
        timer::Timers,
        widgets::{
            get_widget_by_address_mut, get_widget_by_id_mut, RawWidgetId, RootWidget, Widget,
            WidgetBase, WidgetExt,
        },
        window_handler::WindowHandler,
        App,
    },
    arboard::Clipboard,
    cosmic_text::{fontdb, FontSystem, SwashCache},
    derive_more::From,
    scoped_tls::scoped_thread_local,
    std::{
        any::Any,
        collections::HashMap,
        fmt::Debug,
        rc::Rc,
        time::{Duration, Instant},
    },
    tracing::warn,
    winit::{
        application::ApplicationHandler,
        event::{StartCause, WindowEvent},
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    },
};

// TODO: private
#[derive(Debug, From)]
pub enum UserEvent {
    InvokeCallback(InvokeCallbackEvent),
    WindowRequest(WindowId, WindowRequest),
    Accesskit(accesskit_winit::Event),
    DeleteWidget(RawWidgetId),
}

scoped_thread_local!(static ACTIVE_EVENT_LOOP: ActiveEventLoop);

pub(crate) fn with_active_event_loop<F, R>(f: F) -> R
where
    F: FnOnce(&ActiveEventLoop) -> R,
{
    ACTIVE_EVENT_LOOP.with(f)
}

type BoxInitFn = Box<dyn FnOnce(&mut RootWidget) -> anyhow::Result<()>>;

pub(crate) struct Handler {
    app_builder: AppBuilder,
    is_initialized: bool,
    init: Option<BoxInitFn>,
    root_widget: Option<Box<dyn Widget>>,
    event_loop_proxy: Option<EventLoopProxy<UserEvent>>,
}

impl Handler {
    pub(crate) fn new(
        app: AppBuilder,
        event_loop: &EventLoop<UserEvent>,
        init: impl FnOnce(&mut RootWidget) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self {
            app_builder: app,
            init: Some(Box::new(init)),
            is_initialized: false,
            root_widget: None,
            event_loop_proxy: Some(event_loop.create_proxy()),
        }
    }

    fn before_handler(&mut self) {
        if let Some(root_widget) = &self.root_widget {
            while let Some(timer) = root_widget.base().app().next_ready_timer() {
                timer.callback.invoke(Instant::now());
            }
        }
    }

    fn after_widget_activity(&mut self) {
        let Some(root_widget) = &mut self.root_widget else {
            return;
        };
        loop {
            let mut addrs = root_widget.base().app().take_pending_children_updates();
            if addrs.is_empty() {
                break;
            }
            // Update upper layers first.
            addrs.sort_unstable_by_key(|addr| addr.len());
            for addr in addrs {
                if let Ok(widget) = get_widget_by_address_mut(root_widget.as_mut(), &addr) {
                    widget.update_children();
                }
            }
        }
        let windows = root_widget.base().app().windows();
        //println!("after widget activity1");
        for (_, window) in windows {
            //println!("root_widget_id {:?}", window.root_widget_id);
            if let Some(window_root_widget) =
                get_widget_by_id_mut(root_widget.as_mut(), window.root_widget_id).or_report_err()
            {
                WindowHandler::new(window.shared_window, window_root_widget)
                    .after_widget_activity();
            }
        }
        //println!("after widget activity1 ok");

        if root_widget.base().app().should_exit() {
            with_active_event_loop(|event_loop| event_loop.exit());
        }
    }
}

const DEFAULT_AUTO_REPEAT_DELAY: Duration = Duration::from_millis(500);
const DEFAULT_AUTO_REPEAT_INTERVAL: Duration = Duration::from_millis(50);

impl ApplicationHandler<UserEvent> for Handler {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();

            let Some(root_widget) = &mut self.root_widget else {
                warn!(
                    "cannot dispatch event when root widget doesn't exist: {:?}",
                    event
                );
                return;
            };

            let Some(window) = root_widget.base().app().window_for_winit_id(window_id) else {
                if !matches!(event, WindowEvent::Destroyed | WindowEvent::RedrawRequested) {
                    warn!("missing window object when dispatching event: {:?}", event);
                }
                return;
            };

            if let Some(window_root_widget) =
                get_widget_by_id_mut(root_widget.as_mut(), window.root_widget_id).or_report_err()
            {
                WindowHandler::new(window.shared_window, window_root_widget).handle_event(event);
            }

            self.after_widget_activity();
        })
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            if self.is_initialized {
                return;
            }

            let mut db = fontdb::Database::new();
            for custom_font_path in &self.app_builder.custom_font_paths {
                if let Err(err) = db.load_font_file(custom_font_path) {
                    warn!(
                        "failed to initialize custom font from {:?}: {:?}",
                        custom_font_path, err
                    );
                }
            }
            if self.app_builder.system_fonts {
                db.load_system_fonts();
            }
            let font_system =
                FontSystem::new_with_locale_and_db(FontSystem::new().locale().to_string(), db);

            let shared_system_data = SharedSystemDataInner {
                config: Rc::new(SystemConfig {
                    exit_after_last_window_closes: true,
                    // TODO: should we fetch system settings instead?
                    auto_repeat_delay: self
                        .app_builder
                        .auto_repeat_delay
                        .unwrap_or(DEFAULT_AUTO_REPEAT_DELAY),
                    auto_repeat_interval: self
                        .app_builder
                        .auto_repeat_interval
                        .unwrap_or(DEFAULT_AUTO_REPEAT_INTERVAL),
                    fixed_scale: self.app_builder.fixed_scale,
                }),
                address_book: HashMap::new(),
                font_system,
                swash_cache: SwashCache::new(),
                event_loop_proxy: self.event_loop_proxy.take().expect("only happens once"),
                // TODO: how to detect monitor scale change?
                style: default_style(),
                timers: Timers::new(),
                clipboard: Clipboard::new().expect("failed to initialize clipboard"),
                had_any_windows: false,
                windows: HashMap::new(),
                windows_by_winit_id: HashMap::new(),
                widget_callbacks: HashMap::new(),
                application_shortcuts: Vec::new(),
                pending_children_updates: Vec::new(),
                current_children_update: None,
                current_layout_state: None,
            };
            let app = App::init(shared_system_data);
            let mut root_widget = RootWidget::new(WidgetBase::new_root(app));
            self.init.take().expect("double init")(&mut root_widget).or_report_err();
            self.root_widget = Some(Box::new(root_widget));

            self.is_initialized = true;
        });
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, _cause: StartCause) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            if !self.is_initialized {
                return;
            }
            self.before_handler();
            self.after_widget_activity();
        })
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            let Some(root_widget) = &mut self.root_widget else {
                warn!(
                    "cannot dispatch event when root widget doesn't exist: {:?}",
                    event
                );
                return;
            };
            match event {
                UserEvent::WindowRequest(window_id, request) => {
                    let Some(window) = root_widget.base().app().window(window_id) else {
                        warn!("missing window object when dispatching WindowRequest");
                        return;
                    };

                    let Ok(window_root_widget) =
                        get_widget_by_id_mut(root_widget.as_mut(), window.root_widget_id)
                    else {
                        warn!("missing root widget when dispatching WindowRequest");
                        return;
                    };
                    WindowHandler::new(window.shared_window, window_root_widget)
                        .handle_request(request);
                }
                // TODO: remove event, remove window directly
                UserEvent::InvokeCallback(event) => {
                    dispatch_widget_callback(root_widget.as_mut(), event.callback_id, event.event);
                }
                UserEvent::Accesskit(event) => {
                    let Some(window) = root_widget
                        .base()
                        .app()
                        .window_for_winit_id(event.window_id)
                    else {
                        warn!("missing window object when dispatching Accesskit event");
                        return;
                    };
                    let Ok(window_root_widget) =
                        get_widget_by_id_mut(root_widget.as_mut(), window.root_widget_id)
                    else {
                        warn!("missing root widget when dispatching Accesskit event");
                        return;
                    };
                    WindowHandler::new(window.shared_window, window_root_widget)
                        .handle_accesskit_event(event);
                }
                UserEvent::DeleteWidget(id) => {
                    if id == root_widget.base().id() {
                        self.root_widget = None;
                        with_active_event_loop(|event_loop| event_loop.exit());
                    } else if let Err(err) = root_widget.base_mut().remove_child_by_id(id) {
                        warn!("failed to remove widget: {:?}", err);
                    }
                }
            }
            self.after_widget_activity();
        })
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        ACTIVE_EVENT_LOOP.set(event_loop, || {
            self.before_handler();
            let next_timer = self
                .root_widget
                .as_ref()
                .and_then(|w| w.base().app().next_timer_instant());
            if let Some(next_timer) = next_timer {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_timer));
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
            self.after_widget_activity();
        })
    }
}

fn dispatch_widget_callback(
    root_widget: &mut dyn Widget,
    callback_id: CallbackId,
    event: Box<dyn Any + Send>,
) {
    let Some(callback) = root_widget.base().app().widget_callback(callback_id) else {
        warn!("unknown widget callback id");
        return;
    };
    let Some(address) = root_widget.base().app().address(callback.widget_id) else {
        return;
    };
    let Ok(widget) = get_widget_by_address_mut(root_widget, &address) else {
        return;
    };
    (callback.func)(widget, event).or_report_err();
    widget.update_accessibility_node();
}
