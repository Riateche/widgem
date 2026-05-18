use {
    crate::{
        clean::{App, BoxWidget, WidgetContext, WidgetRenderContext, WidgetTreeItem},
        event_loop::UserEvent,
        style::{css::StyleSelector, defaults::default_style},
        widget_base::last_path_part,
    },
    winit::event_loop::ActiveEventLoop,
};

pub(crate) struct AppHandler {
    app: App,
    root: WidgetTreeItem,
}

fn render_widget(item: &mut WidgetTreeItem, app: &App, event_loop: &ActiveEventLoop) {
    let output = item.render();
}

impl AppHandler {
    pub(crate) fn new(app: App, root: BoxWidget) -> Self {
        let root_ctx = WidgetContext {
            is_in_window: false,
            is_focused: false,
            is_under_mouse: false,
            is_effectively_enabled: true,
            is_effectively_visible: true,
            // TODO: allow overriding style
            system_style: default_style(),
            custom_style: None,
            style_selector: StyleSelector::new(last_path_part((*root).type_name()).into()),
            effective_scale: None,
        };
        Self {
            app,
            root: WidgetTreeItem::new(root, root_ctx),
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        render_widget(&mut self.root, &self.app, event_loop);
    }
}

impl winit::application::ApplicationHandler<UserEvent> for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.render(event_loop);
        todo!()
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        _event: winit::event::WindowEvent,
    ) {
        todo!()
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        let _ = (event_loop, cause);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let _ = (event_loop, device_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}
