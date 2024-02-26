use demand::Confirm;

fn main() {
    let ms = Confirm::new("Are you sure?")
        .description("This will do a thing.")
        .affirmative("Yes!")
        .negative("No.");
    ms.run().expect("error running confirm");
}
