use comfy_table::{Table, ContentArrangement};

pub trait Tableable {
    fn to_table(&self) -> Table;
}

pub fn render<T: Tableable>(value: &T) {
    let mut table = value.to_table();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    println!("{table}");
}
