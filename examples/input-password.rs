use demand::Input;

fn main() {
    let t = Input::new("Set a password")
        .placeholder("Enter password")
        .prompt("Password: ")
        .password(true);
    let i = t.run().expect("error running input");
    println!("Password: {}", i);
}
