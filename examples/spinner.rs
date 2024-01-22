use std::{thread::sleep, time::Duration};

use demand::{Spinner, SpinnerStyle};

fn main() {
    Spinner::new("Loading Data...")
        .style(SpinnerStyle::line())
        .run(|| {
            sleep(Duration::from_secs(2));
        })
        .expect("error running spinner");
    println!("Done!");
}
