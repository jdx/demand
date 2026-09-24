use demand::{Input, InputValidator};

fn main() {
    // `run_parsed` validates and parses in one step, returning a `usize`.
    let max_length = Input::new("What is the max. length of a name?")
        .run_parsed(parse_usize)
        .expect("a max length");

    let name_validator = NameValidation { max_length };

    let name = Input::new("What's your name?")
        .validator(name_validator)
        .run()
        .expect("a name");

    println!("Welcome {name}");
}

fn parse_usize(input: &str) -> Result<usize, &'static str> {
    input.parse().map_err(|_| "Expected a positive integer")
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
