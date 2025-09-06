use itertools::Itertools;
use widgem::{MonitorExt, Widget};

fn main() {
    widgem::run(|root| {
        let rects = root
            .base()
            .app()
            .available_monitors()
            .map(|monitor| monitor.work_area())
            .map(|rect| {
                (
                    rect.left().to_i32(),
                    rect.top().to_i32(),
                    rect.size_x().to_i32(),
                    rect.size_y().to_i32(),
                )
            })
            .collect_vec();
        println!("{:?}", rects);
        root.base().app().exit();
        Ok(())
    })
    .unwrap();
}
