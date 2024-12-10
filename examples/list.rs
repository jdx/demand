use std::io;

use demand::List;

fn main() {
    let list = List::new("Toppings")
        .description("List of available toppings")
        .item("Lettuce")
        .item("Tomatoes")
        .item("Charm Sauce")
        .item("Jalapenos")
        .item("Cheese")
        .item("Vegan Cheese")
        .item("Nutella")
        .item("Peanut Butter")
        .item("Banana")
        .item("Strawberries")
        .item("Blueberries")
        .item("Pineapple")
        .item("Mango")
        .item("Kiwi")
        .item("Passion Fruit")
        .item("Peaches")
        .item("Raspberries")
        .item("Blackberries")
        .item("Mint")
        .item("Chocolate Chips")
        .item("Oreos")
        .item("Brownie Bites")
        .item("Cookie Dough")
        .item("Graham Cracker Crumbs")
        .item("M&Ms")
        .item("Reese's Pieces")
        .item("Butterfinger")
        .item("Heath Bar")
        .item("Kit Kat")
        .item("Snickers")
        .item("Twix")
        .item("Caramel")
        .item("Hot Fudge")
        .item("Marshmallow")
        .item("Whipped Cream")
        .item("Chocolate Syrup")
        .item("Caramel Syrup")
        .item("Strawberry Syrup")
        .item("Peanut Butter Syrup")
        .item("Nutella Syrup")
        .item("Honey")
        .item("Sprinkles")
        .item("Chocolate Sprinkles")
        .item("Coconut Flakes")
        .item("Almonds")
        .item("Peanuts")
        .item("Walnuts")
        .item("Pecans")
        .item("Cashews")
        .item("Pistachios")
        .item("Macadamia Nuts")
        .item("Hazelnuts")
        .item("Peanut Butter Cups")
        .item("Gummy Bears")
        .item("Sour Patch Kids")
        .item("Sour Gummy Worms")
        .item("Sour Skittles")
        .item("Skittles")
        .item("Starburst")
        .item("Twizzlers")
        .item("Milk Duds")
        .filterable(true);
    match list.run() {
        Ok(_) => {}
        Err(e) => {
            if e.kind() == io::ErrorKind::Interrupted {
                println!("Input cancelled");
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
