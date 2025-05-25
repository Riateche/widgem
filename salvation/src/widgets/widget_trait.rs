use {
    super::{WidgetCommon, WidgetCommonTyped},
    crate::{
        draw::DrawEvent,
        event::{
            AccessibleActionEvent, EnabledChangeEvent, Event, FocusInEvent, FocusOutEvent,
            InputMethodEvent, KeyboardInputEvent, LayoutEvent, MouseEnterEvent, MouseInputEvent,
            MouseLeaveEvent, MouseMoveEvent, MouseScrollEvent, ScrollToRectEvent, StyleChangeEvent,
            WindowFocusChangeEvent,
        },
        layout::{
            grid::{self, grid_layout},
            SizeHints,
        },
    },
    anyhow::Result,
    downcast_rs::{impl_downcast, Downcast},
};

pub trait Widget: Downcast {
    fn type_name() -> &'static str
    where
        Self: Sized;

    fn is_window_root_type() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn new(common: WidgetCommonTyped<Self>) -> Self
    where
        Self: Sized;

    fn common(&self) -> &WidgetCommon;
    fn common_mut(&mut self) -> &mut WidgetCommon;
    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_mouse_input(&mut self, event: MouseInputEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_scroll(&mut self, event: MouseScrollEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_enter(&mut self, event: MouseEnterEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_move(&mut self, event: MouseMoveEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_mouse_leave(&mut self, event: MouseLeaveEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_input_method(&mut self, event: InputMethodEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_layout(&mut self, event: LayoutEvent) -> Result<()> {
        grid_layout(self, &event.changed_size_hints);
        Ok(())
    }
    fn handle_scroll_to_rect(&mut self, request: ScrollToRectEvent) -> Result<bool> {
        let _ = request;
        Ok(false)
    }
    fn handle_focus_in(&mut self, event: FocusInEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_focus_out(&mut self, event: FocusOutEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_window_focus_change(&mut self, event: WindowFocusChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_accessible_action(&mut self, event: AccessibleActionEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_style_change(&mut self, event: StyleChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_enabled_change(&mut self, event: EnabledChangeEvent) -> Result<()> {
        let _ = event;
        Ok(())
    }
    fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::MouseInput(e) => self.handle_mouse_input(e),
            Event::MouseScroll(e) => self.handle_mouse_scroll(e),
            Event::MouseEnter(e) => self.handle_mouse_enter(e),
            Event::MouseMove(e) => self.handle_mouse_move(e),
            Event::MouseLeave(e) => self.handle_mouse_leave(e).map(|()| true),
            Event::KeyboardInput(e) => self.handle_keyboard_input(e),
            Event::InputMethod(e) => self.handle_input_method(e),
            Event::Draw(e) => self.handle_draw(e).map(|()| true),
            Event::Layout(e) => self.handle_layout(e).map(|()| true),
            Event::FocusIn(e) => self.handle_focus_in(e).map(|()| true),
            Event::FocusOut(e) => self.handle_focus_out(e).map(|()| true),
            Event::WindowFocusChange(e) => self.handle_window_focus_change(e).map(|()| true),
            Event::Accessible(e) => self.handle_accessible_action(e).map(|()| true),
            Event::ScrollToRect(e) => self.handle_scroll_to_rect(e),
            Event::StyleChange(e) => self.handle_style_change(e).map(|()| true),
            Event::EnabledChange(e) => self.handle_enabled_change(e).map(|()| true),
        }
    }

    fn handle_declare_children_request(&mut self) -> Result<()> {
        self.common_mut().has_declare_children_override = false;
        Ok(())
    }
    fn handle_size_hint_x_request(&mut self) -> Result<SizeHints> {
        let options = self.common().grid_options();
        Ok(grid::size_hint_x(&mut self.common_mut().children, &options))
    }
    fn handle_size_hint_y_request(&mut self, size_x: i32) -> Result<SizeHints> {
        let options = self.common().grid_options();
        Ok(grid::size_hint_y(
            &mut self.common_mut().children,
            &options,
            size_x,
        ))
    }
    fn handle_accessible_node_request(&mut self) -> Result<Option<accesskit::NodeBuilder>> {
        Ok(None)
    }
}
impl_downcast!(Widget);
