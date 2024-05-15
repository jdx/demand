use demand::{Dialog, DialogButton};

fn main() {
    let ms = Dialog::new("Are you sure?")
        .description("This will do a thing.")
        .buttons(vec![
            DialogButton::new("Ok"),
            DialogButton::new("Not sure"),
            DialogButton::new("Cancel"),
        ])
        .selected_button(1);
    ms.run().expect("error running confirm");
}
