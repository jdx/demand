use demand::{DemandOption, MultiSelect};

fn main() {
    let multiselect = MultiSelect::new("Toppings")
        .description("Select your toppings")
        .min(1)
        .max(4)
        .filterable(true)
        .option(DemandOption::new("Lettuce").selected(true))
        .option(DemandOption::new("Tomatoes").selected(true))
        .option(DemandOption::new("Charm Sauce"))
        .option(DemandOption::new("Jalapenos").label("Jalapeños"))
        .option(DemandOption::new("Cheese"))
        .option(DemandOption::new("Vegan Cheese"))
        .option(DemandOption::new("Nutella"))
        .option(DemandOption::new("Peanut Butter"))
        .option(DemandOption::new("Banana"))
        .option(DemandOption::new("Strawberries"))
        .option(DemandOption::new("Blueberries"))
        .option(DemandOption::new("Pineapple"))
        .option(DemandOption::new("Mango"))
        .option(DemandOption::new("Kiwi"))
        .option(DemandOption::new("Passion Fruit"))
        .option(DemandOption::new("Peaches"))
        .option(DemandOption::new("Raspberries"))
        .option(DemandOption::new("Blackberries"))
        .option(DemandOption::new("Mint"))
        .option(DemandOption::new("Chocolate Chips"))
        .option(DemandOption::new("Oreos"))
        .option(DemandOption::new("Brownie Bites"))
        .option(DemandOption::new("Cookie Dough"))
        .option(DemandOption::new("Graham Cracker Crumbs"))
        .option(DemandOption::new("M&Ms"))
        .option(DemandOption::new("Reese's Pieces"))
        .option(DemandOption::new("Butterfinger"))
        .option(DemandOption::new("Heath Bar"))
        .option(DemandOption::new("Kit Kat"))
        .option(DemandOption::new("Snickers"))
        .option(DemandOption::new("Twix"))
        .option(DemandOption::new("Caramel"))
        .option(DemandOption::new("Hot Fudge"))
        .option(DemandOption::new("Marshmallow"))
        .option(DemandOption::new("Whipped Cream"))
        .option(DemandOption::new("Chocolate Syrup"))
        .option(DemandOption::new("Caramel Syrup"))
        .option(DemandOption::new("Strawberry Syrup"))
        .option(DemandOption::new("Peanut Butter Syrup"))
        .option(DemandOption::new("Nutella Syrup"))
        .option(DemandOption::new("Honey"))
        .option(DemandOption::new("Sprinkles"))
        .option(DemandOption::new("Chocolate Sprinkles"))
        .option(DemandOption::new("Coconut Flakes"))
        .option(DemandOption::new("Almonds"))
        .option(DemandOption::new("Peanuts"))
        .option(DemandOption::new("Walnuts"))
        .option(DemandOption::new("Pecans"))
        .option(DemandOption::new("Cashews"))
        .option(DemandOption::new("Pistachios"))
        .option(DemandOption::new("Macadamia Nuts"))
        .option(DemandOption::new("Hazelnuts"))
        .option(DemandOption::new("Peanut Butter Cups"))
        .option(DemandOption::new("Gummy Bears"))
        .option(DemandOption::new("Sour Patch Kids"))
        .option(DemandOption::new("Sour Gummy Worms"))
        .option(DemandOption::new("Sour Skittles"))
        .option(DemandOption::new("Skittles"))
        .option(DemandOption::new("Starburst"))
        .option(DemandOption::new("Twizzlers"))
        .option(DemandOption::new("Milk Duds"));
    match multiselect.run() {
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
