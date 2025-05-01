use demand::{Confirm, DemandOption, Input, List, MultiSelect, Select, Spinner, Theme};

fn main() {
    let spinner = Spinner::new("im out here");
    spinner
        .run(|_| {
            Confirm::new("confirm")
                .description("it says confirm")
                .run()
                .unwrap();
            Input::new("input")
                .description("go on say something")
                .suggestions(&["hello there"])
                .validation(|s: &str| match !s.contains('j') {
                    true => Ok(()),
                    false => Err("ew stinky 'j' not welcome here"),
                })
                .theme(&Theme::catppuccin())
                .placeholder("Words go here")
                .run()
                .unwrap();
            Select::new("select")
                .description("hi")
                .options(vec![
                    DemandOption::new("hi"),
                    DemandOption::new("hello"),
                    DemandOption::new("how are you"),
                ])
                .filterable(true)
                .run()
                .unwrap();
            MultiSelect::new("more select")
                .description("hewo")
                .options(vec![
                    DemandOption::new("hi"),
                    DemandOption::new("hello"),
                    DemandOption::new("how are you"),
                ])
                .filterable(true)
                .run()
                .unwrap();
            let _ = List::new("hewo")
                .description("so many things")
                .items(&[
                    "hi",
                    "hewo",
                    "how are u",
                    "i like trains",
                    ":3",
                    "do you like jazz",
                    "hehe",
                ])
                .filterable(true)
                .run();
            // Spinner::new("spinnerception")
            //     .run(|| std::thread::sleep(std::time::Duration::from_secs(1)))
        })
        .unwrap();
}
