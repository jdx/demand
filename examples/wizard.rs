use demand::{DemandOption, Input, MultiSelect, Navigation, Select, Wizard};

/// State that persists across wizard sections
#[derive(Default)]
struct SetupState {
    name: String,
    tools: Vec<String>,
    theme: String,
}

fn main() {
    let mut state = SetupState::default();

    let result = Wizard::new("Project Setup")
        .section("Name", |state: &mut SetupState, theme| {
            let name = Input::new("Project name")
                .description("Enter a name for your project")
                .placeholder("my-project")
                .theme(theme)
                .run()?;

            if name.is_empty() {
                state.name = "my-project".to_string();
            } else {
                state.name = name;
            }
            Ok(Navigation::Next)
        })
        .section("Tools", |state: &mut SetupState, theme| {
            let tools = MultiSelect::new("Select tools")
                .description("Choose which tools to configure (Space to toggle, Enter to confirm)")
                .option(DemandOption::new("node").label("Node.js").selected(true))
                .option(DemandOption::new("python").label("Python"))
                .option(DemandOption::new("rust").label("Rust"))
                .option(DemandOption::new("go").label("Go"))
                .option(DemandOption::new("ruby").label("Ruby"))
                .theme(theme)
                .run()?;

            state.tools = tools.iter().map(|s| s.to_string()).collect();
            Ok(Navigation::Next)
        })
        .section("Theme", |state: &mut SetupState, theme| {
            let selected = Select::new("Color theme")
                .description("Choose your preferred theme")
                .option(DemandOption::new("charm").label("Charm (default)"))
                .option(DemandOption::new("dracula").label("Dracula"))
                .option(DemandOption::new("catppuccin").label("Catppuccin"))
                .option(DemandOption::new("base16").label("Base16"))
                .theme(theme)
                .run()?;

            state.theme = selected.to_string();
            Ok(Navigation::Next)
        })
        .section("Done", |state: &mut SetupState, _theme| {
            println!("\n--- Configuration Summary ---");
            println!("Project: {}", state.name);
            println!("Tools: {}", state.tools.join(", "));
            println!("Theme: {}", state.theme);
            println!("-----------------------------\n");
            Ok(Navigation::Done)
        })
        .run(&mut state);

    match result {
        Ok(()) => {
            println!("Setup complete!");
            println!("  Project: {}", state.name);
            println!("  Tools: {:?}", state.tools);
            println!("  Theme: {}", state.theme);
        }
        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
            println!("Setup cancelled");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
