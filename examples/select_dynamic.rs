use std::{thread, time::Duration};

use demand::{DemandOption, Select};

fn main() {
    let price = 100;
    let mut select = Select::new(format!("Coins inserted: 0, price: {price}"))
        .description("Coins are inserted every second")
        .option(DemandOption::new("Complete payment"))
        .option(DemandOption::new("Quit"));

    // Updates are made from another thread while the select is running.
    let handle = select.handle();
    thread::spawn(move || {
        for balance in (10..).step_by(10) {
            thread::sleep(Duration::from_secs(1));
            handle.set_title(format!("Coins inserted: {balance}, price: {price}"));
            if balance >= price {
                handle.set_description("Paid in full!");
                break;
            }
        }
    });

    let choice = select.run().expect("error running select");
    println!("{choice}");
}
