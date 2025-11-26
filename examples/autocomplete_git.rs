use demand::{Autocomplete, Input};
use std::collections::HashMap;

#[derive(Clone)]
struct GitCompleter {
    commands: HashMap<&'static str, Vec<&'static str>>,
}

impl GitCompleter {
    fn new() -> Self {
        let mut commands = HashMap::new();

        commands.insert(
            "git",
            vec![
                "add", "bisect", "branch", "checkout", "clone", "commit", "diff", "fetch", "grep",
                "init", "log", "merge", "mv", "pull", "push", "rebase", "reset", "restore", "rm",
                "show", "stash", "status", "switch", "tag",
            ],
        );

        commands.insert("branch", vec!["-a", "-d", "-D", "-m", "-r", "--list"]);
        commands.insert(
            "checkout",
            vec!["-b", "-B", "--detach", "--orphan", "--ours", "--theirs"],
        );
        commands.insert(
            "commit",
            vec!["-m", "-a", "--amend", "--no-edit", "-v", "--fixup"],
        );
        commands.insert("log", vec!["--oneline", "--graph", "--all", "-n", "--stat"]);
        commands.insert("push", vec!["--force", "-u", "--tags", "--dry-run"]);
        commands.insert("pull", vec!["--rebase", "--no-rebase", "--ff-only"]);
        commands.insert(
            "stash",
            vec!["push", "pop", "list", "show", "drop", "clear"],
        );
        commands.insert("reset", vec!["--soft", "--hard", "--mixed", "HEAD~1"]);
        commands.insert("rebase", vec!["-i", "--continue", "--abort", "--skip"]);

        Self { commands }
    }
}

impl Autocomplete for GitCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            [] => Ok(vec!["git ".to_string()]),
            ["git"] if !input.ends_with(' ') => Ok(vec!["git ".to_string()]),

            ["git"] if input.ends_with(' ') => Ok(self.commands["git"]
                .iter()
                .map(|cmd| format!("git {} ", cmd))
                .collect()),

            ["git", partial] if !input.ends_with(' ') => Ok(self.commands["git"]
                .iter()
                .filter(|cmd| cmd.starts_with(partial))
                .map(|cmd| format!("git {} ", cmd))
                .collect()),

            ["git", cmd] if input.ends_with(' ') => {
                if let Some(opts) = self.commands.get(cmd) {
                    Ok(opts.iter().map(|o| format!("git {} {}", cmd, o)).collect())
                } else {
                    Ok(Vec::new())
                }
            }

            ["git", cmd, partial] if !input.ends_with(' ') => {
                if let Some(opts) = self.commands.get(cmd) {
                    Ok(opts
                        .iter()
                        .filter(|o| o.starts_with(partial))
                        .map(|o| format!("git {} {}", cmd, o))
                        .collect())
                } else {
                    Ok(Vec::new())
                }
            }

            _ => Ok(Vec::new()),
        }
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(suggestion) = highlighted_suggestion {
            return Ok(Some(suggestion.to_string()));
        }

        let suggestions = self.get_suggestions(input)?;
        if suggestions.len() == 1 {
            return Ok(Some(suggestions[0].clone()));
        }

        if !suggestions.is_empty() {
            let first = &suggestions[0];
            let common_len = suggestions.iter().fold(first.len(), |acc, s| {
                first
                    .chars()
                    .zip(s.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
                    .min(acc)
            });

            if common_len > input.len() {
                return Ok(Some(first[..common_len].to_string()));
            }
        }

        Ok(None)
    }
}

fn main() {
    let input = Input::new("Enter a git command:")
        .description("Tab to autocomplete, arrows to navigate suggestions")
        .placeholder("git ")
        .default_value("git ")
        .autocomplete(GitCompleter::new())
        .max_suggestions_display(8);

    match input.run() {
        Ok(cmd) => println!("Command: {}", cmd),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Cancelled");
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}

