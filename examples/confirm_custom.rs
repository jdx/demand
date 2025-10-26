use demand::Confirm;

fn main() {
    let confirm = Confirm::new("Deploy to production?")
        .description("This will deploy the latest changes.")
        .affirmative("Confirm")
        .negative("Cancel");
    match confirm.run() {
        Ok(result) => {
            if result {
                println!("Confirmed!");
            } else {
                println!("Cancelled!");
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
