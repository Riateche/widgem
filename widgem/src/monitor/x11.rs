use std::cmp::{max, min};

use anyhow::Context as _;
use tracing::warn;
use winit::monitor::MonitorHandle;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;

use crate::types::{Point, PpxSuffix, Rect, Size};

pub fn work_area(monitor: &MonitorHandle) -> anyhow::Result<Rect> {
    let monitor_rect = Rect::from_pos_size(monitor.position().into(), monitor.size().into());

    let (connection, _screen_num) = x11rb::connect(None)?;
    let roots = &connection.setup().roots;
    println!("num screens {:?}", roots.len());
    if roots.len() > 1 {
        warn!("found more than one X11 screen");
    }
    let root_window = roots.first().context("no X11 roots found")?.root;
    // let net_workarea = connection
    //     .intern_atom(false, b"_NET_WORKAREA")
    //     .context("failed to intern atom")?
    //     .reply()
    //     .context("Failed receive interned atom")?
    //     .atom;
    let cardinal_type = connection
        .intern_atom(false, b"CARDINAL")
        .context("failed to intern atom")?
        .reply()
        .context("Failed receive interned atom")?
        .atom;

    let net_client_list = connection
        .intern_atom(false, b"_NET_CLIENT_LIST")
        .context("failed to intern atom")?
        .reply()
        .context("Failed receive interned atom")?
        .atom;
    let net_wm_strut_partial = connection
        .intern_atom(false, b"_NET_WM_STRUT_PARTIAL")
        .context("failed to intern atom")?
        .reply()
        .context("Failed receive interned atom")?
        .atom;
    let window_type = connection
        .intern_atom(false, b"WINDOW")
        .context("failed to intern atom")?
        .reply()
        .context("Failed receive interned atom")?
        .atom;

    // let value = connection
    //     .get_property(false, root_window, net_workarea, cardinal, 0, u32::MAX)?
    //     .reply()?;
    // let value = value.value32().context("property value type mismatch")?;
    // println!("work area data: {:?}", value.collect::<Vec<_>>());

    let client_list = connection
        .get_property(
            false,
            root_window,
            net_client_list,
            window_type,
            0,
            u32::MAX,
        )?
        .reply()?;
    let client_list = client_list
        .value32()
        .context("property value type mismatch")?;

    let root_window_geometry = connection.get_geometry(root_window)?.reply()?;
    println!(
        "root geometry: {}, {}, {}, {}",
        root_window_geometry.x,
        root_window_geometry.y,
        root_window_geometry.width,
        root_window_geometry.height,
    );
    let root_rect = Rect::from_pos_size(
        Point::new(
            i32::from(root_window_geometry.x).ppx(),
            i32::from(root_window_geometry.y).ppx(),
        ),
        Size::new(
            i32::from(root_window_geometry.width).ppx(),
            i32::from(root_window_geometry.height).ppx(),
        ),
    );

    let mut work_area = monitor_rect;
    for window in client_list {
        println!("net client: {:?}", window);

        let strut_partial = connection
            .get_property(
                false,
                window,
                net_wm_strut_partial,
                cardinal_type,
                0,
                u32::MAX,
            )?
            .reply()?;

        if strut_partial.length == 0 {
            continue;
        }
        let strut_partial = strut_partial
            .value32()
            .context("property value type mismatch")?
            .map(|v| {
                i32::try_from(v)
                    .context("strut_partial value overflow")
                    .map(|v| v.ppx())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        if strut_partial.len() != 12 {
            warn!(
                "invalid length of _NET_WM_STRUT_PARTIAL: expected 12, got {}",
                strut_partial.len()
            );
        }
        println!("strut_partial = {strut_partial:?}");

        let window_geometry = connection.get_geometry(window)?.reply()?;
        let window_absolute_top_left = connection
            .translate_coordinates(window, root_window, window_geometry.x, window_geometry.y)?
            .reply()?;
        println!(
            "geometry: {}, {}; translated: {}, {}, {}, {}",
            window_geometry.x,
            window_geometry.y,
            window_absolute_top_left.dst_x,
            window_absolute_top_left.dst_y,
            window_geometry.width,
            window_geometry.height,
        );

        let window_rect = Rect::from_pos_size(
            Point::new(
                i32::from(window_absolute_top_left.dst_x).ppx(),
                i32::from(window_absolute_top_left.dst_y).ppx(),
            ),
            Size::new(
                i32::from(window_geometry.width).ppx(),
                i32::from(window_geometry.height).ppx(),
            ),
        );
        if !window_rect.intersects(monitor_rect) {
            println!("not in monitor, skipping");
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
            println!("detected left panel");
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
            println!("detected right panel");
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
            println!("detected top panel");
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
            println!("detected bottom panel");
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
