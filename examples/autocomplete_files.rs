use std::{fs, path::Path};

use demand::{Autocomplete, Input};

#[derive(Clone)]
struct FileCompleter;

impl Autocomplete for FileCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let (dir, prefix) = if input.is_empty() {
            (".", "")
        } else if input.ends_with('/') || input.ends_with('\\') {
            (input, "")
        } else {
            let path = Path::new(input);
            match (path.parent(), path.file_name()) {
                (Some(parent), Some(name)) => {
                    let parent_str = if parent.as_os_str().is_empty() {
                        "."
                    } else {
                        parent.to_str().unwrap_or(".")
                    };
                    (parent_str, name.to_str().unwrap_or(""))
                }
                _ => (".", input),
            }
        };

        let mut suggestions = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                if prefix.is_empty() || name_str.to_lowercase().starts_with(&prefix.to_lowercase())
                {
                    let full_path = if dir == "." {
                        name_str.to_string()
                    } else {
                        format!("{}/{}", dir.trim_end_matches('/'), name_str)
                    };

                    let display = if entry.path().is_dir() {
                        format!("{}/", full_path)
                    } else {
                        full_path
                    };

                    suggestions.push(display);
                }
            }
        }

        suggestions.sort();
        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(highlighted_suggestion.map(|s| s.to_string()))
    }
}

fn main() {
    let input = Input::new("Select a file:")
        .description("Type a path or browse with arrow keys")
        .placeholder("./")
        .autocomplete(FileCompleter)
        .max_suggestions_display(10);

    match input.run() {
        Ok(path) => println!("Selected: {}", path),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Cancelled");
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}
