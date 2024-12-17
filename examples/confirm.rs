use demand::Confirm;

fn main() {
    let confirm = Confirm::new("Are you sure?")
        .description("This will do a thing.")
        .affirmative("Yes!")
        .negative("No.");
    match confirm.run() {
        Ok(confirm) => confirm,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
                false
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
