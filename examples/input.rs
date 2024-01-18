use demand::Input;

fn main() {
    let t = Input::new("What's your name?")
        .description("We'll use this to personalize your experience.")
        .placeholder("Enter your name")
        .prompt("Name: ");
    let i = t.run().expect("error running input");
    println!("Input: {}", i);
}
