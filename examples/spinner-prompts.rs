use demand::{Confirm, DemandOption, Input, MultiSelect, Select, Spinner};

fn main() {
    let spinner = Spinner::new("im out here");
    spinner
        .run(|| {
            Confirm::new("confirm")
                .description("it says confirm")
                .run()
                .unwrap();
            Input::new("input:")
                .description("go on say something")
                .run()
                .unwrap();
            Select::new("select")
                .description("hi")
                .options(vec![
                    DemandOption::new("hi"),
                    DemandOption::new("hello"),
                    DemandOption::new("how are you"),
                ])
                .run()
                .unwrap();
            MultiSelect::new("more select")
                .description("hewo")
                .options(vec![
                    DemandOption::new("hi"),
                    DemandOption::new("hello"),
                    DemandOption::new("how are you"),
                ])
                .run()
                .unwrap();
        })
        .unwrap();
}
