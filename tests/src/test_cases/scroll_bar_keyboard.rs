pub use super::scroll_bar::RootWidget;

use crate::context::Context;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;
    ctx.connection.key("f")?;
    ctx.snapshot(&mut window, "focused")?;
    ctx.connection.key("1")?;
    ctx.snapshot(&mut window, "increased range")?;

    ctx.connection.key("Down")?;
    ctx.snapshot(&mut window, "step down")?;
    ctx.connection.key("Down")?;
    ctx.snapshot(&mut window, "step down")?;

    ctx.connection.key("Page_Down")?;
    ctx.snapshot(&mut window, "page down")?;
    ctx.connection.key("Page_Down")?;
    ctx.snapshot(&mut window, "page down")?;

    ctx.connection.key("Up")?;
    ctx.snapshot(&mut window, "step up")?;
    ctx.connection.key("Up")?;
    ctx.snapshot(&mut window, "step up")?;

    ctx.connection.key("Page_Up")?;
    ctx.snapshot(&mut window, "page up")?;
    ctx.connection.key("Page_Up")?;
    ctx.snapshot(&mut window, "page up")?;

    ctx.connection.key("End")?;
    ctx.snapshot(&mut window, "end")?;
    ctx.connection.key("Home")?;
    ctx.snapshot(&mut window, "home")?;

    window.close()?;
    Ok(())
}
