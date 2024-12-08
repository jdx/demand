use demand::Input;

fn main() {
    let input = Input::new("Set a password")
        .placeholder("Enter password")
        .prompt("Password: ")
        .password(true);
    let _ = match input.run() {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Input cancelled");
                return;
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
