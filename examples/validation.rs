use demand::{Input, InputValidator};

fn main() {
    let max_length = Input::new("What is the max. length of a name?")
        .validation(validate_usize)
        .run()
        .expect("a max length");
    let max_length = max_length.parse().expect("valid usize");

    let name_validator = NameValidation { max_length };

    let name = Input::new("What's your name?")
        .validator(name_validator)
        .run()
        .expect("a name");

    println!("Welcome {name}");
}

fn validate_usize(input: &str) -> Result<(), &'static str> {
    input
        .parse::<usize>()
        .map_err(|_| "Expected a positive integer")?;
    Ok(())
}

struct NameValidation {
    max_length: usize,
}

impl InputValidator for NameValidation {
    fn check(&self, input: &str) -> Result<(), String> {
        if input.len() > self.max_length {
            return Err(format!(
                "Name must be at most {} characters, got {}",
                self.max_length,
                input.len()
            ));
        }
        Ok(())
    }
}
