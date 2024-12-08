use demand::Input;

fn main() {
    let notempty_minlen = |s: &str| {
        if s.is_empty() {
            return Err("Name cannot be empty");
        }
        if s.len() < 8 {
            return Err("Name must be at least 8 characters");
        }
        Ok(())
    };

    let input = Input::new("What's your name?")
        .description("We'll use this to personalize your experience.")
        .placeholder("Enter your name")
        .prompt("Name: ")
        .suggestions(&[
            "Adam Grant",
            "Danielle Steel",
            "Eveline Widmer-Schlumpf",
            "Robert De Niro",
            "Ronaldo Rodrigues de Jesus",
            "Sarah Michelle Gellar",
            "Yael Naim",
            "Zack Snyder",
        ])
        .validation(notempty_minlen);
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
