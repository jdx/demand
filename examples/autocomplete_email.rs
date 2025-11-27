use demand::{Autocomplete, Input};

#[derive(Clone)]
struct EmailCompleter {
    domains: Vec<&'static str>,
}

impl EmailCompleter {
    fn new() -> Self {
        Self {
            domains: vec![
                "gmail.com",
                "yahoo.com",
                "outlook.com",
                "hotmail.com",
                "icloud.com",
                "protonmail.com",
                "fastmail.com",
                "hey.com",
                "pm.me",
                "tutanota.com",
            ],
        }
    }
}

impl Autocomplete for EmailCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if !input.contains('@') {
            return Ok(Vec::new());
        }

        let parts: Vec<&str> = input.splitn(2, '@').collect();
        let username = parts[0];
        let domain_prefix = parts.get(1).unwrap_or(&"");

        let suggestions: Vec<String> = self
            .domains
            .iter()
            .filter(|domain| {
                domain_prefix.is_empty()
                    || domain
                        .to_lowercase()
                        .starts_with(&domain_prefix.to_lowercase())
            })
            .map(|domain| format!("{}@{}", username, domain))
            .collect();

        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(suggestion) = highlighted_suggestion {
            return Ok(Some(suggestion.to_string()));
        }

        if input.contains('@') {
            let suggestions = self.get_suggestions(input)?;
            if suggestions.len() == 1 {
                return Ok(Some(suggestions[0].clone()));
            }
        }

        Ok(None)
    }
}

fn main() {
    let input = Input::new("Enter your email:")
        .description("Type @ to see domain suggestions")
        .placeholder("user@example.com")
        .autocomplete(EmailCompleter::new())
        .validation(|s| {
            if s.contains('@') && s.contains('.') {
                Ok(())
            } else {
                Err("Please enter a valid email address")
            }
        });

    match input.run() {
        Ok(email) => println!("Email entered: {}", email),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Cancelled");
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}
