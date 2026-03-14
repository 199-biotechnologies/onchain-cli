use serde::Serialize;
use crate::errors::EvmError;

pub fn render<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

pub fn render_error(err: &EvmError) {
    let json = serde_json::json!({
        "error": err.machine_code(),
        "message": err.to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}
