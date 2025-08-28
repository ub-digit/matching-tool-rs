pub mod csv;
pub mod xlsx;
pub mod text;
pub mod json;

use crate::args::Config;
use crate::matcher::OutputRecord;
use crate::args::OutputFormat;
use serde::{Deserialize, Serialize};

pub enum Cell {
    String(String),
    Number(f64),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Output {
    Stdout,
    File(String),
}

pub fn output_records(config: &Config, records: &[OutputRecord]) {
    // Create the output directory for options with path if it does not exist
    if let Output::File(path) = &config.output {
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).expect("Unable to create output directory");
        }
    }
    match (config.output_format, &config.output) {
        (OutputFormat::Text, Output::Stdout) => text::output_records(config,  records),
        (OutputFormat::Json, Output::File(path)) => json::output_records(config, path, records),
        (OutputFormat::CSV, Output::File(path)) => csv::output_records(config, path, records),
        (OutputFormat::XLSX, Output::File(path)) => xlsx::output_records(config, path, records),
        _ => unimplemented!("Output format not implemented"),
    }
}

