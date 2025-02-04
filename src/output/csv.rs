use std::fs::File;
use std::io::{BufWriter, Write};
use crate::args::Config;
use crate::output::Cell;
use crate::matcher::OutputRecord;

pub fn output_records(config: &Config, path: &str, records: &[OutputRecord]) {
    let headers = build_headers(config);
    let rows = build_rows(config, records);
    output_csv_file(path, &headers, &rows);
}

fn build_headers(config: &Config) -> Vec<String> {
    let mut headers = vec!["card".to_string(), "edition_idx".to_string(), "title".to_string(), "author".to_string(), "location".to_string(), "year".to_string(), "match_stat".to_string(), "id".to_string(), "similarity".to_string(), "zscore".to_string()];
    if config.options.include_source_data {
        headers.push("source_title".to_string());
        headers.push("source_author".to_string());
        headers.push("source_location".to_string());
        headers.push("source_year".to_string());
    }
    headers
}

fn build_rows(config: &Config, records: &[OutputRecord]) -> Vec<Vec<Cell>> {
    records.iter().flat_map(|record| {
        let mut rows = vec![];
        for (source_record, similarity, zscore) in &record.top {
            let mut row = vec![
                Cell::String(record.card.clone()),
                Cell::Number(record.record.edition as f64),
                Cell::String(record.record.title.clone()),
                Cell::String(record.record.author.clone()),
                Cell::String(record.record.location.clone()),
                Cell::String(record.record.year.to_string()),
                Cell::String(record.stats.to_string()),
                Cell::String(source_record.id.clone()),
                Cell::Number(*similarity as f64),
                Cell::Number(*zscore as f64),
            ];
            if config.options.include_source_data {
                row.push(Cell::String(source_record.title.clone()));
                row.push(Cell::String(source_record.author.clone()));
                row.push(Cell::String(source_record.location.clone()));
                row.push(Cell::String(source_record.year.to_string()));
            }
            rows.push(row);
        }
        rows
    }).collect()
}

fn output_csv_file(path: &str, headers: &[String], rows: &[Vec<Cell>]) {
    let file = File::create(path).expect("Unable to create file");
    let mut writer = BufWriter::new(&file);
    output_csv_header(&mut writer, headers);
    for row in rows {
        output_csv_row(&mut writer, row);
    }
}

// When outputting in CSV format, there are two options based on the include-source-data option:
// 1. If include-source-data is set, the output will include the source data for the matched records
//  => card, edition_idx, title, author, location, year, match_stat, id, similarity, zscore, source_title, source_author, source_location, source_year
// 2. If include-source-data is not set, the output will only include the matched records
//  => card, edition_idx, title, author, location, year, match_stat, id, similarity, zscore
fn output_csv_header(output: &mut dyn Write, headers: &[String]) {
    let _ = writeln!(output, "{}", headers.join("\t"));
}

fn output_csv_row(output: &mut dyn Write, row: &[Cell]) {
    let row_str = row.iter().map(|cell| match cell {
        Cell::String(s) => s.to_string(),
        Cell::Number(n) => n.to_string(),
    }).collect::<Vec<String>>().join("\t");
    let _ = writeln!(output, "{}", row_str);
}
