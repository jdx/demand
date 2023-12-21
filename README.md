# demand

![Crates.io](https://img.shields.io/crates/d/demand)
![docs.rs](https://img.shields.io/docsrs/demand)
![GitHub License](https://img.shields.io/github/license/jdx/demand)
![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/jdx/demand/test.yml)
![GitHub issues](https://img.shields.io/github/issues/jdx/demand)

A prompt library for Rust. Based on [huh? for Go](https://github.com/charmbracelet/huh).

## Input

Single-line text input.

TODO

## Text

Multi-line text input.

TODO

## Select

Select from a list of options.

TODO

## Multiselect

Select multiple options from a list.
Run example with [`cargo run --example multiselect`](./examples/multiselect.rs).

![Multiselect](./assets/multiselect.png)

```rust
use demand::{DemandOption, MultiSelect};

fn main() {
    let ms = MultiSelect::new("Toppings")
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
        .option(DemandOption::new("Nutella"));
    ms.run().expect("error running multi select");
}
```

## Confirm

Confirm a question with a yes or no.
