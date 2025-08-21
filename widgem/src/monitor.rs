use {crate::types::Rect, winit::monitor::MonitorHandle};

pub trait MonitorExt {
    /// Returns global physical coordinates of the area of the monitor that is not allocated to toolbars.
    fn work_area(&self) -> Option<Rect>;
}

impl MonitorExt for MonitorHandle {
    #[cfg(target_os = "macos")]
    fn work_area(&self) -> Option<Rect> {
        use {
            core_graphics::display::CGDisplay, objc2::rc::Retained, objc2_app_kit::NSScreen,
            tracing::trace, winit::platform::macos::MonitorHandleExtMacOS,
        };

        trace!(
            "winit size {:?}, pos {:?}, scale {:?}",
            self.size(),
            self.position(),
            self.scale_factor()
        );

        let scale = self.scale_factor();
        // It is intentional that we use `CGMainDisplayID` (as opposed to
        // `NSScreen::mainScreen`), because that's what the screen coordinates
        // are relative to, no matter which display the window is currently on.
        let main_screen_height = CGDisplay::main().bounds().size.height;

        let visible_frame = unsafe {
            trace!("main display bounds {:?}", CGDisplay::main().bounds());

            let screen = Retained::retain(self.ns_screen()? as *mut NSScreen)?;

            trace!("frame {:?}", screen.frame());
            // trace!(
            //     "frame backing {:?}",
            //     screen.convertRectToBacking(screen.frame())
            // );

            screen.visibleFrame()
        };
        trace!("visible frame {:?}", visible_frame);
        let origin_y = main_screen_height - visible_frame.size.height - visible_frame.origin.y;
        Some(Rect::from_xywh(
            ((visible_frame.origin.x * scale).round() as i32).into(),
            ((origin_y * scale).round() as i32).into(),
            ((visible_frame.size.width * scale).round() as i32).into(),
            ((visible_frame.size.height * scale).round() as i32).into(),
        ))
    }
}
