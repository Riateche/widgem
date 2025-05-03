use {
    super::{WidgetCommon, WidgetCommonTyped},
    crate::{
        draw::DrawEvent,
        event::{
            AccessibleActionEvent, DeclareChildrenEvent, EnabledChangeEvent, Event, FocusInEvent,
            FocusOutEvent, ImeEvent, KeyboardInputEvent, LayoutEvent, MouseEnterEvent,
            MouseInputEvent, MouseLeaveEvent, MouseMoveEvent, MouseScrollEvent, ScrollToRectEvent,
            StyleChangeEvent, WindowFocusChangeEvent,
        },
        layout::{
            grid::{self},
            SizeHintMode,
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
    fn handle_declare_children(&mut self, event: DeclareChildrenEvent) -> Result<()> {
        let _ = event;
        self.common_mut().has_declare_children_override = false;
        self.common_mut().delete_undeclared_children = false;
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
    fn handle_ime(&mut self, event: ImeEvent) -> Result<bool> {
        let _ = event;
        Ok(false)
    }
    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let options = self.common().grid_options();
        let Some(size) = self.common().size() else {
            return Ok(());
        };
        let rects = grid::layout(&mut self.common_mut().children, &options, size)?;
        self.common_mut().set_child_rects(&rects)
    }
    fn handle_scroll_to_rect(&mut self, event: ScrollToRectEvent) -> Result<bool> {
        let _ = event;
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
            Event::Ime(e) => self.handle_ime(e),
            Event::Draw(e) => self.handle_draw(e).map(|()| true),
            Event::DeclareChildren(e) => self.handle_declare_children(e).map(|()| true),
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
    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let options = self.common().grid_options();
        grid::size_hint_x(&mut self.common_mut().children, &options, mode)
    }
    fn recalculate_size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let options = self.common().grid_options();
        grid::size_hint_y(&mut self.common_mut().children, &options, size_x, mode)
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        let options = self.common().grid_options();
        grid::size_x_fixed(&mut self.common_mut().children, &options)
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        let options = self.common().grid_options();
        grid::size_y_fixed(&mut self.common_mut().children, &options)
    }

    // TODO: result?
    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        None
    }
}
impl_downcast!(Widget);
