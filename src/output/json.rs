use crate::args::Config;
use crate::matcher::OutputRecord;
use serde::Serialize;


/// Writes data to a JSON file (.json)
///
/// # Arguments
///
/// * `filename` - The name of the file to create.
/// * `data` - A vector of vectors containing the data to write.
///
/// # Errors
///
/// Returns an error if the file extension is not supported or if there is an issue writing the file.

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum JsonRow {
    Normal(JsonRowNormal),
    Empty(JsonRowEmpty),
    Extended(JsonRowExtended),
}

#[derive(Debug, Serialize)]
struct JsonRowNormal {
    card: String,
    edition_idx: u32,
    title: String,
    author: String,
    location: String,
    year: String,
    match_stat: String,
    id: String,
    similarity: f64,
    zscore: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_year: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonRowEmpty {
    card: String,
    edition_idx: u32,
    title: String,
    author: String,
    location: String,
    year: String,
    match_stat: String,
}

#[derive(Debug, Serialize)]
struct JsonRowExtended {
    #[serde(rename = "box")]
    box_name: String,
    card: String,
    #[serde(rename = "card_ID")]
    card_id: String,
    #[serde(rename = "match_object_ID")]
    match_object_id: String,
    card_type: String,
    #[serde(rename = "matched_ID")]
    matched_id: String,
    json: String,
    edition_idx: u32,
    title: String,
    author: String,
    location: String,
    year: String,
    match_stat: String,
    id: String,
    similarity: f64,
    zscore: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_year: Option<String>,
    original_similarity: f64,
    overlap_score: f64,
    adjusted_overlap_score: f64,
    jaro_winkler_score: f64,
}

pub fn output_records(config: &Config, path: &str, records: &[OutputRecord]) {
    let rows = build_rows(config, records);
    write_json_file(path, &rows).expect("Unable to write JSON file");
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

fn build_normal_row(config: &Config, record: &OutputRecord, rows: &mut Vec<JsonRow>) {
    if record.top.len() == 0 {
        // Special case when there are no matches (top is empty), we write a single row with the record data and No match, and nothing else
        rows.push(JsonRow::Empty(JsonRowEmpty {
            card: record.card.clone(),
            edition_idx: record.record.edition as u32,
            title: record.record.title.clone(),
            author: record.record.author.clone(),
            location: record.record.location.clone(),
            year: record.record.year.to_string(),
            match_stat: record.stats.to_string(),
        }));
        return;
    }
    for candidate in &record.top {
        let source_record_id = if let Some(source_record) = &candidate.source_record {
            source_record.id.clone()
        } else {
            "".to_string()
        };
        let mut row = JsonRowNormal {
            card: record.card.clone(),
            edition_idx: record.record.edition as u32,
            title: record.record.title.clone(),
            author: record.record.author.clone(),
            location: record.record.location.clone(),
            year: record.record.year.to_string(),
            match_stat: record.stats.to_string(),
            id: source_record_id.clone(),
            similarity: candidate.similarity as f64,
            zscore: candidate.zscore as f64,
            source_title: None,
            source_author: None,
            source_location: None,
            source_year: None,
        };
        if config.options.include_source_data {
            if let Some(source_record) = &candidate.source_record {
                row.source_title = Some(source_record.title.clone());
                row.source_author = Some(source_record.author.clone());
                row.source_location = Some(source_record.location.clone());
                row.source_year = Some(source_record.year.to_string());
            }
        }
        rows.push(JsonRow::Normal(row));
    }
}

fn build_extended_row(config: &Config, record: &OutputRecord, rows: &mut Vec<JsonRow>) {
    // record.card is of style "box/card.json" (e.g. "003_00153.json")
    // This gives: box="003", card="00153", json="003_00153.json" (record.card)
    let parts: Vec<&str> = record.card.split('_').collect();
    let box_name = parts.get(0).unwrap_or(&"").to_string();
    let card_name = parts.get(1).unwrap_or(&"").replace(".json", "");
    let json_name = record.card.clone();
    let card_id = format!("{}_{}", box_name, card_name);
    let match_object_id = format!("{}_{}_{}", box_name, card_name, record.record.edition);
    let card_type = translate_publication_type(&record.record.publication_type);
    if record.top.len() == 0 {
        // Special case when there are no matches (top is empty), we write a single row with the record data and No match, and nothing else
        rows.push(JsonRow::Empty(JsonRowEmpty {
            card: card_name,
            edition_idx: record.record.edition as u32,
            title: record.record.title.clone(),
            author: record.record.author.clone(),
            location: record.record.location.clone(),
            year: record.record.year.to_string(),
            match_stat: record.stats.to_string(),
        }));
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
        let mut row = JsonRowExtended {
            box_name: box_name.clone(),
            card: card_name.clone(),
            card_id: card_id.clone(),
            match_object_id: match_object_id.clone(),
            card_type: card_type.clone(),
            matched_id: matched_id.to_string(),
            json: json_name.clone(),
            edition_idx: record.record.edition as u32,
            title: record.record.title.clone(),
            author: record.record.author.clone(),
            location: record.record.location.clone(),
            year: record.record.year.to_string(),
            match_stat: record.stats.to_string(),
            id: source_record_id.clone(),
            similarity: candidate.similarity as f64,
            zscore: candidate.zscore as f64,
            source_title: None,
            source_author: None,
            source_location: None,
            source_year: None,
            original_similarity: candidate.original_similarity as f64,
            overlap_score: candidate.overlap_score as f64,
            adjusted_overlap_score: candidate.adjusted_overlap_score as f64,
            jaro_winkler_score: candidate.jaro_winkler_score as f64,
        };
        if config.options.include_source_data {
            if let Some(source_record) = &candidate.source_record {
                row.source_title = Some(source_record.title.clone());
                row.source_author = Some(source_record.author.clone());
                row.source_location = Some(source_record.location.clone());
                row.source_year = Some(source_record.year.to_string());
            }
        }
        rows.push(JsonRow::Extended(row));
    }
}

fn build_rows(config: &Config, records: &[OutputRecord]) -> Vec<JsonRow> {
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

fn write_json_file(path: &str, rows: &[JsonRow]) -> Result<(), std::io::Error> {
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, rows)?;
    Ok(())
}

