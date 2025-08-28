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
    if config.options.extended_output {
        build_headers_extended(config)
    } else {
        build_headers_normal(config)
    }
}

// Extended output has the headers: box, card, card_ID, match_object_ID, card_type, matched_ID, json
fn build_headers_extended(config: &Config) -> Vec<String> {
    let mut headers = vec![
        "box".to_string(),
        "card".to_string(),
        "card_ID".to_string(),
        "match_object_ID".to_string(),
        "card_type".to_string(),
        "matched_ID".to_string(),
        "json".to_string(),
        "edition_idx".to_string(),
        "title".to_string(),
        "author".to_string(),
        "location".to_string(),
        "year".to_string(),
        "match_stat".to_string(),
        "id".to_string(),
        "similarity".to_string(),
        "zscore".to_string(),
    ];
    if config.options.include_source_data {
        headers.push("source_title".to_string());
        headers.push("source_author".to_string());
        headers.push("source_location".to_string());
        headers.push("source_year".to_string());
    }
    headers.push("original_similarity".to_string());
    headers.push("overlap_score".to_string());
    headers.push("adjusted_overlap_score".to_string());
    headers.push("jaro_winkler_score".to_string());
    headers
}

fn build_headers_normal(config: &Config) -> Vec<String> {
    let mut headers = vec!["card".to_string(), "edition_idx".to_string(), "title".to_string(), "author".to_string(), "location".to_string(), "year".to_string(), "match_stat".to_string(), "id".to_string(), "similarity".to_string(), "zscore".to_string()];
    if config.options.include_source_data {
        headers.push("source_title".to_string());
        headers.push("source_author".to_string());
        headers.push("source_location".to_string());
        headers.push("source_year".to_string());
    }
    headers
}

fn translate_publication_type(publication_type: &str) -> String {
    match publication_type {
        "monographic-component-part" => "Bidrag".to_string(),
        "multi-volume" => "Flerbandsverk".to_string(),
        "periodical" => "Seriell resurs".to_string(),
        "offprint" => "Särtryck".to_string(),
        "facsimile" => "Faksimil".to_string(),
        "cross-reference" => "Hänvisning".to_string(),
        "monograph" => "Monografi".to_string(),
        _ => publication_type.to_string(),
    }
}

fn build_normal_row(config: &Config, record: &OutputRecord, rows: &mut Vec<Vec<Cell>>) {
    if record.top.len() == 0 {
        // Special case when there are no matches (top is empty), we write a single row with the record data and No match, and nothing else
        rows.push(vec![
            Cell::String(record.card.clone()),
            Cell::Number(record.record.edition as f64),
            Cell::String(record.record.title.clone()),
            Cell::String(record.record.author.clone()),
            Cell::String(record.record.location.clone()),
            Cell::String(record.record.year.to_string()),
            Cell::String(record.stats.to_string()),
        ]);
        return;
    }
    for candidate in &record.top {
        let source_record_id = if let Some(source_record) = &candidate.source_record {
            source_record.id.clone()
        } else {
            "".to_string()
        };
        let mut row = vec![
            Cell::String(record.card.clone()),
            Cell::Number(record.record.edition as f64),
            Cell::String(record.record.title.clone()),
            Cell::String(record.record.author.clone()),
            Cell::String(record.record.location.clone()),
            Cell::String(record.record.year.to_string()),
            Cell::String(record.stats.to_string()),
            Cell::String(source_record_id),
            Cell::Number(candidate.similarity as f64),
            Cell::Number(candidate.zscore as f64),
        ];
        if config.options.include_source_data {
            if let Some(source_record) = &candidate.source_record {
                row.push(Cell::String(source_record.title.clone()));
                row.push(Cell::String(source_record.author.clone()));
                row.push(Cell::String(source_record.location.clone()));
                row.push(Cell::String(source_record.year.to_string()));
            } else {
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
            }
        }
        rows.push(row);
    }
}

fn build_extended_row(config: &Config, record: &OutputRecord, rows: &mut Vec<Vec<Cell>>) {
    // record.card is of style "box/card.json" (e.g. "003/00153.json")
    // This gives: box="003", card="00153", json="003/00153.json" (record.card)
    let parts: Vec<&str> = record.card.split('/').collect();
    let box_name = parts.get(0).unwrap_or(&"").to_string();
    let card_name = parts.get(1).unwrap_or(&"").replace(".json", "");
    let json_name = record.card.clone();
    let card_id = format!("{}_{}", box_name, card_name);
    let match_object_id = format!("{}_{}_{}", box_name, card_name, record.record.edition);
    let card_type = translate_publication_type(&record.record.publication_type);
    if record.top.len() == 0 {
        // Special case when there are no matches (top is empty), we write a single row with the record data and No match, and nothing else
        rows.push(vec![
            Cell::String(box_name),
            Cell::String(card_name),
            Cell::String(card_id),
            Cell::String(match_object_id),
            Cell::String(card_type),
            Cell::String("".to_string()),
            Cell::String(json_name),
            Cell::Number(record.record.edition as f64),
            Cell::String(record.record.title.clone()),
            Cell::String(record.record.author.clone()),
            Cell::String(record.record.location.clone()),
            Cell::String(record.record.year.to_string()),
            Cell::String(record.stats.to_string()),
        ]);
        return;
    }
    for candidate in &record.top {
        let source_record_id = if let Some(source_record) = &candidate.source_record {
            source_record.id.clone()
        } else {
            "".to_string()
        };
        // matched_ID is the last part of the source_record.id after the last slash
        let matched_id = source_record_id.split('/').last().unwrap_or("");
        let mut row = vec![
            Cell::String(box_name.clone()),
            Cell::String(card_name.clone()),
            Cell::String(card_id.clone()),
            Cell::String(match_object_id.clone()),
            Cell::String(card_type.clone()),
            Cell::String(matched_id.to_string()),
            Cell::String(json_name.clone()),
            Cell::Number(record.record.edition as f64),
            Cell::String(record.record.title.clone()),
            Cell::String(record.record.author.clone()),
            Cell::String(record.record.location.clone()),
            Cell::String(record.record.year.to_string()),
            Cell::String(record.stats.to_string()),
            Cell::String(source_record_id),
            Cell::Number(candidate.similarity as f64),
            Cell::Number(candidate.zscore as f64),
        ];
        if config.options.include_source_data {
            if let Some(source_record) = &candidate.source_record {
                row.push(Cell::String(source_record.title.clone()));
                row.push(Cell::String(source_record.author.clone()));
                row.push(Cell::String(source_record.location.clone()));
                row.push(Cell::String(source_record.year.to_string()));
            } else {
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
                row.push(Cell::String("".to_string()));
            }
        }
        row.push(Cell::Number(candidate.original_similarity as f64));
        row.push(Cell::Number(candidate.overlap_score as f64));
        row.push(Cell::Number(candidate.adjusted_overlap_score as f64));
        row.push(Cell::Number(candidate.jaro_winkler_score as f64));
        rows.push(row);
    }
}

fn build_rows(config: &Config, records: &[OutputRecord]) -> Vec<Vec<Cell>> {
    records.iter().flat_map(|record| {
        let mut rows = vec![];
        if config.options.extended_output {
            build_extended_row(config, record, &mut rows);
        } else {
            build_normal_row(config, record, &mut rows);
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

