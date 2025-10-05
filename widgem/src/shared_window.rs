use {
    crate::{
        accessibility::AccessibilityNodes,
        child_key::ChildKey,
        draw::DrawEvent,
        event::FocusReason,
        event_loop::{with_active_event_loop, UserEvent},
        system::OrWarn,
        types::{PhysicalPixels, Point, Rect, Size},
        App, MonitorExt, RawWidgetId, Widget, WidgetAddress, WidgetExt, WindowRectRequest,
    },
    accesskit::NodeId,
    anyhow::{bail, Context},
    derivative::Derivative,
    derive_more::From,
    std::{
        cell::RefCell,
        cmp::{max, min},
        collections::HashSet,
        fmt::Display,
        mem,
        num::NonZeroU32,
        panic::catch_unwind,
        rc::Rc,
        sync::Mutex,
        time::{Duration, Instant},
    },
    tiny_skia::Pixmap,
    tracing::{info, trace, warn},
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, MouseButton, WindowEvent},
        event_loop::EventLoopProxy,
        keyboard::ModifiersState,
        window::{
            CursorIcon, Fullscreen, Icon, Theme, WindowAttributes, WindowButtons, WindowLevel,
        },
    },
};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;
#[cfg(windows)]
use winit::platform::windows::WindowAttributesExtWindows;
#[cfg(all(unix, not(target_vendor = "apple")))]
use winit::platform::x11::WindowAttributesExtX11;

// Extra size to avoid visual artifacts when resizing the window.
// Must be > 0 to avoid panic on pixmap creation.
const EXTRA_SURFACE_SIZE: u32 = 50;
// TODO: get system setting
const DOUBLE_CLICK_TIMEOUT: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, Copy)]
pub enum MouseEventState {
    NotAccepted,
    AcceptedBy(RawWidgetId),
}

impl MouseEventState {
    pub fn is_accepted(self) -> bool {
        matches!(self, Self::AcceptedBy(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(RawWidgetId);

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: WindowId,
    pub shared_window: SharedWindow,
    pub root_widget_id: RawWidgetId,
}

// TODO: private
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SharedWindowInner {
    pub id: WindowId,
    pub root_widget_id: RawWidgetId,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub modifiers_state: ModifiersState,
    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub is_window_focused: bool,
    pub accessibility_nodes: AccessibilityNodes,
    pub mouse_entered_widgets: Vec<(Rect, RawWidgetId)>,
    pub current_mouse_event_state: Option<MouseEventState>,
    pub pixmap: Rc<RefCell<Pixmap>>,
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `winit_window`.
    #[derivative(Debug = "ignore")]
    pub surface: Option<softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>>,
    #[derivative(Debug = "ignore")]
    pub softbuffer_context: Option<softbuffer::Context<Rc<winit::window::Window>>>,
    // Mutex provides unwind safety.
    #[derivative(Debug = "ignore")]
    pub accesskit_adapter: Option<Mutex<accesskit_winit::Adapter>>,
    pub winit_window: Option<Rc<winit::window::Window>>,
    pub min_inner_size: Size,
    pub preferred_inner_size: Size,
    pub input_method_enabled: bool,
    pub ime_cursor_area: Rect,

    pub pending_size_hint_invalidations: Vec<WidgetAddress>,
    pub pending_redraw: bool,
    pub pending_accessibility_updates: Vec<WidgetAddress>,

    // TODO: refactor as struct
    pub focusable_widgets: Vec<(Vec<(ChildKey, RawWidgetId)>, RawWidgetId)>,
    pub focusable_widgets_changed: bool,

    pub focused_widget: Option<(Vec<(ChildKey, RawWidgetId)>, RawWidgetId)>,
    pub mouse_grabber_widget: Option<RawWidgetId>,
    pub num_clicks: u32,
    pub last_click_button: Option<MouseButton>,
    pub last_click_instant: Option<Instant>,
    pub is_delete_widget_on_close_enabled: bool,

    pub attributes: Attributes,
    pub event_loop_proxy: EventLoopProxy<UserEvent>,
}

#[derive(Debug, Clone)]
pub struct Attributes {
    pub outer_position: Option<Point>,
    pub resizable: bool,
    pub enabled_buttons: WindowButtons,
    pub title: Option<String>,
    pub maximized: bool,
    pub visible: bool,
    pub transparent: bool,
    pub blur: bool,
    pub decorations: bool,
    pub window_icon: Option<Icon>,
    pub preferred_theme: Option<Theme>,
    pub resize_increments: Option<Size>,
    pub content_protected: bool,
    pub window_level: WindowLevel,
    pub active: Option<bool>,
    pub fullscreen: Option<Fullscreen>,
    // TODO: more platform specific
    pub has_macos_shadow: Option<bool>,
    pub x11_window_type: Option<Vec<X11WindowType>>,
    pub skip_windows_taskbar: Option<bool>,
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            outer_position: None,
            resizable: true,
            enabled_buttons: WindowButtons::all(),
            title: None,
            maximized: false,
            fullscreen: None,
            visible: true,
            transparent: false,
            blur: false,
            decorations: true,
            window_level: Default::default(),
            window_icon: None,
            preferred_theme: None,
            resize_increments: None,
            content_protected: false,
            active: None,
            has_macos_shadow: None,
            x11_window_type: None,
            skip_windows_taskbar: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedWindow(Rc<RefCell<SharedWindowInner>>);

impl SharedWindow {
    pub(crate) fn new(root_widget_id: RawWidgetId, app: &App) -> Self {
        SharedWindow(Rc::new(RefCell::new(SharedWindowInner {
            id: WindowId(RawWidgetId::new_unique()),
            root_widget_id,
            cursor_position: None,
            cursor_entered: false,
            modifiers_state: ModifiersState::default(),
            pressed_mouse_buttons: HashSet::new(),
            is_window_focused: false,
            accessibility_nodes: AccessibilityNodes::new(),
            mouse_entered_widgets: Vec::new(),
            current_mouse_event_state: None,
            pixmap: Rc::new(RefCell::new(Pixmap::new(1, 1).unwrap())),
            surface: None,
            softbuffer_context: None,
            accesskit_adapter: None,
            winit_window: None,
            input_method_enabled: false,
            ime_cursor_area: Rect::default(),
            pending_size_hint_invalidations: Vec::new(),
            pending_redraw: false,
            pending_accessibility_updates: Vec::new(),
            focusable_widgets: Vec::new(),
            focusable_widgets_changed: false,
            focused_widget: None,
            mouse_grabber_widget: None,
            num_clicks: 0,
            last_click_button: None,
            last_click_instant: None,
            is_delete_widget_on_close_enabled: true,
            // This is updated in `init_window`
            min_inner_size: Size::default(),
            // This is updated in `init_window`
            preferred_inner_size: Size::default(),
            attributes: Attributes::default(),
            event_loop_proxy: app.event_loop_proxy(),
        })))
    }

    pub fn try_remove(self) -> bool {
        Rc::try_unwrap(self.0).is_ok()
    }

    pub fn has_winit_window(&self) -> bool {
        self.0.borrow().winit_window.is_some()
    }

    pub fn init_winit_window(&self, root_widget: &mut dyn Widget) {
        if self.0.borrow().winit_window.is_some() {
            return;
        }
        let attributes: Attributes = self.0.borrow().attributes.clone();

        let mut monitor = None;
        if let Some(outer_position) = attributes.outer_position {
            for monitor in root_widget.base().app().available_monitors() {
                if monitor.rect().contains(outer_position) {
                    break;
                }
            }
        }
        if monitor.is_none() {
            monitor = root_widget
                .base()
                .app()
                .primary_monitor()
                .or_else(|| root_widget.base().app().available_monitors().next());
        }

        if let Some(monitor) = &monitor {
            if root_widget.base().app().config().fixed_scale.is_none() {
                root_widget.set_scale(Some(monitor.scale_factor() as f32));
            }
        }

        let size_hints_x = root_widget.size_hint_x(None);

        let monitor_work_area = monitor.as_ref().map(|m| m.work_area());
        let size_x = if let Some(monitor_rect) = monitor_work_area {
            max(
                size_hints_x.min(),
                min(size_hints_x.preferred(), monitor_rect.size_x()),
            )
        } else {
            max(size_hints_x.min(), size_hints_x.preferred())
        };

        let size_hint_y_min = root_widget.size_hint_y(size_hints_x.min()).min();
        let size_hint_y_preferred = root_widget
            .size_hint_y(size_hints_x.preferred())
            .preferred();
        let size_y = if let Some(monitor_rect) = monitor_work_area {
            max(
                size_hint_y_min,
                min(size_hint_y_preferred, monitor_rect.size_y()),
            )
        } else {
            max(size_hint_y_min, size_hint_y_preferred)
        };
        let mut size = Size::new(size_x, size_y);
        let min_size = Size::new(size_hints_x.min(), size_hint_y_min);
        trace!("window content min size hint: {min_size:?}");
        trace!("window content preferred size hint: {size:?}");

        trace!("requested window position: {:?}", attributes.outer_position);
        let mut position = attributes.outer_position.map(|position| {
            if let Some(monitor_rect) = monitor_work_area {
                Point::new(
                    position
                        .x()
                        .clamp(monitor_rect.left(), monitor_rect.right() - size.x()),
                    position
                        .y()
                        .clamp(monitor_rect.top(), monitor_rect.bottom() - size.y()),
                )
            } else {
                position
            }
        });
        trace!(
            "window position adjusted to monitor work area: {:?}",
            position
        );

        if let Some(window_rect_response) = root_widget
            .handle_window_rect_request(WindowRectRequest {
                suggested_position: position,
                suggested_size: size,
                monitor,
            })
            .or_warn()
        {
            if let Some(new_position) = window_rect_response.position {
                position = Some(new_position);
                trace!(
                    "window position adjusted to widget response: {:?}",
                    position
                );
            }
            if let Some(new_size) = window_rect_response.size {
                size = new_size;
            }
        }

        let mut attrs = WindowAttributes::default()
            .with_inner_size(PhysicalSize::from(size))
            .with_min_inner_size(PhysicalSize::from(min_size))
            .with_resizable(attributes.resizable)
            .with_enabled_buttons(attributes.enabled_buttons)
            .with_maximized(attributes.maximized)
            // Window must be hidden until we initialize accesskit
            .with_visible(false)
            .with_transparent(attributes.transparent)
            .with_blur(attributes.blur)
            .with_decorations(attributes.decorations)
            .with_window_icon(attributes.window_icon.clone())
            .with_theme(attributes.preferred_theme)
            .with_content_protected(attributes.content_protected)
            .with_window_level(attributes.window_level)
            .with_fullscreen(attributes.fullscreen.clone());

        if let Some(title) = &attributes.title {
            attrs = attrs.with_title(title);
        }
        if let Some(active) = attributes.active {
            attrs = attrs.with_active(active);
        }
        if let Some(position) = position {
            attrs = attrs.with_position(PhysicalPosition::from(position));
        }
        if let Some(resize_increments) = attributes.resize_increments {
            attrs = attrs.with_resize_increments(PhysicalSize::from(resize_increments));
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(has_shadow) = attributes.has_macos_shadow {
                attrs = attrs.with_has_shadow(has_shadow);
            }
        }
        #[cfg(all(unix, not(target_vendor = "apple")))]
        {
            if let Some(v) = &attributes.x11_window_type {
                attrs = attrs.with_x11_window_type(v.iter().copied().map(Into::into).collect());
            }
        }
        #[cfg(windows)]
        {
            if let Some(v) = attributes.skip_windows_taskbar {
                attrs = attrs.with_skip_taskbar(v);
            }
        }
        let winit_window = Rc::new(with_active_event_loop(|event_loop| {
            event_loop.create_window(attrs).unwrap()
        }));
        let softbuffer_context = softbuffer::Context::new(winit_window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&softbuffer_context, winit_window.clone()).unwrap();
        let accesskit_adapter = with_active_event_loop(|event_loop| {
            accesskit_winit::Adapter::with_event_loop_proxy(
                event_loop,
                &winit_window,
                root_widget.base().app().event_loop_proxy(),
            )
        });
        winit_window.set_visible(attributes.visible);
        trace!(
            "real window position after creation: {:?}",
            winit_window.outer_position(),
        );
        let winit_id = winit_window.id();
        {
            let mut inner = self.0.borrow_mut();
            inner.winit_window = Some(winit_window.clone());
            inner.softbuffer_context = Some(softbuffer_context);
            inner.surface = Some(surface);
            inner.accesskit_adapter = Some(Mutex::new(accesskit_adapter));
            inner.min_inner_size = min_size;
            inner.attributes.outer_position = None;
        }

        root_widget
            .base()
            .app()
            .add_winit_window(winit_id, self.id());
        if root_widget.base().app().config().fixed_scale.is_none() {
            if root_widget.base().self_scale().is_some_and(|widget_scale| {
                (widget_scale - winit_window.scale_factor() as f32).abs() >= 0.1
            }) {
                info!(
                    "rescaling widget after creating window: {:?} -> {}",
                    root_widget.base().self_scale(),
                    winit_window.scale_factor(),
                );
                root_widget.set_scale(Some(winit_window.scale_factor() as f32));
            }
        }
        self.set_visible(true);
        // For some reason it's necessary to request redraw again after initializing accesskit on Windows.
        self.clear_pending_redraw();
        self.request_redraw();
    }

    pub fn id(&self) -> WindowId {
        self.0.borrow().id
    }

    pub(crate) fn winit_id(&self) -> Option<winit::window::WindowId> {
        self.0.borrow().winit_window.as_ref().map(|w| w.id())
    }

    pub fn root_widget_id(&self) -> RawWidgetId {
        self.0.borrow().root_widget_id
    }

    pub(crate) fn root_accessibility_node_id(&self) -> NodeId {
        self.0.borrow().accessibility_nodes.root()
    }

    pub(crate) fn update_accessibility_node(&self, parent: Option<NodeId>, child: NodeId) {
        self.0
            .borrow_mut()
            .accessibility_nodes
            .update_node(parent, child);
    }

    pub(crate) fn remove_accessibility_node(
        &self,
        parent: Option<NodeId>,
        child: NodeId,
        key_in_parent: ChildKey,
    ) {
        self.0
            .borrow_mut()
            .accessibility_nodes
            .remove_node(parent, child, key_in_parent);
    }

    pub(crate) fn accessibility_node_updated(&self, id: NodeId, node: Option<accesskit::Node>) {
        self.0.borrow_mut().accessibility_nodes.update(id, node);
    }

    pub fn mouse_grabber_widget(&self) -> Option<RawWidgetId> {
        self.0.borrow().mouse_grabber_widget
    }

    pub(crate) fn set_mouse_grabber_widget(&self, id: Option<RawWidgetId>) {
        self.0.borrow_mut().mouse_grabber_widget = id;
    }

    pub fn focused_widget(&self) -> Option<RawWidgetId> {
        self.0.borrow().focused_widget.as_ref().map(|x| x.1)
    }

    pub fn cursor_position(&self) -> Option<Point> {
        self.0.borrow().cursor_position
    }

    pub fn inner_position(&self) -> anyhow::Result<Point> {
        let inner = self.0.borrow();
        let window = inner
            .winit_window
            .as_ref()
            .context("native window is not created yet")?;
        let position = window
            .inner_position()
            .context("winit window position is unsupported")?;
        Ok(position.into())
    }

    pub fn outer_position(&self) -> anyhow::Result<Point> {
        let this = self.0.borrow();
        if let Some(window) = &this.winit_window {
            let position = window
                .outer_position()
                .context("winit window position is unsupported")?;
            Ok(position.into())
        } else {
            this.attributes
                .outer_position
                .context("native window is not created yet")
        }
    }

    pub fn set_outer_position(&self, position: Point) {
        let mut this = self.0.borrow_mut();
        if let Some(window) = &this.winit_window {
            window.set_outer_position(PhysicalPosition::from(position));
        }
        this.attributes.outer_position = Some(position);
    }

    pub(crate) fn num_clicks(&self) -> u32 {
        self.0.borrow().num_clicks
    }

    pub(crate) fn any_mouse_buttons_pressed(&self) -> bool {
        !self.0.borrow().pressed_mouse_buttons.is_empty()
    }

    pub(crate) fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.0.borrow().pressed_mouse_buttons.contains(&button)
    }

    pub fn is_delete_widget_on_close_enabled(&self) -> bool {
        self.0.borrow().is_delete_widget_on_close_enabled
    }

    // Private: visibility should be set on the window's root widget
    pub(crate) fn set_visible(&self, visible: bool) {
        let mut this = self.0.borrow_mut();
        if let Some(w) = this.winit_window.as_ref() {
            w.set_visible(visible);
        }
        this.attributes.visible = visible;
    }

    pub(crate) fn set_cursor(&self, icon: CursorIcon) {
        if let Some(w) = self.0.borrow().winit_window.as_ref() {
            w.set_cursor(icon);
        }
    }

    pub fn close(&self) {
        // TODO: add option to confirm close or do something else
        if self.is_delete_widget_on_close_enabled() {
            let event = UserEvent::DeleteWidget(self.root_widget_id());
            let _ = self.0.borrow().event_loop_proxy.send_event(event);
        }
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.0.borrow().modifiers_state
    }

    pub(crate) fn set_modifiers(&self, modifiers: ModifiersState) {
        self.0.borrow_mut().modifiers_state = modifiers;
    }

    pub fn inner_size(&self) -> Size {
        self.0
            .borrow()
            .winit_window
            .as_ref()
            .map(|w| w.inner_size())
            .unwrap_or_default()
            .into()
    }

    pub fn min_inner_size(&self) -> Size {
        self.0.borrow().min_inner_size
    }

    pub fn preferred_inner_size(&self) -> Size {
        self.0.borrow().preferred_inner_size
    }

    pub fn scale(&self) -> f32 {
        let this = self.0.borrow();
        // TODO: get expected scale based on monitor
        this.winit_window
            .as_ref()
            .map_or(1.0, |w| w.scale_factor() as f32)
    }

    pub(crate) fn set_min_inner_size(&self, size: Size) {
        let this = &mut *self.0.borrow_mut();
        if size != this.min_inner_size {
            if let Some(w) = this.winit_window.as_ref() {
                w.set_min_inner_size(Some(PhysicalSize::from(size)));
            }
            this.min_inner_size = size;
        }
    }

    pub fn set_preferred_inner_size(&self, size: Size) {
        self.0.borrow_mut().preferred_inner_size = size;
    }

    pub fn request_inner_size(&self, size: Size) -> Option<Size> {
        self.0
            .borrow()
            .winit_window
            .as_ref()
            .and_then(|w| w.request_inner_size(PhysicalSize::from(size)))
            .map(Into::into)
    }

    pub(crate) fn clear_pending_redraw(&self) {
        self.0.borrow_mut().pending_redraw = false;
    }

    pub fn is_focused(&self) -> bool {
        self.0.borrow().is_window_focused
    }

    pub(crate) fn pass_event_to_accesskit(&self, event: &WindowEvent) {
        let this = &mut *self.0.borrow_mut();
        if let Some(adapter) = this.accesskit_adapter.as_ref() {
            let Ok(mut adapter) = adapter.lock() else {
                return;
            };
            adapter.process_event(this.winit_window.as_ref().unwrap(), event);
        }
    }

    pub(crate) fn prepare_draw(&self, app: &App) -> Option<DrawEvent> {
        let this = &mut *self.0.borrow_mut();
        if this.winit_window.is_none() {
            warn!("cannot draw without a window");
            return None;
        }
        let (width, height) = {
            let size = this.winit_window.as_ref().unwrap().inner_size();
            (
                size.width + EXTRA_SURFACE_SIZE,
                size.height + EXTRA_SURFACE_SIZE,
            )
        };

        this.surface
            .as_mut()
            .unwrap()
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        {
            let mut pixmap = this.pixmap.borrow_mut();
            if pixmap.width() != width || pixmap.height() != height {
                *pixmap = Pixmap::new(width, height).unwrap();
            }
        }

        if !this.pending_redraw {
            return None;
        }
        let draw_event = DrawEvent::new(
            Rc::clone(&this.pixmap),
            Point::default(),
            Rect::from_pos_size(
                Point::default(),
                Size::new(
                    PhysicalPixels::from_i32(width as i32),
                    PhysicalPixels::from_i32(height as i32),
                ),
            ),
        );
        // TODO: option to turn off background, allow customizing with classes or inline style
        let color = app.style().root_background_color();
        this.pixmap.borrow_mut().fill(color);
        this.pending_redraw = false;
        Some(draw_event)
    }

    pub(crate) fn finalize_draw(&self) {
        let this = &mut *self.0.borrow_mut();
        if this.winit_window.is_none() {
            warn!("cannot draw without a window");
            return;
        }
        let mut buffer = this.surface.as_mut().unwrap().buffer_mut().unwrap();

        {
            let pixmap = this.pixmap.borrow();

            let pixmap_data: &[u32] = bytemuck::cast_slice(pixmap.data());
            for (dest, src) in buffer.iter_mut().zip(pixmap_data) {
                // tiny-skia uses an RGBA format, while softbuffer uses XRGB.
                *dest = src.swap_bytes() >> 8;
            }
        }

        buffer.present().unwrap();
    }

    pub(crate) fn cursor_entered(&self) {
        let this = &mut *self.0.borrow_mut();
        this.cursor_entered = true;
    }

    pub(crate) fn cursor_left(&self) {
        let this = &mut *self.0.borrow_mut();
        this.cursor_entered = false;
        this.cursor_position = None;
    }

    pub(crate) fn cursor_moved(&self, pos_in_window: Point) -> bool {
        let this = &mut *self.0.borrow_mut();
        if this.cursor_position != Some(pos_in_window) {
            this.cursor_position = Some(pos_in_window);
            true
        } else {
            false
        }
    }

    pub(crate) fn mouse_input(&self, state: ElementState, button: MouseButton) {
        let this = &mut *self.0.borrow_mut();
        match state {
            ElementState::Pressed => {
                this.pressed_mouse_buttons.insert(button);
                let had_recent_click = this
                    .last_click_instant
                    .is_some_and(|last| last.elapsed() < DOUBLE_CLICK_TIMEOUT);
                let same_button = this.last_click_button == Some(button);
                if had_recent_click && same_button {
                    this.num_clicks += 1;
                } else {
                    this.num_clicks = 1;
                    this.last_click_button = Some(button);
                }
                this.last_click_instant = Some(Instant::now());
            }
            ElementState::Released => {
                this.pressed_mouse_buttons.remove(&button);
            }
        }
    }

    pub(crate) fn is_mouse_entered(&self, id: RawWidgetId) -> bool {
        self.0
            .borrow()
            .mouse_entered_widgets
            .iter()
            .any(|(_, x)| *x == id)
    }

    pub(crate) fn add_mouse_entered(&self, rect: Rect, id: RawWidgetId) {
        self.0.borrow_mut().mouse_entered_widgets.push((rect, id));
    }

    pub(crate) fn ime_enabled(&self) {
        let this = &*self.0.borrow();
        if let Some(w) = this.winit_window.as_ref() {
            w.set_ime_cursor_area(
                PhysicalPosition::from(this.ime_cursor_area.top_left()),
                PhysicalSize::from(this.ime_cursor_area.size()),
            );
        }
    }

    pub(crate) fn focus_changed(&self, focused: bool) -> bool {
        let this = &mut *self.0.borrow_mut();
        this.is_window_focused = focused;
        if !focused && this.mouse_grabber_widget.is_some() {
            this.mouse_grabber_widget = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn pop_mouse_entered_widget(&self) -> Option<RawWidgetId> {
        let this = &mut *self.0.borrow_mut();
        let pos = this.cursor_position;
        let list = &mut this.mouse_entered_widgets;
        let index = list.iter().position(|(rect, id)| {
            pos.is_none_or(|pos| !rect.contains(pos)) && Some(*id) != this.mouse_grabber_widget
        })?;
        Some(list.remove(index).1)
    }

    pub fn set_ime_cursor_area(&self, rect: Rect) {
        let mut this = self.0.borrow_mut();
        if this.ime_cursor_area != rect {
            if let Some(w) = this.winit_window.as_ref() {
                w.set_ime_cursor_area(
                    PhysicalPosition::from(rect.top_left()),
                    PhysicalSize::from(rect.size()),
                );
            } //TODO: actual size
            this.ime_cursor_area = rect;
        }
    }

    // TODO: should there be a proper way to do this in winit?
    pub fn cancel_ime_preedit(&self) {
        let this = self.0.borrow();
        if this.input_method_enabled {
            if let Some(w) = this.winit_window.as_ref() {
                w.set_ime_allowed(false);
                w.set_ime_allowed(true);
            }
        }
    }

    // Only for the case when ime_allowed is changed for the focused widget.
    pub fn set_ime_allowed(&self, allowed: bool) {
        let this = self.0.borrow();
        if let Some(w) = this.winit_window.as_ref() {
            w.set_ime_allowed(allowed);
        }
    }

    pub fn request_redraw(&self) {
        let mut this = self.0.borrow_mut();
        if !this.pending_redraw {
            this.pending_redraw = true;
            if let Some(w) = this.winit_window.as_ref() {
                w.request_redraw();
            }
        }
    }

    pub fn add_focusable_widget(&self, addr: WidgetAddress, id: RawWidgetId) {
        let mut this = self.0.borrow_mut();
        let Some(relative_addr) = addr.strip_prefix(this.root_widget_id) else {
            warn!("add_focusable_widget: address outside root");
            return;
        };
        let pair = (relative_addr.to_vec(), id);
        if let Err(index) = this.focusable_widgets.binary_search(&pair) {
            this.focusable_widgets.insert(index, pair);
            this.focusable_widgets_changed = true;
        }
    }

    pub fn remove_focusable_widget(&self, addr: WidgetAddress, id: RawWidgetId) {
        let mut this = self.0.borrow_mut();
        let Some(relative_addr) = addr.strip_prefix(this.root_widget_id) else {
            warn!("remove_focusable_widget: address outside root");
            return;
        };
        let pair = (relative_addr.to_vec(), id);
        if let Ok(index) = this.focusable_widgets.binary_search(&pair) {
            this.focusable_widgets.remove(index);
            this.focusable_widgets_changed = true;
        }
    }

    pub(crate) fn move_keyboard_focus(
        &self,
        direction: i32,
    ) -> Option<(Vec<(ChildKey, RawWidgetId)>, RawWidgetId)> {
        let this = self.0.borrow();
        let focused_widget = this.focused_widget.as_ref()?;
        if this.focusable_widgets.is_empty() {
            None
        } else if let Ok(index) = this.focusable_widgets.binary_search(focused_widget) {
            let new_index =
                (index as i32 + direction).rem_euclid(this.focusable_widgets.len() as i32);
            Some(this.focusable_widgets[new_index as usize].clone())
        } else {
            warn!("focused widget is unknown");
            this.focusable_widgets.first().cloned()
        }
    }

    pub(crate) fn pending_auto_focus(&self) -> Option<(Vec<(ChildKey, RawWidgetId)>, RawWidgetId)> {
        let this = &*self.0.borrow();
        if this.focused_widget.is_none() {
            this.focusable_widgets.first().cloned()
        } else {
            None
        }
    }

    pub(crate) fn push_accessibility_updates(&mut self) {
        let this = &mut *self.0.borrow_mut();
        if let Some(adapter) = this.accesskit_adapter.as_ref() {
            let update = this.accessibility_nodes.take_update();
            let r = catch_unwind(|| {
                adapter
                    .lock()
                    .expect("accesskit adapter mutex is poisoned")
                    .update_if_active(|| update);
            });
            if let Err(err) = r {
                let err_msg = err
                    .downcast_ref::<&'static str>()
                    .map(|s| s.to_string())
                    .or_else(|| err.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| format!("{err:?}"));
                warn!("accesskit panicked: {err_msg}");
                this.accesskit_adapter = None;
            }
        }
    }

    pub(crate) fn invalidate_size_hint(&self, addr: WidgetAddress) {
        let this = &mut *self.0.borrow_mut();
        this.pending_size_hint_invalidations.push(addr);
    }

    pub(crate) fn take_pending_size_hint_invalidations(&self) -> Vec<WidgetAddress> {
        mem::take(&mut self.0.borrow_mut().pending_size_hint_invalidations)
    }

    pub(crate) fn request_accessibility_update(&self, addr: WidgetAddress) {
        let this = &mut *self.0.borrow_mut();
        this.pending_accessibility_updates.push(addr);
    }

    pub(crate) fn take_pending_accessibility_updates(&self) -> Vec<WidgetAddress> {
        mem::take(&mut self.0.borrow_mut().pending_accessibility_updates)
    }

    pub(crate) fn unset_focus(&self) -> Option<(Vec<(ChildKey, RawWidgetId)>, RawWidgetId)> {
        let mut this = self.0.borrow_mut();
        let old = this.focused_widget.take();
        if let Some(w) = this.winit_window.as_ref() {
            w.set_ime_allowed(false);
        }
        this.input_method_enabled = false;
        this.accessibility_nodes.set_focus(None);
        old
    }

    pub(crate) fn set_focus(
        &self,
        addr_id: (Vec<(ChildKey, RawWidgetId)>, RawWidgetId),
        input_method_enabled: bool,
    ) {
        let mut this = self.0.borrow_mut();
        this.accessibility_nodes.set_focus(Some(addr_id.1.into()));
        this.focused_widget = Some(addr_id);
        if let Some(w) = this.winit_window.as_ref() {
            w.set_ime_allowed(input_method_enabled);
        }
        this.input_method_enabled = input_method_enabled;
    }

    pub(crate) fn is_registered_as_focusable(
        &self,
        addr_id: &(Vec<(ChildKey, RawWidgetId)>, RawWidgetId),
    ) -> bool {
        let this = &*self.0.borrow();
        this.focusable_widgets.binary_search(addr_id).is_ok()
    }

    pub(crate) fn focused_widget_is_focusable(&self) -> bool {
        let this = &*self.0.borrow();
        if let Some(focused_widget) = &this.focused_widget {
            this.focusable_widgets.binary_search(focused_widget).is_ok()
        } else {
            true
        }
    }

    pub(crate) fn focusable_widgets_changed(&self) -> bool {
        let this = &*self.0.borrow();
        this.focusable_widgets_changed
    }

    pub(crate) fn clear_focusable_widgets_changed(&self) {
        let this = &mut *self.0.borrow_mut();
        this.focusable_widgets_changed = false;
    }

    pub(crate) fn current_mouse_event_state(&self) -> anyhow::Result<MouseEventState> {
        let this = &*self.0.borrow();
        this.current_mouse_event_state
            .context("no current mouse event")
    }

    pub(crate) fn accept_current_mouse_event(&self, widget_id: RawWidgetId) -> anyhow::Result<()> {
        let this = &mut *self.0.borrow_mut();
        if let Some(state) = &mut this.current_mouse_event_state {
            if let MouseEventState::NotAccepted = state {
                *state = MouseEventState::AcceptedBy(widget_id);
                Ok(())
            } else {
                bail!("event already accepted");
            }
        } else {
            bail!("no current mouse event");
        }
    }

    pub(crate) fn init_mouse_event_state(&self) -> anyhow::Result<()> {
        let this = &mut *self.0.borrow_mut();
        if this.current_mouse_event_state.is_none() {
            this.current_mouse_event_state = Some(MouseEventState::NotAccepted);
            Ok(())
        } else {
            bail!("window already has another current mouse event");
        }
    }

    pub(crate) fn take_mouse_event_state(&self) -> anyhow::Result<MouseEventState> {
        let this = &mut *self.0.borrow_mut();
        this.current_mouse_event_state
            .take()
            .context("no current mouse event")
    }

    pub fn set_title(&self, title: impl Display) {
        let title = title.to_string();
        let this = &mut *self.0.borrow_mut();
        if Some(&title) == this.attributes.title.as_ref() {
            return;
        }
        if let Some(window) = &this.winit_window {
            window.set_title(&title);
        }
        this.attributes.title = Some(title);
    }

    pub fn set_decorations(&self, value: bool) {
        let this = &mut *self.0.borrow_mut();
        if value == this.attributes.decorations {
            return;
        }
        if let Some(window) = &this.winit_window {
            window.set_decorations(value);
        }
        this.attributes.decorations = value;
    }

    pub fn set_resizable(&self, value: bool) {
        let this = &mut *self.0.borrow_mut();
        if value == this.attributes.resizable {
            return;
        }
        if let Some(window) = &this.winit_window {
            window.set_resizable(value);
        }
        this.attributes.resizable = value;
    }

    pub fn is_resizable(&self) -> bool {
        self.0.borrow().attributes.resizable
    }

    pub fn set_window_level(&self, value: WindowLevel) {
        let this = &mut *self.0.borrow_mut();
        if value == this.attributes.window_level {
            return;
        }
        if let Some(window) = &this.winit_window {
            window.set_window_level(value);
        }
        this.attributes.window_level = value;
    }

    #[allow(unused_variables)]
    pub fn set_has_macos_shadow(&self, value: bool) {
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowExtMacOS;

            let this = &mut *self.0.borrow_mut();
            if Some(&value) == this.attributes.has_macos_shadow.as_ref() {
                return;
            }
            if let Some(window) = &this.winit_window {
                window.set_has_shadow(value);
            }
            this.attributes.has_macos_shadow = Some(value);
        }
    }

    #[allow(unused_variables)]
    pub fn set_x11_window_type(&self, value: Vec<X11WindowType>) {
        #[cfg(all(unix, not(target_vendor = "apple")))]
        {
            let this = &mut *self.0.borrow_mut();
            if Some(&value) == this.attributes.x11_window_type.as_ref() {
                return;
            }
            if this.winit_window.is_some() {
                warn!("changing x11 window type after window creation is unsupported");
            }
            this.attributes.x11_window_type = Some(value);
        }
    }

    #[allow(unused_variables)]
    pub fn set_skip_windows_taskbar(&self, value: bool) {
        #[cfg(windows)]
        {
            use winit::platform::windows::WindowExtWindows;

            let this = &mut *self.0.borrow_mut();
            if Some(value) == this.attributes.skip_windows_taskbar {
                return;
            }
            if let Some(window) = &this.winit_window {
                window.set_skip_taskbar(value);
            }
            this.attributes.skip_windows_taskbar = Some(value);
        }
    }
}

#[derive(Debug, From)]
pub enum WindowRequest {
    SetFocus(SetFocusRequest),
    ScrollToRect(ScrollToRectRequest),
}

#[derive(Debug)]
pub struct SetFocusRequest {
    pub widget_id: RawWidgetId,
    pub reason: FocusReason,
}

#[derive(Debug)]
pub struct ScrollToRectRequest {
    pub widget_id: RawWidgetId,
    // In widget coordinates.
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum X11WindowType {
    Desktop,
    Dock,
    Toolbar,
    Menu,
    Utility,
    Splash,
    Dialog,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
    Combo,
    Dnd,
    Normal,
}

#[cfg(all(unix, not(target_vendor = "apple")))]
impl From<X11WindowType> for winit::platform::x11::WindowType {
    fn from(value: X11WindowType) -> Self {
        match value {
            X11WindowType::Desktop => winit::platform::x11::WindowType::Desktop,
            X11WindowType::Dock => winit::platform::x11::WindowType::Dock,
            X11WindowType::Toolbar => winit::platform::x11::WindowType::Toolbar,
            X11WindowType::Menu => winit::platform::x11::WindowType::Menu,
            X11WindowType::Utility => winit::platform::x11::WindowType::Utility,
            X11WindowType::Splash => winit::platform::x11::WindowType::Splash,
            X11WindowType::Dialog => winit::platform::x11::WindowType::Dialog,
            X11WindowType::DropdownMenu => winit::platform::x11::WindowType::DropdownMenu,
            X11WindowType::PopupMenu => winit::platform::x11::WindowType::PopupMenu,
            X11WindowType::Tooltip => winit::platform::x11::WindowType::Tooltip,
            X11WindowType::Notification => winit::platform::x11::WindowType::Notification,
            X11WindowType::Combo => winit::platform::x11::WindowType::Combo,
            X11WindowType::Dnd => winit::platform::x11::WindowType::Dnd,
            X11WindowType::Normal => winit::platform::x11::WindowType::Normal,
        }
    }
}
