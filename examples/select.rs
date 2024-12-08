use demand::{DemandOption, Select};

fn main() {
    let ms = Select::new("Country")
        .description("Pick a country")
        .filterable(true)
        .option(
            DemandOption::new("US")
                .label("United States")
                .selected(true),
        )
        .option(DemandOption::new("DE").label("Germany"))
        .option(DemandOption::new("BR").label("Brazil"))
        .option(DemandOption::new("CA").label("Canada"))
        .option(DemandOption::new("MX").label("Mexico"))
        .option(DemandOption::new("FR").label("France"))
        .option(DemandOption::new("IT").label("Italy"))
        .option(DemandOption::new("ES").label("Spain"))
        .option(DemandOption::new("JP").label("Japan"))
        .option(DemandOption::new("CN").label("China"))
        .option(DemandOption::new("IN").label("India"))
        .option(DemandOption::new("RU").label("Russia"))
        .option(DemandOption::new("AU").label("Australia"))
        .option(DemandOption::new("GB").label("United Kingdom"))
        .option(DemandOption::new("NL").label("Netherlands"))
        .option(DemandOption::new("SE").label("Sweden"))
        .option(DemandOption::new("CH").label("Switzerland"))
        .option(DemandOption::new("NO").label("Norway"))
        .option(DemandOption::new("DK").label("Denmark"))
        .option(DemandOption::new("BE").label("Belgium"))
        .option(DemandOption::new("AT").label("Austria"))
        .option(DemandOption::new("PL").label("Poland"))
        .option(DemandOption::new("TR").label("Turkey"))
        .option(DemandOption::new("CZ").label("Czech Republic"))
        .option(DemandOption::new("IE").label("Ireland"))
        .option(DemandOption::new("SG").label("Singapore"))
        .option(DemandOption::new("HK").label("Hong Kong"))
        .option(DemandOption::new("KR").label("South Korea"))
        .option(DemandOption::new("AR").label("Argentina"))
        .option(DemandOption::new("CL").label("Chile"))
        .option(DemandOption::new("CO").label("Colombia"))
        .option(DemandOption::new("PE").label("Peru"))
        .option(DemandOption::new("VE").label("Venezuela"))
        .option(DemandOption::new("UA").label("Ukraine"))
        .option(DemandOption::new("RO").label("Romania"))
        .option(DemandOption::new("ZA").label("South Africa"))
        .option(DemandOption::new("EG").label("Egypt"))
        .option(DemandOption::new("SA").label("Saudi Arabia"))
        .option(DemandOption::new("AE").label("United Arab Emirates"));
    let _ = match ms.run() {
        Ok(value) => value,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("Input cancelled");
                return;
            } else {
                panic!("Error: {}", e);
            }
        }
    };
}
