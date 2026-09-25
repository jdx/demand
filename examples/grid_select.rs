use demand::{GridRow, GridSelect};

fn main() {
    let columns = ["Current", "Range", "Latest"];
    let grid = GridSelect::new("Pick the packages you want to upgrade")
        .description("Choose a version for each package")
        .columns(columns)
        .filterable(true)
        .row(
            GridRow::new("@floating-ui/react-dom")
                .cell("^0.4.3")
                .empty_cell()
                .cell("^0.5.0"),
        )
        .row(
            GridRow::new("@prisma/client")
                .cell("^3.8.1")
                .cell("^3.10.0"),
        )
        .row(
            GridRow::new("@types/node")
                .cell("^16.11.21")
                .cell("^16.11.26")
                .cell("^17.0.21"),
        )
        .row(
            GridRow::new("chalk")
                .cell("^4.1.2")
                .empty_cell()
                .cell("^5.0.0"),
        )
        .row(GridRow::new("jest").cell("^27.4.7").cell("^27.5.1"));
    match grid.run() {
        Ok(choices) => {
            for (package, column) in choices {
                println!("{package}: {}", columns[column]);
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::Interrupted {
                println!("{}", e);
            } else {
                panic!("Error: {}", e);
            }
        }
    }
}
