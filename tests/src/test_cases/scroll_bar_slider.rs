pub use super::scroll_bar::RootWidget;

use crate::context::Context;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    // Workaround for winit issue:
    // https://github.com/rust-windowing/winit/issues/2841
    window.minimize()?;
    window.activate()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.mouse_move(40, 20)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(50, 20)?;
    ctx.snapshot(&mut window, "moved slider by 10 px")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider")?;
    window.mouse_move(15, 15)?;
    ctx.snapshot(&mut window, "highlighted left arrow")?;
    ctx.connection.mouse_click(1)?;
    ctx.snapshot(&mut window, "step left by 5")?;

    window.mouse_move(60, 18)?;
    ctx.snapshot(&mut window, "highlighted slider")?;
    ctx.connection.mouse_down(1)?;
    ctx.snapshot(&mut window, "grabbed slider")?;
    window.mouse_move(60, 88)?;
    ctx.snapshot(&mut window, "dragged down and outside - no effect")?;
    window.mouse_move(50, 88)?;
    ctx.snapshot(&mut window, "dragged left")?;
    ctx.connection.mouse_up(1)?;
    ctx.snapshot(&mut window, "released slider - no highlight")?;

    window.close()?;
    Ok(())
}
