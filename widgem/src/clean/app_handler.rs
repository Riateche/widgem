use {
    crate::{
        clean::{App, BoxWidget},
        event_loop::UserEvent,
    },
    winit::event_loop::ActiveEventLoop,
};

pub(crate) struct AppHandler {
    app: App,
    root: BoxWidget,
}

impl AppHandler {
    pub(crate) fn new(app: App, root: BoxWidget) -> Self {
        Self { app, root }
    }

    fn render(&mut self, _event_loop: &ActiveEventLoop) {
        todo!()
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
