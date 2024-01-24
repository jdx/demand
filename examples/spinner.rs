use std::{thread::sleep, time::Duration};

use demand::{Spinner, SpinnerStyle};

fn main() {
    let custom_style = SpinnerStyle {
        frames: vec![
            "   ", "-  ", "-- ", "---", " --", "  -", "   ", "  -", " --", "---", "-- ", "-  ",
        ],
        fps: Duration::from_millis(1000 / 10),
    };

    Spinner::new("Loading Data...")
        .style(&custom_style)
        .run(|| {
            sleep(Duration::from_secs(2));
        })
        .expect("error running spinner");
    println!("Data loaded.");
}
