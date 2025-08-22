#[cfg(all(unix, not(target_os = "macos")))]
mod x11;

use {crate::types::Rect, winit::monitor::MonitorHandle};

pub trait MonitorExt {
    fn rect(&self) -> Rect;

    /// Returns global physical coordinates of the area of the monitor that is not allocated to
    /// system panels (taskbar on Windows, desktop panels on Linux, dock and menu bar on MacOS).
    fn work_area(&self) -> Rect;
}

impl MonitorExt for MonitorHandle {
    fn rect(&self) -> Rect {
        Rect::from_pos_size(self.position().into(), self.size().into())
    }

    #[cfg(target_os = "macos")]
    fn work_area(&self) -> Rect {
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
        Rect::from_xywh(
            ((visible_frame.origin.x * scale).round() as i32).into(),
            ((origin_y * scale).round() as i32).into(),
            ((visible_frame.size.width * scale).round() as i32).into(),
            ((visible_frame.size.height * scale).round() as i32).into(),
        )
    }

    #[cfg(target_os = "windows")]
    fn work_area(&self) -> Rect {
        use {
            crate::types::PpxSuffix,
            std::ffi::c_void,
            tracing::warn,
            windows_sys::Win32::{
                Foundation::{GetLastError, RECT},
                Graphics::Gdi::{GetMonitorInfoW, MONITORINFO},
            },
            winit::platform::windows::MonitorHandleExtWindows,
        };

        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };
        unsafe {
            if GetMonitorInfoW(self.hmonitor() as *mut c_void, &mut info) == 0 {
                warn!(
                    "failed to get monitor info (error code: {})",
                    GetLastError()
                );
                return self.rect();
            };
        }
        let work_rect = info.rcWork;
        Rect::from_x1y1x2y2(
            work_rect.left.ppx(),
            work_rect.top.ppx(),
            work_rect.right.ppx(),
            work_rect.bottom.ppx(),
        )
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn work_area(&self) -> Rect {
        // TODO: wayland
        x11::work_area(self)
    }
}
