use demand::Input;

fn main() {
    let input = Input::new("Set a password")
        .placeholder("Enter password")
        .prompt("Password: ")
        .password(true);
    match input.run() {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
                return;
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
