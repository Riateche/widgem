use std::{
    any::Any, collections::HashMap, fmt::Debug, marker::PhantomData, rc::Rc,
    sync::mpsc::SyncSender, time::Instant,
};

use accesskit_winit::ActionRequestEvent;
use anyhow::{anyhow, Result};
use arboard::Clipboard;
use cosmic_text::{FontSystem, SwashCache};
use derive_more::From;
use log::{trace, warn};
use scoped_tls::scoped_thread_local;
use tiny_skia::Pixmap;
use winit::{
    error::EventLoopError,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopWindowTarget},
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
    ActionRequest(ActionRequestEvent),
    SnapshotRequest(SyncSender<Snapshot>),
    DispatchWindowEvent(usize, WindowEvent),
}

scoped_thread_local!(static WINDOW_TARGET: EventLoopWindowTarget<UserEvent>);

pub fn with_window_target<F, R>(f: F) -> R
where
    F: FnOnce(&EventLoopWindowTarget<UserEvent>) -> R,
{
    WINDOW_TARGET.with(f)
}

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

fn default_scale<T>(window_target: &EventLoopWindowTarget<T>) -> f32 {
    let monitor = window_target
        .primary_monitor()
        .or_else(|| window_target.available_monitors().next());
    if let Some(monitor) = monitor {
        monitor.scale_factor() as f32
    } else {
        warn!("unable to find any monitors");
        1.0
    }
}

pub fn run<State: 'static>(
    make_state: impl FnOnce(&mut CallbackContext<State>) -> State,
) -> Result<(), EventLoopError> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build()?;

    let mut windows = LinkedHashMap::<WindowId, Window>::new();

    let mut snapshot_sender = None;

    let shared_system_data = SharedSystemDataInner {
        address_book: HashMap::new(),
        font_system: FontSystem::new(),
        swash_cache: SwashCache::new(),
        event_loop_proxy: event_loop.create_proxy(),
        // TODO: how to detect monitor scale change?
        default_style: Rc::new(ComputedStyle::old_new(
            default_style(),
            default_scale(&event_loop),
        )),
        timers: Timers::new(),
        clipboard: Clipboard::new().expect("failed to initialize clipboard"),
        new_windows: Vec::new(),
        exit_after_last_window_closes: true,
        widget_callbacks: HashMap::new(),
    };
    SYSTEM.with(|system| {
        *system.0.borrow_mut() = Some(shared_system_data);
    });

    let mut callback_maker = CallbackMaker::<State>::new();
    let mut callbacks = Callbacks::<State>::new();

    let mut state = {
        let mut ctx = CallbackContext {
            windows: &mut windows,
            add_callback: Box::new(|f| callback_maker.add(f)),
            marker: PhantomData,
        };
        WINDOW_TARGET.set(&event_loop, || make_state(&mut ctx))
    };
    callbacks.add_all(&mut callback_maker);
    fetch_new_windows(&mut windows);

    event_loop.run(move |event, window_target| {
        WINDOW_TARGET.set(window_target, || {
            fetch_new_windows(&mut windows);
            while let Some(timer) = with_system(|system| system.timers.pop()) {
                timer.callback.invoke(Instant::now());
            }

            match event {
                Event::WindowEvent { window_id, event } => {
                    if let Some(window) = windows.get_mut(&window_id) {
                        window.handle_event(event);
                    }
                }
                Event::UserEvent(event) => match event {
                    UserEvent::WindowRequest(window_id, request) => {
                        if let Some(window) = windows.get_mut(&window_id) {
                            window.handle_request(request);
                        }
                    }
                    UserEvent::WindowClosed(window_id) => {
                        windows.remove(&window_id);
                        if windows.is_empty() {
                            let exit = with_system(|s| s.exit_after_last_window_closes);
                            if exit {
                                window_target.exit();
                            }
                        }
                    }
                    UserEvent::InvokeCallback(event) => match event.kind {
                        CallbackKind::State => {
                            {
                                let mut ctx = CallbackContext {
                                    windows: &mut windows,
                                    add_callback: Box::new(|f| callback_maker.add(f)),
                                    marker: PhantomData,
                                };

                                callbacks.call(&mut state, &mut ctx, event);
                            }
                            callbacks.add_all(&mut callback_maker);
                            for mut entry in windows.entries() {
                                entry.get_mut().after_widget_activity();
                            }
                        }
                        CallbackKind::Widget => {
                            dispatch_widget_callback(&mut windows, event.callback_id, event.event);
                        }
                    },
                    UserEvent::ActionRequest(request) => {
                        trace!("accesskit request: {:?}", request);
                        if let Some(window) = windows.get_mut(&request.window_id) {
                            window.handle_accessible_request(request.request);
                        } else {
                            warn!("accesskit request for unknown window: {:?}", request);
                        }
                    }
                    UserEvent::SnapshotRequest(sender) => {
                        snapshot_sender = Some(sender);
                    }
                    UserEvent::DispatchWindowEvent(window_index, window_event) => {
                        let elem = windows.entries().nth(window_index);
                        if let Some(mut elem) = elem {
                            elem.get_mut().handle_event(window_event);
                        } else {
                            warn!(
                                "event dispatch request for unknown window index: {:?}",
                                window_index
                            );
                        }
                    }
                },
                Event::AboutToWait => {
                    let snapshot_sender = snapshot_sender.take();
                    if let Some(sender) = snapshot_sender {
                        let snapshots_vec: Vec<Pixmap> = windows
                            .iter()
                            .map(|(_, w)| w.pixmap.borrow().clone())
                            .collect();
                        let result = sender.send(Snapshot(snapshots_vec));
                        if let Err(snapshot) = result {
                            warn!("Failed to send snapshot {:?}", snapshot);
                        }
                    }

                    let next_timer = with_system(|system| system.timers.next_instant());
                    if let Some(next_timer) = next_timer {
                        window_target.set_control_flow(ControlFlow::WaitUntil(next_timer));
                    } else {
                        window_target.set_control_flow(ControlFlow::Wait);
                    }
                }
                _ => {}
            }
            fetch_new_windows(&mut windows);
        });
    })
}
