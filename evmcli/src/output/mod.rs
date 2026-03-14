pub mod json;
pub mod table;

use serde::Serialize;
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputFormat {
    Json,
    Table,
}

impl OutputFormat {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Self::Json
        } else {
            Self::Table
        }
    }
}

pub fn render<T: Serialize + table::Tableable>(value: &T, format: OutputFormat) {
    match format {
        OutputFormat::Json => json::render(value),
        OutputFormat::Table => table::render(value),
    }
}

pub fn render_error(err: &crate::errors::EvmError, format: OutputFormat) {
    match format {
        OutputFormat::Json => json::render_error(err),
        OutputFormat::Table => {
            eprintln!("Error: {err}");
        }
    }
}
