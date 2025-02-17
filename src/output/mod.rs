pub mod csv;
pub mod xlsx;
pub mod text;

use crate::args::Config;
use crate::matcher::OutputRecord;
use crate::args::OutputFormat;

pub enum Cell {
    String(String),
    Number(f64),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Output {
    Stdout,
    File(String),
}

pub fn output_records(config: &Config, records: &[OutputRecord]) {
    match (config.output_format, &config.output) {
        (OutputFormat::Text, Output::Stdout) => text::output_records(config,  records),
        (OutputFormat::CSV, Output::File(path)) => csv::output_records(config, path, records),
        (OutputFormat::XLSX, Output::File(path)) => xlsx::output_records(config, path, records),
        _ => unimplemented!("Output format not implemented"),
    }
}

