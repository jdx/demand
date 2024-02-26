use demand::Input;

fn main() {
    let t = Input::new("Set a password")
        .placeholder("Enter password")
        .prompt("Password: ")
        .password(true);
    t.run().expect("error running input");
}
