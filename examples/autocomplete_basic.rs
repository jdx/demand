use demand::Input;

fn main() {
    let input = Input::new("Select a programming language:")
        .description("Start typing to see suggestions")
        .placeholder("e.g., rust, python, zig")
        .autocomplete_fn(|input| {
            let languages = [
                "rust",
                "python",
                "typescript",
                "go",
                "java",
                "c",
                "c++",
                "c#",
                "ruby",
                "swift",
                "kotlin",
                "scala",
                "haskell",
                "elixir",
                "clojure",
                "zig",
            ];

            let suggestions: Vec<String> = languages
                .iter()
                .filter(|lang| {
                    input.is_empty() || lang.to_lowercase().contains(&input.to_lowercase())
                })
                .map(|s| s.to_string())
                .collect();

            Ok(suggestions)
        });

    match input.run() {
        Ok(language) => println!("You selected: {}", language),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Cancelled");
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}
