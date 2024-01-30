use demand::Input;

fn main() {
    let not_empty_min_8 = |s: &str| {
        if s.is_empty() {
            return Err("Name cannot be empty");
        }
        if s.len() < 8 {
            return Err("Name must be at least 8 characters");
        }
        Ok(())
    };

    let t = Input::new("What's your name?")
        .description("We'll use this to personalize your experience.")
        .placeholder("Enter your name")
        .prompt("Name: ")
        .validation(not_empty_min_8);
    let i = t.run().expect("error running input");
    println!("Input: {}", i);
}
