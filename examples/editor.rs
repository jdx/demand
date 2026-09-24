use demand::Editor;

fn main() {
    let notes = Editor::new("Release notes")
        .description("Summarize what changed in this release.")
        .default_value("## Changes\n\n- \n")
        .extension("md")
        .run()
        .expect("error running editor");
    println!("{notes}");
}
