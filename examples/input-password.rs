use demand::Input;

fn main() {
    let input = Input::new("Set a password")
        .placeholder("Enter password")
        .prompt("Password: ")
        .password(true);
    match input.run() {
        Ok(_) => {}
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}
