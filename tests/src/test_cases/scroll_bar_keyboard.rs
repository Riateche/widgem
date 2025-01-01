pub use super::scroll_bar::RootWidget;

use crate::context::Context;

pub fn check(ctx: &mut Context) -> anyhow::Result<()> {
    let mut window = ctx.wait_for_window_by_pid()?;
    window.resize(160, 66)?;
    ctx.snapshot(&mut window, "scroll bar")?;

    window.close()?;
    Ok(())
}
