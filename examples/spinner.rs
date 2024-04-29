use std::{thread::sleep, time::Duration};

use demand::{Spinner, SpinnerStyle, Theme};

fn main() {
    let custom_style = SpinnerStyle {
        frames: vec![
            "   ", "-  ", "-- ", "---", " --", "  -", "   ", "  -", " --", "---", "-- ", "-  ",
        ],
        fps: Duration::from_millis(1000 / 10),
    };
    let dots = SpinnerStyle::dots();
    let line = SpinnerStyle::line();

    let charm = Theme::charm();
    let catppuccin = Theme::catppuccin();
    Spinner::new("Loading Data...")
        .style(&custom_style)
        .run(|s| {
            sleep(Duration::from_secs(2));
            let mut toggle = false;
            for name in ["Files", "Data", "Your Soul"] {
                let _ = s.title(format!("Loading {name}..."));
                let _ = s.style(if toggle { &dots } else { &line });
                let _ = s.theme(if toggle { &catppuccin } else { &charm });
                toggle = !toggle;
                sleep(Duration::from_secs(2));
            }
        })
        .expect("error running spinner");
    println!("Data loaded.");
}
