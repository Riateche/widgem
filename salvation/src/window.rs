use std::{cell::RefCell, collections::HashSet, rc::Rc, time::Instant};

use derivative::Derivative;
use derive_more::From;
use log::warn;
use tiny_skia::Pixmap;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::MouseButton,
    keyboard::ModifiersState,
    window::WindowId,
};

use crate::{
    accessible::AccessibleNodes,
    event::{FocusReason, MouseLeaveEvent},
    types::{Point, Rect, Size},
    widgets::{get_widget_by_id_mut, RawWidgetId, Widget, WidgetAddress, WidgetExt},
};

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
    pub pixmap: Rc<RefCell<Pixmap>>,
    // Drop order must be maintained as
    // `surface` -> `softbuffer_context` -> `winit_window`
    // to maintain safety requirements.
    #[derivative(Debug = "ignore")]
    pub surface: softbuffer::Surface<Rc<winit::window::Window>, Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub softbuffer_context: softbuffer::Context<Rc<winit::window::Window>>,
    #[derivative(Debug = "ignore")]
    pub accesskit_adapter: accesskit_winit::Adapter,
    pub winit_window: Rc<winit::window::Window>,
    pub min_size: Size,
    pub preferred_size: Size,
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
    pub delete_widget_on_close: bool,
}

#[derive(Debug, Clone)]
pub struct Window(pub Rc<RefCell<WindowInner>>);

impl Window {
    pub fn pop_mouse_entered_widget(&self) -> Option<RawWidgetId> {
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
        focused_widget: &(Vec<(usize, RawWidgetId)>, RawWidgetId),
        direction: i32,
    ) -> Option<(Vec<(usize, RawWidgetId)>, RawWidgetId)> {
        let this = self.0.borrow();
        if this.focusable_widgets.is_empty() {
            return None;
        }
        if let Ok(index) = this.focusable_widgets.binary_search(focused_widget) {
            let new_index =
                (index as i32 + direction).rem_euclid(this.focusable_widgets.len() as i32);
            Some(this.focusable_widgets[new_index as usize].clone())
        } else {
            warn!("focused widget is unknown");
            this.focusable_widgets.first().cloned()
        }
    }

    pub(crate) fn dispatch_cursor_leave(&mut self, root_widget: &mut dyn Widget) {
        while let Some(id) = self.pop_mouse_entered_widget() {
            if let Ok(widget) = get_widget_by_id_mut(root_widget, id) {
                widget.dispatch(MouseLeaveEvent::new().into());
            }
        }
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
