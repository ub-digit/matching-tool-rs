use rust_xlsxwriter::{Workbook, XlsxError, Format};
use crate::output::Cell;
use crate::args::Config;
use crate::matcher::OutputRecord;

/// Writes data to either an Excel (.xlsx) or OpenDocument Spreadsheet (.ods) file.
///
/// # Arguments
///
/// * `filename` - The name of the file to create.
/// * `data` - A vector of vectors containing the data to write.
///
/// # Errors
///
/// Returns an error if the file extension is not supported or if there is an issue writing the file.

pub fn output_records(config: &Config, path: &str, records: &[OutputRecord]) {
    let headers = build_headers(config);
    let rows = build_rows(config, records);
    write_excel_file(path, &headers, &rows).expect("Unable to write Excel file");
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
        // Special case when there are no matches (top is empty), we write a single row with the record data and No match, and nothing else
        if record.top.len() == 0 {
            let row = vec![
                Cell::String(record.card.clone()),
                Cell::Number(record.record.edition as f64),
                Cell::String(record.record.title.clone()),
                Cell::String(record.record.author.clone()),
                Cell::String(record.record.location.clone()),
                Cell::String(record.record.year.to_string()),
                Cell::String(record.stats.to_string()),
            ];
            return vec![row];
        }
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

fn write_excel_file(path: &str, headers: &[String], rows: &[Vec<Cell>]) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write the headers: card, edition, title, author, location, year
    // in bold
    let bold = Format::new().set_bold();
    let wrap = Format::new().set_text_wrap();

    // Write header row (row 0, 0-indexed column)
    for (col_idx, header) in headers.iter().enumerate() {
        worksheet.write_with_format(0, col_idx as u16, header, &bold)?;
    }

    // Write rows (row 1 and beyond)
    for (row_idx, row) in rows.iter().enumerate() {
        let row_idx = (row_idx + 1) as u32;
        for (col_idx, cell) in row.iter().enumerate() {
            match cell {
                Cell::String(s) => {
                    worksheet.write_with_format(row_idx, col_idx as u16, s, &wrap)?;
                }
                Cell::Number(n) => {
                    worksheet.write_number(row_idx, col_idx as u16, *n)?;
                }
            }
        }
    }

    workbook.save(path)?;
    Ok(())
}

