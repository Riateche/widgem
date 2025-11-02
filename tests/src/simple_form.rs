use {
    widgem::{
        layout::Layout,
        types::Axis,
        widgets::{Button, Label, RootWidget, ScrollBar, TextArea, TextInput},
        WidgetExt, Window,
    },
    widgem_tester::Context,
};

fn init_form(root: &mut RootWidget) -> anyhow::Result<()> {
    let mut contents = root
        .set_main_content(Window::init(module_path!().into()))?
        .set_layout(Layout::ExplicitGrid)
        .contents_mut();

    let mut current_cell_y = 0;
    let user_name_label_id = contents
        .set_next_item(Label::init("User name:".into()))?
        .set_grid_cell(0, current_cell_y)
        .id();
    contents
        .set_next_item(TextInput::init())?
        .set_grid_cell(1, current_cell_y)
        .set_labelled_by(user_name_label_id.raw());

    current_cell_y += 1;
    let scroll_bar_label_id = contents
        .set_next_item(Label::init("Scroll bar label:".into()))?
        .set_grid_cell(0, current_cell_y)
        .id();
    contents
        .set_next_item(ScrollBar::init(Axis::X))?
        .set_grid_cell(1, current_cell_y)
        .set_labelled_by(scroll_bar_label_id.raw());

    current_cell_y += 1;
    contents
        .set_next_item(Label::init("Multiline label\nSecond line".into()))?
        .set_grid_cell(1, current_cell_y);

    current_cell_y += 1;
    let text_area_label_id = contents
        .set_next_item(Label::init("Long text:".into()))?
        .set_grid_cell(0, current_cell_y)
        .id();
    contents
        .set_next_item(TextArea::init())?
        .set_grid_cell(1, current_cell_y)
        .set_labelled_by(text_area_label_id.raw())
        //.set_expand_to_fit_content(true)
        //.set_wrap(Wrap::Word)
        ;

    current_cell_y += 1;
    let submit = contents
        .set_next_item(Button::init("Submit".into()))?
        .set_grid_cell(1, current_cell_y);
    let on_submit_triggered = submit.callback(|this, ()| {
        this.set_text("OK!");
        Ok(())
    });
    submit.on_triggered(on_submit_triggered);

    Ok(())
}

#[widgem_tester::test]
fn main(ctx: &mut Context) -> anyhow::Result<()> {
    ctx.run(init_form)?;

    let window = ctx.wait_for_window_by_pid()?;
    ctx.set_blinking_expected(true);
    window.snapshot("form")?;
    window.close()?;
    Ok(())
}

#[cfg(target_os = "windows")]
mod windows {
    use {
        super::*,
        anyhow::{ensure, Context as _},
        cadd::prelude::IntoType,
        std::{thread::sleep, time::Duration},
        uiautomation::{
            controls::ControlType,
            patterns::{UIValuePattern, UIWindowPattern},
            types::{TreeScope, UIProperty},
            UIAutomation, UIElement, UITreeWalker,
        },
    };

    #[widgem_tester::test]
    fn windows_accessibility(ctx: &mut Context) -> anyhow::Result<()> {
        ctx.run(init_form)?;

        let window = ctx.wait_for_window_by_pid()?;
        ctx.set_blinking_expected(true);
        window.snapshot("form")?;
        sleep(Duration::from_secs(1));

        let automation = UIAutomation::new()?;
        let root = automation.get_root_element()?;
        let pid_condition = automation.create_property_condition(
            UIProperty::ProcessId,
            (ctx.test_subject_pid()? as i32).into(),
            None,
        )?;
        let uia_window = root
            .find_first(TreeScope::Children, &pid_condition)
            .context("failed to find window by pid")?;

        ensure!(uia_window.get_name()? == "widgem_tests::simple_form");
        let window_rect = uia_window.get_bounding_rectangle()?;
        ensure!(window_rect.get_width() == 270);
        ensure!(window_rect.get_height() == 191);
        let walker = automation.get_control_view_walker()?;
        let title_bar = walker.get_first_child(&uia_window)?;
        ensure!(title_bar.get_control_type()? == ControlType::TitleBar);

        let user_name = walker.get_next_sibling(&title_bar)?;
        ensure!(user_name.get_control_type()? == ControlType::Edit);
        ensure!(user_name.get_name()? == "User name:");
        let user_name_value = user_name.get_pattern::<UIValuePattern>()?;
        ensure!(user_name_value.get_value()? == "");
        ensure!(
            user_name
                .get_property_value(UIProperty::HasKeyboardFocus)?
                .try_into_type::<bool>()?
                == true
        );
        let user_name_rect = user_name.get_bounding_rectangle()?;
        ensure!(user_name_rect.get_width() == 131);
        ensure!(user_name_rect.get_height() == 27);

        user_name_value.set_value("Hello")?;
        window.snapshot("input hello")?;
        ensure!(user_name_value.get_value()? == "Hello");

        let multiline_label = walker.get_next_sibling(&user_name)?;
        ensure!(multiline_label.get_control_type()? == ControlType::Text);
        ensure!(multiline_label.get_name()? == "Multiline label\nSecond line");
        let multiline_label_rect = multiline_label.get_bounding_rectangle()?;
        ensure!(multiline_label_rect.get_width() == 88);
        ensure!(multiline_label_rect.get_height() == 37);
        ensure!(multiline_label_rect.get_left() - user_name_rect.get_left() == 0);
        ensure!(multiline_label_rect.get_top() - user_name_rect.get_top() == 58);

        let submit_button = walker.get_next_sibling(&multiline_label)?;
        ensure!(submit_button.get_control_type()? == ControlType::Button);
        ensure!(submit_button.get_name()? == "Submit");
        let submit_button_rect = submit_button.get_bounding_rectangle()?;
        ensure!(submit_button_rect.get_width() == 55);
        ensure!(submit_button_rect.get_height() == 29);
        ensure!(submit_button_rect.get_left() - user_name_rect.get_left() == 0);
        ensure!(submit_button_rect.get_top() - user_name_rect.get_top() == 101);

        submit_button.set_focus()?;
        ctx.set_blinking_expected(false);
        window.snapshot("focused submit button")?;

        submit_button.click()?;
        window.snapshot("clicked submit button")?;

        ensure!(walker.get_next_sibling(&submit_button).is_err());

        uia_window.get_pattern::<UIWindowPattern>()?.close()?;

        Ok(())
    }

    #[allow(dead_code)]
    fn print_element(
        walker: &UITreeWalker,
        element: &UIElement,
        level: usize,
    ) -> anyhow::Result<()> {
        for _ in 0..level {
            print!(" ")
        }
        println!(
            "{} - {} [pid={}]",
            element.get_classname()?,
            element.get_name()?,
            element.get_process_id()?,
        );

        if level < 1 {
            if let Ok(child) = walker.get_first_child(element) {
                print_element(walker, &child, level + 1)?;

                let mut next = child;
                while let Ok(sibling) = walker.get_next_sibling(&next) {
                    print_element(walker, &sibling, level + 1)?;

                    next = sibling;
                }
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use {
        super::*,
        anyhow::{ensure, Context as _},
        objc2_core_foundation::{CFBoolean, CFString},
        uitest::{AXUIElementExt, WindowExt},
    };

    #[widgem_tester::test]
    fn macos_accessibility(ctx: &mut Context) -> anyhow::Result<()> {
        ctx.run(init_form)?;

        let window = ctx.wait_for_window_by_pid()?;
        ctx.set_blinking_expected(true);
        window.snapshot("form")?;
        let ax_window = window.ui_element();
        let _ = ax_window.children()?;
        let children = ax_window.children()?;
        let root_group = children.first().context("root group not found")?.clone();
        ensure!(root_group.role()? == "AXGroup");
        let mut root_children = root_group.children()?.into_iter();

        let user_name = root_children.next().context("not enough root children")?;
        ensure!(user_name.role()? == "AXTextArea");

        ensure!(
            user_name
                .attribute("AXValue")?
                .context("missing AXValue attribute")?
                .downcast_ref::<CFString>()
                .context("AXValue is not CFString")?
                .to_string()
                == ""
        );

        ensure!(user_name.is_attribute_settable_safe("AXValue")?);
        user_name.set_attribute("AXValue", &CFString::from_static_str("Hello"))?;
        window.snapshot("input hello")?;
        ensure!(
            user_name
                .attribute("AXValue")?
                .context("missing AXValue attribute")?
                .downcast_ref::<CFString>()
                .context("AXValue is not CFString")?
                .to_string()
                == "Hello"
        );

        user_name.set_attribute("AXValue", &CFString::from_static_str("Hello мир"))?;
        window.snapshot("input hello world ru")?;
        ensure!(
            user_name
                .attribute("AXValue")?
                .context("missing AXValue attribute")?
                .downcast_ref::<CFString>()
                .context("AXRole is not CFString")?
                .to_string()
                == "Hello мир"
        );

        let label = root_children.next().context("not enough root children")?;
        ensure!(label.role()? == "AXStaticText");
        ensure!(
            label
                .attribute("AXValue")?
                .context("missing AXValue attribute")?
                .downcast_ref::<CFString>()
                .context("AXRole is not CFString")?
                .to_string()
                == "Multiline label\nSecond line"
        );

        let submit = root_children.next().context("not enough root children")?;
        ensure!(submit.role()? == "AXButton");
        ensure!(
            submit
                .attribute("AXTitle")?
                .context("missing AXTitle attribute")?
                .downcast_ref::<CFString>()
                .context("AXTitle is not CFString")?
                .to_string()
                == "Submit"
        );
        ensure!(submit.action_names()?.iter().any(|s| s == "AXPress"));
        unsafe { submit.perform_action(&CFString::from_static_str("AXPress")) };
        window.snapshot("pressed submit")?;

        submit.set_attribute("AXFocused", CFBoolean::new(true))?;
        ctx.set_blinking_expected(false);
        window.snapshot("focused submit")?;

        window.close()?;
        Ok(())
    }
}
