use std::{thread::sleep, time::Duration};

use widgem::{
    shortcut::{KeyCombinations, Shortcut, ShortcutScope},
    widgets::{Column, Label, ScrollArea},
    Widget, WidgetExt, Window,
};
use widgem_tester::{context::Context, Key};

#[widgem_tester::test]
pub fn scroll_area(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        let window = root
            .set_main_content(Window::init(module_path!().into()))?
            .set_padding_enabled(false);
        let on_r = window.callback(move |window, _| {
            let is_resizable = window.is_resizable();
            window.set_resizable(!is_resizable);
            Ok(())
        });
        window.base_mut().add_shortcut(Shortcut::new(
            KeyCombinations::from_str_portable("R").unwrap(),
            ShortcutScope::Application,
            on_r,
        ));
        let mut content_items = window
            .set_main_content(ScrollArea::init())?
            .set_size_x_fixed(Some(false))
            .set_content(Column::init())?
            .contents_mut();

        for i in 0..20 {
            content_items.set_next_item(Label::init(format!("text item {i}")))?;
        }
        Ok(())
    })?;
    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("scroll area")?;
    window.resize(150, 150)?;
    window.snapshot("resized 150x150")?;

    // Avoid conflict with macos window resizing
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
    }
    // scroll down button
    window.mouse_move(146, 146)?;
    ctx.ui().mouse_left_click()?;
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
    }
    window.snapshot("step down")?;
    ctx.ui().mouse_scroll_down()?;
    window.snapshot("scroll down")?;
    window.resize(110, 150)?;
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
    }
    window.mouse_move(10, 10)?;
    window.mouse_move(-100, -100)?;
    window.snapshot("resized 110x150")?;
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
        sleep(Duration::from_secs(1));
    }
    window.resize(100, 150)?;
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
    }
    window.mouse_move(-100, -100)?;
    window.snapshot("resized 100x150")?;
    if cfg!(target_os = "macos") {
        ctx.ui().key(Key::Unicode('r'))?;
        sleep(Duration::from_secs(1));
    }
    // horizontal scroll slider
    window.mouse_move(30, 145)?;
    ctx.ui().mouse_left_press()?;
    window.mouse_move(100, 145)?;
    ctx.ui().mouse_left_release()?;
    window.snapshot("scroll right")?;
    window.resize(160, 150)?;
    window.snapshot("resized 160x150")?;
    window.resize(160, 500)?;
    window.snapshot("resized 160x500")?;
    window.resize(160, 600)?;
    window.snapshot("resized 160x600")?;

    window.close()?;
    Ok(())
}

#[widgem_tester::test]
pub fn layout(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(|root| {
        let mut window_contents = root
            .set_main_content(Window::init(module_path!().into()))?
            .contents_mut();
        window_contents.set_next_item(Label::init("before".into()))?;
        let mut content = window_contents
            .set_next_item(ScrollArea::init())?
            .set_content(Column::init())?
            .set_padding_enabled(false)
            .contents_mut();

        for i in 0..20 {
            content.set_next_item(Label::init(format!("text item {i}")))?;
        }
        window_contents.set_next_item(Label::init("after".into()))?;
        Ok(())
    })?;

    let window = ctx.wait_for_window_by_pid()?;
    window.snapshot("initial")?;
    window.resize(300, 800)?;
    window.snapshot("resized to 300x800")?;
    window.resize(300, 350)?;
    window.snapshot("resized to 300x350")?;

    window.close()?;
    Ok(())
}
