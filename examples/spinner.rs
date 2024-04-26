use std::{thread::sleep, time::Duration};

use demand::{Spinner, SpinnerAction, SpinnerStyle, Theme};

fn main() {
    let custom_style = SpinnerStyle {
        frames: vec![
            "   ", "-  ", "-- ", "---", " --", "  -", "   ", "  -", " --", "---", "-- ", "-  ",
        ],
        fps: Duration::from_millis(1000 / 10),
    };

    Spinner::new("Loading Data...")
        .style(custom_style)
        .run(|s| {
            sleep(Duration::from_secs(2));
            let mut toggle = false;
            for name in ["Files", "Data", "Your Soul"] {
                let _ = s.send(SpinnerAction::Title(format!("Loading {name}...")));
                let _ = s.send(SpinnerAction::Style(if toggle {
                    SpinnerStyle::dots()
                } else {
                    SpinnerStyle::line()
                }));
                let _ = s.send(SpinnerAction::Theme(
                    if toggle {
                        Theme::catppuccin()
                    } else {
                        Theme::charm()
                    }
                    .into(),
                ));
                toggle = !toggle;
                sleep(Duration::from_secs(2));
            }
        })
        .expect("error running spinner");
    println!("Data loaded.");
}
