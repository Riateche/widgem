pub use super::scroll_bar::RootWidget;

use crate::context::Context;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(100, 20)?;
    ctx.snapshot(&mut window, "highlighted pager")?;

    ctx.connection.mouse_scroll_down()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection.mouse_scroll_down()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection.mouse_scroll_up()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection.mouse_scroll_up()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection.mouse_scroll_right()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection.mouse_scroll_right()?;
    ctx.snapshot(&mut window, "scrolled down")?;

    ctx.connection.mouse_scroll_left()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    ctx.connection.mouse_scroll_left()?;
    ctx.snapshot(&mut window, "scrolled up")?;

    window.close()?;
    Ok(())
}
