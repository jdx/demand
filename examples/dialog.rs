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
    let _ = match dialog.run() {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Dialog was cancelled");
                return;
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
