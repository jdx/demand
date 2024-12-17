use demand::{Dialog, DialogButton};

fn main() {
    let dialog = Dialog::new("Are you sure?")
        .description("This will do a thing.")
        .buttons(vec![
            DialogButton::new("Ok"),
            DialogButton::new("Not sure"),
            DialogButton::new("Cancel"),
        ])
        .selected_button(1);
    match dialog.run() {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
                return;
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
