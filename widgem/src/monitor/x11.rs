use {
    crate::{
        event_loop::with_active_event_loop,
        types::{PpxSuffix, Rect},
    },
    anyhow::{bail, Context as _},
    itertools::Itertools,
    std::cmp::{max, min},
    tracing::{trace, warn},
    winit::{monitor::MonitorHandle, platform::x11::MonitorHandleExtX11},
    x11rb::{
        connection::Connection,
        protocol::xproto::{Atom, ConnectionExt as _, Window},
        rust_connection::RustConnection,
    },
};

trait ConnectionExt: Connection {
    fn atom(&self, name: &str) -> anyhow::Result<Atom> {
        let value = self
            .intern_atom(false, name.as_bytes())
            .context("failed to intern atom")?
            .reply()
            .context("failed to receive interned atom")?;
        Ok(value.atom)
    }
}

impl<T: Connection> ConnectionExt for T {}

struct Atoms {
    cardinal_type: Atom,
    window_type: Atom,
    net_workarea: Atom,
    net_current_desktop: Atom,
    net_wm_strut_partial: Atom,
    net_client_list: Atom,
}

struct Client {
    connection: RustConnection,
    root_window: Window,
    atoms: Atoms,
}

impl Client {
    fn init() -> anyhow::Result<Self> {
        let (connection, _screen_num) = x11rb::connect(None)?;
        let roots = &connection.setup().roots;
        if roots.len() > 1 {
            warn!("found more than one X11 screen");
        }
        let root_window = roots.first().context("no X11 roots found")?.root;
        Ok(Self {
            atoms: Atoms {
                cardinal_type: connection.atom("CARDINAL")?,
                window_type: connection.atom("WINDOW")?,
                net_workarea: connection.atom("_NET_WORKAREA")?,
                net_current_desktop: connection.atom("_NET_CURRENT_DESKTOP")?,
                net_wm_strut_partial: connection.atom("_NET_WM_STRUT_PARTIAL")?,
                net_client_list: connection.atom("_NET_CLIENT_LIST")?,
            },
            connection,
            root_window,
        })
    }

    fn net_work_area(&self) -> anyhow::Result<Rect> {
        let net_workarea = self
            .connection
            .get_property(
                false,
                self.root_window,
                self.atoms.net_workarea,
                self.atoms.cardinal_type,
                0,
                u32::MAX,
            )?
            .reply()?;
        let net_workarea = net_workarea
            .value32()
            .context("_NET_WORKAREA value type mismatch")?
            .map(|v| {
                i32::try_from(v)
                    .context("_NET_WORKAREA value overflow")
                    .map(|v| v.ppx())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        trace!("_NET_WORKAREA = {net_workarea:?}");
        if net_workarea.len() < 4 {
            bail!("_NET_WORKAREA value too short");
        }
        Ok(Rect::from_xywh(
            net_workarea[0],
            net_workarea[1],
            net_workarea[2],
            net_workarea[3],
        ))
    }

    fn gtk_work_area(&self, monitor: &MonitorHandle) -> anyhow::Result<Option<Rect>> {
        let net_current_desktop = self
            .connection
            .get_property(
                false,
                self.root_window,
                self.atoms.net_current_desktop,
                self.atoms.cardinal_type,
                0,
                u32::MAX,
            )?
            .reply()?;

        if net_current_desktop.length == 0 {
            return Ok(None);
        }
        let current_desktop = net_current_desktop
            .value32()
            .context("_NET_CURRENT_DESKTOP type mismatch")?
            .next()
            .context("missing value for _NET_CURRENT_DESKTOP")?;
        trace!("_NET_CURRENT_DESKTOP = {current_desktop:?}");

        let gtk_workareas = self
            .connection
            .get_property(
                false,
                self.root_window,
                self.connection
                    .atom(&format!("_GTK_WORKAREAS_D{current_desktop}"))?,
                self.atoms.cardinal_type,
                0,
                u32::MAX,
            )?
            .reply()?;
        if gtk_workareas.length == 0 {
            return Ok(None);
        }
        let gtk_workareas = gtk_workareas
            .value32()
            .context("_GTK_WORKAREAS_D value type mismatch")?
            .map(|v| {
                i32::try_from(v)
                    .context("_GTK_WORKAREAS_D value overflow")
                    .map(|v| v.ppx())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        trace!("_GTK_WORKAREAS_D = {gtk_workareas:?}");

        if gtk_workareas.len() % 4 != 0 {
            bail!(
                "invalid length for _GTK_WORKAREAS_D: {}",
                gtk_workareas.len()
            );
        }

        let monitor_rect = Rect::from_pos_size(monitor.position().into(), monitor.size().into());
        trace!("monitor_rect = {monitor_rect:?}");

        let matching_rects = gtk_workareas
            .chunks(4)
            .map(|chunk| Rect::from_xywh(chunk[0], chunk[1], chunk[2], chunk[3]))
            .filter(|rect| rect.intersects(monitor_rect))
            .collect_vec();
        if matching_rects.is_empty() {
            bail!("no rect from _GTK_WORKAREAS_D matches monitor rect");
        }
        if matching_rects.len() > 1 {
            warn!("multiple rects from _GTK_WORKAREAS_D match monitor rect");
        }
        Ok(Some(matching_rects[0]))
    }

    fn strut_based_work_area(&self, monitor: &MonitorHandle) -> anyhow::Result<Rect> {
        let client_list = self
            .connection
            .get_property(
                false,
                self.root_window,
                self.atoms.net_client_list,
                self.atoms.window_type,
                0,
                u32::MAX,
            )?
            .reply()?;
        let client_list = client_list
            .value32()
            .context("_NET_CLIENT_LIST value type mismatch")?;

        let root_window_geometry = self.connection.get_geometry(self.root_window)?.reply()?;
        trace!(
            "root geometry: {}, {}, {}, {}",
            root_window_geometry.x,
            root_window_geometry.y,
            root_window_geometry.width,
            root_window_geometry.height,
        );
        let root_rect = Rect::from_xywh(
            i32::from(root_window_geometry.x).ppx(),
            i32::from(root_window_geometry.y).ppx(),
            i32::from(root_window_geometry.width).ppx(),
            i32::from(root_window_geometry.height).ppx(),
        );

        let monitor_rect = Rect::from_pos_size(monitor.position().into(), monitor.size().into());
        trace!("monitor_rect = {monitor_rect:?}");

        let mut work_area = monitor_rect;
        for window in client_list {
            let strut_partial = self
                .connection
                .get_property(
                    false,
                    window,
                    self.atoms.net_wm_strut_partial,
                    self.atoms.cardinal_type,
                    0,
                    u32::MAX,
                )?
                .reply()?;

            if strut_partial.length == 0 {
                continue;
            }
            let strut_partial = strut_partial
                .value32()
                .context("_NET_WM_STRUT_PARTIAL value type mismatch")?
                .map(|v| {
                    i32::try_from(v)
                        .context("_NET_WM_STRUT_PARTIAL value overflow")
                        .map(|v| v.ppx())
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            if strut_partial.len() != 12 {
                warn!(
                    "invalid length of _NET_WM_STRUT_PARTIAL: expected 12, got {}",
                    strut_partial.len()
                );
            }
            trace!("_NET_WM_STRUT_PARTIAL({window}) = {strut_partial:?}");

            let window_geometry = self.connection.get_geometry(window)?.reply()?;
            let window_absolute_top_left = self
                .connection
                .translate_coordinates(
                    window,
                    self.root_window,
                    window_geometry.x,
                    window_geometry.y,
                )?
                .reply()?;
            trace!(
                "geometry({}): {}, {}; translated: {}, {}, {}, {}",
                window,
                window_geometry.x,
                window_geometry.y,
                window_absolute_top_left.dst_x,
                window_absolute_top_left.dst_y,
                window_geometry.width,
                window_geometry.height,
            );

            let window_rect = Rect::from_xywh(
                i32::from(window_absolute_top_left.dst_x).ppx(),
                i32::from(window_absolute_top_left.dst_y).ppx(),
                i32::from(window_geometry.width).ppx(),
                i32::from(window_geometry.height).ppx(),
            );
            if !window_rect.intersects(monitor_rect) {
                continue;
            }

            let left = strut_partial[0];
            let right = strut_partial[1];
            let top = strut_partial[2];
            let bottom = strut_partial[3];
            let left_start_y = strut_partial[4];
            let left_end_y = strut_partial[5];
            let right_start_y = strut_partial[6];
            let right_end_y = strut_partial[7];
            let top_start_x = strut_partial[8];
            let top_end_x = strut_partial[9];
            let bottom_start_x = strut_partial[10];
            let bottom_end_x = strut_partial[11];

            let rect_left = Rect::from_x1y1x2y2(
                root_rect.left(),
                left_start_y,
                root_rect.left() + left,
                left_end_y,
            );
            if monitor_rect.intersects(rect_left) {
                trace!("detected left panel ({rect_left:?})");
                work_area = Rect::from_x1y1x2y2(
                    max(work_area.left(), rect_left.right()),
                    work_area.top(),
                    work_area.right(),
                    work_area.bottom(),
                );
            }
            let rect_right = Rect::from_x1y1x2y2(
                root_rect.right() - right,
                right_start_y,
                root_rect.right(),
                right_end_y,
            );
            if monitor_rect.intersects(rect_right) {
                trace!("detected right panel ({rect_right:?})");
                work_area = Rect::from_x1y1x2y2(
                    work_area.left(),
                    work_area.top(),
                    min(work_area.right(), rect_right.left()),
                    work_area.bottom(),
                );
            }
            let rect_top = Rect::from_x1y1x2y2(
                top_start_x,
                root_rect.top(),
                top_end_x,
                root_rect.top() + top,
            );
            if monitor_rect.intersects(rect_top) {
                trace!("detected top panel ({rect_top:?})");
                work_area = Rect::from_x1y1x2y2(
                    work_area.left(),
                    max(work_area.top(), rect_top.bottom()),
                    work_area.right(),
                    work_area.bottom(),
                );
            }
            let rect_bottom = Rect::from_x1y1x2y2(
                bottom_start_x,
                root_rect.bottom() - bottom,
                bottom_end_x,
                root_rect.bottom(),
            );
            if monitor_rect.intersects(rect_bottom) {
                trace!("detected bottom panel ({rect_bottom:?})");
                work_area = Rect::from_x1y1x2y2(
                    work_area.left(),
                    work_area.top(),
                    work_area.right(),
                    min(work_area.bottom(), rect_bottom.top()),
                );
            }
        }

        Ok(work_area)
    }

    fn work_area(&self, monitor: &MonitorHandle) -> anyhow::Result<Rect> {
        let num_monitors =
            with_active_event_loop(|event_loop| event_loop.available_monitors().count());
        trace!("num_monitors = {num_monitors:?}");

        // _NET_WORKAREA is usually correct if there is only one monitor.
        // If there are multiple displays, _NET_WORKAREA typically only accounts for toolbars on the primary monitor.
        if num_monitors == 1 {
            match self.net_work_area() {
                Ok(rect) => return Ok(rect),
                Err(err) => {
                    warn!(?err, "net_workarea fetch failed");
                }
            }
        }

        // _GTK_WORKAREAS_D is perfect but it's only present in GTK-based environments (Gnome, Mate).
        match self.gtk_work_area(monitor) {
            Ok(Some(rect)) => return Ok(rect),
            Ok(None) => {}
            Err(err) => {
                warn!(?err, "gtk_workarea fetch failed");
            }
        }

        // Worst case, we calculate it from panel window properties.
        self.strut_based_work_area(monitor)
    }
}

thread_local! {
    static CLIENT: Option<Client> =
        Client::init().inspect_err(|err| warn!(?err, "failed to init X11 connection")).ok();

}

pub fn work_area(monitor: &MonitorHandle) -> Rect {
    CLIENT.with(|connection| {
        if let Some(connection) = connection {
            match connection.work_area(monitor) {
                Ok(rect) => {
                    trace!("work_area({}) = {:?}", monitor.native_id(), rect);
                    return rect;
                }
                Err(err) => {
                    tracing::warn!("failed to get monitor work area: {err:?}");
                }
            }
        }

        Rect::from_pos_size(monitor.position().into(), monitor.size().into())
    })
}
