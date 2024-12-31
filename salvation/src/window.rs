use {
    crate::{
        accessible::AccessibleNodes,
        draw::DrawEvent,
        event::FocusReason,
        event_loop::with_active_event_loop,
        layout::SizeHints,
        system::with_system,
        types::{Point, Rect, Size},
        widgets::{RawWidgetId, WidgetAddress},
    },
    accesskit::{NodeBuilder, NodeId},
    anyhow::{bail, Context},
    derivative::Derivative,
    derive_more::From,
    log::warn,
    std::{
        cell::RefCell,
        collections::HashSet,
        mem,
        num::NonZeroU32,
        rc::Rc,
        time::{Duration, Instant},
    },
    tiny_skia::Pixmap,
    winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{ElementState, MouseButton, WindowEvent},
        keyboard::ModifiersState,
        window::{CursorIcon, WindowAttributes, WindowId},
    },
};

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

#[derive(Derivative)]
#[derivative(Debug)]
pub struct WindowInner {
    pub id: WindowId,
    pub root_widget_id: RawWidgetId,
    pub cursor_position: Option<Point>,
    pub cursor_entered: bool,
    pub modifiers_state: ModifiersState,
    pub pressed_mouse_buttons: HashSet<MouseButton>,
    pub is_window_focused: bool,
    pub accessible_nodes: AccessibleNodes,
    pub mouse_entered_widgets: Vec<(Rect, RawWidgetId)>,
    pub pending_size_hint_invalidations: Vec<WidgetAddress>,
    pub current_mouse_event_state: Option<MouseEventState>,
    pub pixmap: Rc<RefCell<Pixmap>>,
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `winit_window`.
    #[derivative(Debug = "ignore")]
    pub surface: softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub softbuffer_context: softbuffer::Context<Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub accesskit_adapter: accesskit_winit::Adapter,
    pub winit_window: Rc<winit::window::Window>,
    pub min_inner_size: Size,
    pub preferred_inner_size: Size,
    pub ime_allowed: bool,
    pub ime_cursor_area: Rect,

    pub pending_redraw: bool,

    // TODO: refactor as struct
    pub focusable_widgets: Vec<(Vec<(usize, RawWidgetId)>, RawWidgetId)>,
    pub focusable_widgets_changed: bool,

    pub focused_widget: Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)>,
    pub mouse_grabber_widget: Option<RawWidgetId>,
    pub num_clicks: u32,
    pub last_click_button: Option<MouseButton>,
    pub last_click_instant: Option<Instant>,
    pub is_delete_widget_on_close_enabled: bool,
}

impl WindowInner {
    fn unset_focus(&mut self) -> Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)> {
        let old = self.focused_widget.take();
        self.winit_window.set_ime_allowed(false);
        self.ime_allowed = false;
        self.accessible_nodes.set_focus(None);
        old
    }
}

#[derive(Debug, Clone)]
pub struct Window(Rc<RefCell<WindowInner>>);

impl Window {
    pub(crate) fn new(
        mut attrs: WindowAttributes,
        root_widget_id: RawWidgetId,
        size_hints_x: SizeHints,
        size_hints_y: SizeHints,
    ) -> Self {
        let preferred_size = Size::new(size_hints_x.preferred, size_hints_y.preferred);
        let min_size = Size::new(size_hints_x.min, size_hints_y.min);
        attrs = attrs
            .with_inner_size(PhysicalSize::new(preferred_size.x, preferred_size.y))
            .with_min_inner_size(PhysicalSize::new(min_size.x, min_size.y));
        let winit_window = Rc::new(with_active_event_loop(|event_loop| {
            event_loop.create_window(attrs).unwrap()
        }));
        let softbuffer_context = softbuffer::Context::new(winit_window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&softbuffer_context, winit_window.clone()).unwrap();
        let accesskit_adapter = accesskit_winit::Adapter::with_event_loop_proxy(
            &winit_window,
            with_system(|system| system.event_loop_proxy.clone()),
        );
        let inner_size = winit_window.inner_size();

        let id = winit_window.id();
        Window(Rc::new(RefCell::new(WindowInner {
            id,
            root_widget_id,
            cursor_position: None,
            cursor_entered: false,
            modifiers_state: ModifiersState::default(),
            pressed_mouse_buttons: HashSet::new(),
            is_window_focused: false,
            accessible_nodes: AccessibleNodes::new(),
            mouse_entered_widgets: Vec::new(),
            pending_size_hint_invalidations: Vec::new(),
            current_mouse_event_state: None,
            pixmap: Rc::new(RefCell::new(
                Pixmap::new(
                    inner_size.width + EXTRA_SURFACE_SIZE,
                    inner_size.height + EXTRA_SURFACE_SIZE,
                )
                .unwrap(),
            )),
            surface,
            softbuffer_context,
            accesskit_adapter,
            winit_window,
            ime_allowed: false,
            ime_cursor_area: Rect::default(),
            pending_redraw: false,
            focusable_widgets: Vec::new(),
            focusable_widgets_changed: false,
            focused_widget: None,
            mouse_grabber_widget: None,
            num_clicks: 0,
            last_click_button: None,
            last_click_instant: None,
            is_delete_widget_on_close_enabled: true,
            min_inner_size: min_size,
            preferred_inner_size: preferred_size,
        })))
    }

    pub fn id(&self) -> WindowId {
        self.0.borrow().id
    }

    pub fn root_widget_id(&self) -> RawWidgetId {
        self.0.borrow().root_widget_id
    }

    pub(crate) fn accessible_root(&self) -> NodeId {
        self.0.borrow().accessible_nodes.root()
    }

    pub(crate) fn accessible_unmount(&self, parent: Option<NodeId>, child: NodeId) {
        self.0.borrow_mut().accessible_nodes.unmount(parent, child);
    }

    pub(crate) fn accessible_mount(
        &self,
        parent: Option<NodeId>,
        child: NodeId,
        index_in_parent: usize,
    ) {
        self.0
            .borrow_mut()
            .accessible_nodes
            .mount(parent, child, index_in_parent);
    }

    pub(crate) fn accessible_update(&self, id: NodeId, node: Option<NodeBuilder>) {
        self.0.borrow_mut().accessible_nodes.update(id, node);
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

    pub fn set_visible(&self, visible: bool) {
        self.0.borrow_mut().winit_window.set_visible(visible);
    }

    pub(crate) fn set_cursor(&self, icon: CursorIcon) {
        self.0.borrow_mut().winit_window.set_cursor(icon);
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.0.borrow().modifiers_state
    }

    pub(crate) fn set_modifiers(&self, modifiers: ModifiersState) {
        self.0.borrow_mut().modifiers_state = modifiers;
    }

    pub fn inner_size(&self) -> Size {
        let inner_size = self.0.borrow().winit_window.inner_size();
        Size::new(inner_size.width as i32, inner_size.height as i32)
    }

    pub fn min_inner_size(&self) -> Size {
        self.0.borrow().min_inner_size
    }

    pub fn preferred_inner_size(&self) -> Size {
        self.0.borrow().preferred_inner_size
    }

    pub fn set_min_inner_size(&self, size: Size) {
        let this = &mut *self.0.borrow_mut();
        if size != this.min_inner_size {
            this.winit_window
                .set_min_inner_size(Some(PhysicalSize::new(size.x, size.y)));
            this.min_inner_size = size;
        }
    }

    pub fn set_preferred_inner_size(&self, size: Size) {
        self.0.borrow_mut().preferred_inner_size = size;
    }

    pub fn request_inner_size(&self, size: Size) -> Option<Size> {
        let size = PhysicalSize::new(size.x, size.y);
        let response = self.0.borrow().winit_window.request_inner_size(size);
        response.map(|size| Size::new(size.width as i32, size.height as i32))
    }

    pub(crate) fn clear_pending_redraw(&self) {
        self.0.borrow_mut().pending_redraw = false;
    }

    pub fn is_focused(&self) -> bool {
        self.0.borrow().is_window_focused
    }

    pub(crate) fn pass_event_to_accesskit(&self, event: &WindowEvent) {
        let this = &mut *self.0.borrow_mut();
        this.accesskit_adapter
            .process_event(&this.winit_window, event);
    }

    pub(crate) fn prepare_draw(&self) -> Option<DrawEvent> {
        let this = &mut *self.0.borrow_mut();
        let (width, height) = {
            let size = &this.winit_window.inner_size();
            (
                size.width + EXTRA_SURFACE_SIZE,
                size.height + EXTRA_SURFACE_SIZE,
            )
        };

        this.surface
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
            Rect {
                top_left: Point::default(),
                size: Size {
                    x: width as i32,
                    y: height as i32,
                },
            },
        );
        // TODO: option to turn off background, set style
        let color = with_system(|system| system.default_style.0.background);
        this.pixmap.borrow_mut().fill(color);
        this.pending_redraw = false;
        Some(draw_event)
    }

    pub(crate) fn finalize_draw(&self) {
        let this = &mut *self.0.borrow_mut();
        let mut buffer = this.surface.buffer_mut().unwrap();
        buffer.copy_from_slice(bytemuck::cast_slice(this.pixmap.borrow().data()));

        // tiny-skia uses an RGBA format, while softbuffer uses XRGB. To convert, we need to
        // iterate over the pixels and shift the pixels over.
        buffer.iter_mut().for_each(|pixel| {
            let [r, g, b, _] = pixel.to_ne_bytes();
            *pixel = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
        });

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
                    .map_or(false, |last| last.elapsed() < DOUBLE_CLICK_TIMEOUT);
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
        this.winit_window.set_ime_cursor_area(
            PhysicalPosition::new(
                this.ime_cursor_area.top_left.x,
                this.ime_cursor_area.top_left.y,
            ),
            PhysicalSize::new(this.ime_cursor_area.size.x, this.ime_cursor_area.size.y),
        );
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
            pos.map_or(true, |pos| !rect.contains(pos)) && Some(*id) != this.mouse_grabber_widget
        })?;
        Some(list.remove(index).1)
    }

    pub fn set_ime_cursor_area(&self, rect: Rect) {
        let mut this = self.0.borrow_mut();
        if this.ime_cursor_area != rect {
            this.winit_window.set_ime_cursor_area(
                PhysicalPosition::new(rect.top_left.x, rect.top_left.y),
                PhysicalSize::new(rect.size.x, rect.size.y),
            ); //TODO: actual size
            this.ime_cursor_area = rect;
        }
    }

    // TODO: should there be a proper way to do this in winit?
    pub fn cancel_ime_preedit(&self) {
        let this = self.0.borrow();
        if this.ime_allowed {
            this.winit_window.set_ime_allowed(false);
            this.winit_window.set_ime_allowed(true);
        }
    }

    pub fn request_redraw(&self) {
        let mut this = self.0.borrow_mut();
        if !this.pending_redraw {
            this.pending_redraw = true;
            this.winit_window.request_redraw();
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
    ) -> Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)> {
        let mut this = self.0.borrow_mut();
        let focused_widget = this.focused_widget.as_ref()?;
        let output = if this.focusable_widgets.is_empty() {
            None
        } else if let Ok(index) = this.focusable_widgets.binary_search(focused_widget) {
            let new_index =
                (index as i32 + direction).rem_euclid(this.focusable_widgets.len() as i32);
            Some(this.focusable_widgets[new_index as usize].clone())
        } else {
            warn!("focused widget is unknown");
            this.focusable_widgets.first().cloned()
        };
        if output.is_none() {
            this.unset_focus();
        }
        output
    }

    pub(crate) fn pending_auto_focus(&self) -> Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)> {
        let this = &*self.0.borrow();
        if this.focused_widget.is_none() {
            this.focusable_widgets.first().cloned()
        } else {
            None
        }
    }

    pub(crate) fn push_accessible_updates(&mut self) {
        let this = &mut *self.0.borrow_mut();
        this.accesskit_adapter
            .update_if_active(|| this.accessible_nodes.take_update());
    }

    pub(crate) fn invalidate_size_hint(&self, addr: WidgetAddress) {
        let this = &mut *self.0.borrow_mut();
        this.pending_size_hint_invalidations.push(addr);
    }

    pub(crate) fn take_pending_size_hint_invalidations(&self) -> Vec<WidgetAddress> {
        mem::take(&mut self.0.borrow_mut().pending_size_hint_invalidations)
    }

    pub(crate) fn unset_focus(&self) -> Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)> {
        let mut this = self.0.borrow_mut();
        this.unset_focus()
    }

    pub(crate) fn set_focus(
        &self,
        addr_id: (Vec<(usize, RawWidgetId)>, RawWidgetId),
        ime_allowed: bool,
    ) {
        let mut this = self.0.borrow_mut();
        this.accessible_nodes.set_focus(Some(addr_id.1.into()));
        this.focused_widget = Some(addr_id);
        this.winit_window.set_ime_allowed(ime_allowed);
        this.ime_allowed = ime_allowed;
    }

    pub(crate) fn is_registered_as_focusable(
        &self,
        addr_id: &(Vec<(usize, RawWidgetId)>, RawWidgetId),
    ) -> bool {
        let this = &*self.0.borrow();
        this.focusable_widgets.binary_search(addr_id).is_ok()
    }

    pub(crate) fn check_focus_after_widget_activity(&self) -> bool {
        let this = &mut *self.0.borrow_mut();
        if this.focusable_widgets_changed {
            this.focusable_widgets_changed = false;

            if let Some(focused_widget) = &this.focused_widget {
                if this
                    .focusable_widgets
                    .binary_search(focused_widget)
                    .is_err()
                {
                    this.unset_focus();
                }
            }
            true
        } else {
            false
        }
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
}

#[derive(Debug, From)]
pub enum WindowRequest {
    SetFocus(SetFocusRequest),
}

#[derive(Debug)]
pub struct SetFocusRequest {
    pub widget_id: RawWidgetId,
    pub reason: FocusReason,
}
