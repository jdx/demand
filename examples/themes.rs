use std::env::args;

use demand::{Confirm, DemandOption, Input, MultiSelect, Theme};

fn handle_run<T>(result: Result<T, std::io::Error>) -> T {
    match result {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
                std::process::exit(0);
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}

fn main() {
    let theme = match args().nth(1).unwrap_or_default().as_str() {
        "base16" => Theme::base16(),
        "charm" => Theme::charm(),
        "catppuccin" => Theme::catppuccin(),
        "dracula" => Theme::dracula(),
        "" => Theme::new(),
        theme => unimplemented!("theme {} not implemented", theme),
    };

    let i = Input::new("What's your e-mail?")
        .description("Please enter your e-mail address.")
        .placeholder("john.doe@acme.com")
        .theme(&theme);
    handle_run(i.run());

    let ms = MultiSelect::new("Interests")
        .description("Select your interests")
        .min(1)
        .max(4)
        .filterable(true)
        .option(DemandOption::new("Art"))
        .option(DemandOption::new("Books"))
        .option(DemandOption::new("Food"))
        .option(DemandOption::new("Music"))
        .option(DemandOption::new("Technology"))
        .option(DemandOption::new("Travel"))
        .option(DemandOption::new("Sports"))
        .theme(&theme);
    handle_run(ms.run());

    let c = Confirm::new("Confirm privacy policy")
        .description("Do you accept the privacy policy?")
        .affirmative("Yes")
        .negative("No")
        .theme(&theme);
    handle_run(c.run());
}
