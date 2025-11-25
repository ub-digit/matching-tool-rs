use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use zip::read::ZipArchive;
use crate::matcher::JsonRecord;
use crate::args::Config;
use serde::{Serialize, Deserialize};
use pest::Parser;
use pest_derive::Parser;
use pest::iterators::Pairs;

#[derive(Parser)]
#[grammar = "year_grammar.pest"]
struct YearParser;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRecordLoader {
    #[serde(default)]
    pub title: Option<String>, // title in the vectors
    #[serde(default)]
    pub author: Option<String>, // author in the vectors
    #[serde(default)]
    pub publication_type: Option<String>, // not used for matching
    pub editions: Vec<JsonEditionLoader>, // Partially used. If there are multiple editions, it is treated as if there are multiple records
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEditionLoader {
    #[serde(default)]
    pub part: Option<String>, // not used for matching
    #[serde(default)]
    pub format: Option<String>, // not used for matching
    #[serde(rename = "placeOfPublication", default)]
    pub place_of_publication: Option<String>, // location in the vectors
    #[serde(rename = "yearOfPublication", default)]
    pub year_of_publication: Option<u32>, // year in the vectors
}

// Same structure as JsonRecordLoader, but used for version 2 of the JSON input format
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRecordLoaderV2 {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub title: Option<String>, // title in the vectors
    #[serde(default)]
    pub author: Option<String>, // author in the vectors
    #[serde(default)]
    pub publication_type: Option<String>, // not used for matching
    #[serde(default)]
    pub is_reference_card: bool, // not used for matching
    pub editions: Vec<JsonEditionLoaderV2>, // Partially used. If there are multiple editions, it is treated as if there are multiple records
    #[serde(default)]
    pub invalid_json: bool, // if true, this record is invalid and should be skipped
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum JsonRecordEditionLoaderYearV2 {
    Single(u32),
    Multiple(Vec<u32>),
    None,
}

impl Default for JsonRecordEditionLoaderYearV2 {
    fn default() -> Self {
        JsonRecordEditionLoaderYearV2::None
    }
}

impl From<&JsonRecordEditionLoaderYearV2> for Vec<u32> {
    fn from(year_enum: &JsonRecordEditionLoaderYearV2) -> Self {
        match year_enum {
            JsonRecordEditionLoaderYearV2::Single(y) => vec![*y],
            JsonRecordEditionLoaderYearV2::Multiple(ys) => ys.clone(),
            JsonRecordEditionLoaderYearV2::None => Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonEditionLoaderV2 {
    #[serde(default)]
    pub part: Option<String>, // not used for matching
    #[serde(default)]
    pub format: Option<String>, // not used for matching
    #[serde(default)]
    pub place_of_publication: Vec<String>, // location in the vectors, will be joined with " "
    #[serde(default)]
    pub year_of_publication: JsonRecordEditionLoaderYearV2, // year in the vectors (only the lowest year value that is not 0 will be used and converted to string, or empty string if all values are 0 or there are no values)
    #[serde(default)]
    pub year_of_publication_compact_string: Option<String>, // String that may be parsed by the YearParser to get multiple years
    #[serde(default)]
    pub edition_statement: Option<String>,
    #[serde(default)]
    pub volume_designation: Option<String>,
    #[serde(default)]
    pub serial_titles: Vec<String>,
}

pub fn read_zip_file(config: &Config, file_path: &str, schema_version: i32) -> (String, Vec<(String, JsonRecord)>) {
    let inputdata = read_input_to_btreemap(file_path);
    if schema_version == 2 {
        return convert_to_jsonarray_v2(config, inputdata);
    } else {
        return convert_to_jsonarray(inputdata);
    }
}

fn read_input_to_btreemap(path: &str) -> BTreeMap<String, String> {
    if is_directory(path) {
        read_directory_to_btreemap(path)
    } else {
        read_zip_to_btreemap(path)
    }
}

// Check path and determine if it is a file or a directory
pub fn is_directory(path: &str) -> bool {
    let metadata = std::fs::metadata(path);
    if let Ok(meta) = metadata {
        return meta.is_dir();
    }
    false
}

fn read_zip_to_btreemap(file_path: &str) -> BTreeMap<String, String> {
    // Open the ZIP file
    let file = File::open(file_path).expect("Failed to open file");
    let mut archive = ZipArchive::new(file).expect("Failed to open ZIP file");

    // Initialize the BTreeMap to store filenames and their contents
    let mut file_contents_map = BTreeMap::new();

    // Iterate through each file in the ZIP archive
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("Failed to get file from ZIP archive");
        if file.is_file() {
            // Read the file's content into a buffer
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).expect("Failed to read file");
            let buffer = String::from_utf8_lossy(&buffer).to_string();

            // Insert the filename and its content into the BTreeMap
            file_contents_map.insert(file.name().to_string(), buffer);
        }
    }

    file_contents_map
}

// Read all files (no subdirectories) from a directory into a BTreeMap
fn read_directory_to_btreemap(dir_path: &str) -> BTreeMap<String, String> {
    let mut file_contents_map = BTreeMap::new();
    let entries = std::fs::read_dir(dir_path).expect("Failed to read directory");
    for entry in entries {
        let entry = entry.expect("Failed to get directory entry");
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let content = std::fs::read_to_string(&path).expect("Failed to read file");
            file_contents_map.insert(filename, content);
        }
    }
    file_contents_map
}

// Return (systemprompt, Vec<JsonRecord>)
fn convert_to_jsonarray(inputdata: BTreeMap<String, String>) -> (String, Vec<(String, JsonRecord)>) {
    let mut jsonarray = Vec::new();
    let mut systemprompt = String::new();
    for (filename, content) in inputdata {
        // First check if the file is the system prompt (a file with the extension .prompt)
        if filename.ends_with(".prompt") {
            systemprompt = content;
            continue;
        }
        // Only handle files with the .json extension
        if !filename.ends_with(".json") {
            continue;
        }
        // Skip any path that starts with __MACOSX
        if filename.starts_with("__MACOSX") {
            continue;
        }
        // Skip any path that starts with .DS_Store
        if filename.starts_with(".DS_Store") {
            continue;
        }
        let record: JsonRecordLoader = match serde_json::from_str(&content) {
            Ok(record) => record,
            Err(e) => {
                // Try to load as a JsonRecordArrayLoader and if there is one and only one record,
                // use that record, otherwise panic for every other reason.
                if let Ok(mut json_array) = serde_json::from_str::<Vec<JsonRecordLoader>>(&content) {
                    if json_array.len() == 1 {
                        json_array.pop().unwrap() // At this point we know there is exactly one record
                    } else {
                        panic!("Expected one record in JSON array, found {}", json_array.len());
                    }
                } else {
                    panic!("Failed to parse JSON file {}: {}", filename, e);
                }
            }
        };
        for (edition_idx, edition) in record.editions.iter().enumerate() {
            let jsonrecord = JsonRecord {
                edition: edition_idx,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: edition.place_of_publication.clone().unwrap_or_default(),
                year: edition.year_of_publication.clone().unwrap_or_default().to_string(),
                publication_type: record.publication_type.clone().unwrap_or_default(),
                allowed_years: Vec::new(), // Not used in version 1
            };
            jsonarray.push((filename.clone(), jsonrecord));
        }
        // Special handling for case where there are no editions. Here we set the edition to 9999999
        if record.editions.is_empty() {
            let jsonrecord = JsonRecord {
                edition: 9999999,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: String::new(),
                year: String::new(),
                publication_type: record.publication_type.clone().unwrap_or_default(),
                allowed_years: Vec::new(), // Not used in version 1
            };
            jsonarray.push((filename.clone(), jsonrecord));
        }
    }
    (systemprompt, jsonarray)
}

fn convert_to_jsonarray_v2(config: &Config, inputdata: BTreeMap<String, String>) -> (String, Vec<(String, JsonRecord)>) {
    let mut jsonarray = Vec::new();
    let mut systemprompt = String::new();
    for (filename, content) in inputdata {
        // First check if the file is the system prompt (a file with the extension .prompt)
        if filename.ends_with(".prompt") {
            systemprompt = content;
            continue;
        }
        // Only handle files with the .json extension
        if !filename.ends_with(".json") {
            continue;
        }
        // Skip any path that starts with __MACOSX
        if filename.starts_with("__MACOSX") {
            continue;
        }
        // Skip any path that starts with .DS_Store
        if filename.starts_with(".DS_Store") {
            continue;
        }
        let record: JsonRecordLoaderV2 = match serde_json::from_str(&content) {
            Ok(record) => record,
            Err(e) => {
                // Try to load as a JsonRecordArrayLoader and if there is one and only one record,
                // use that record, otherwise panic for every other reason.
                if let Ok(mut json_array) = serde_json::from_str::<Vec<JsonRecordLoaderV2>>(&content) {
                    if json_array.len() == 1 {
                        json_array.pop().unwrap() // At this point we know there is exactly one record
                    } else {
                        if config.verbose {
                            println!("Expected one record in JSON array, found {}", json_array.len());
                        }
                        create_invalid_json_loader_record_v2()
                    }
                } else {
                    if config.verbose {
                        println!("Failed to parse JSON file {}: {}", filename, e);
                    }
                    create_invalid_json_loader_record_v2()
                }
            }
        };
        let publication_type_string = match (&record.is_reference_card, &record.publication_type) {
            (true, _) => "cross-reference".to_string(),
            (false, Some(pt)) => pt.to_string(),
            (false, None) => "".to_string(),
        };
        let basename = filename.split('/').last().unwrap_or(&filename).to_string();
        for (edition_idx, edition) in record.editions.iter().enumerate() {
            let edition_years = extract_years(config, edition);
            let lowest_non_zero_year = match &edition_years {
                JsonRecordEditionLoaderYearV2::Single(y) => *y,
                JsonRecordEditionLoaderYearV2::Multiple(ys) => ys.iter().filter(|y| **y > 0).min().cloned().unwrap_or(0),
                JsonRecordEditionLoaderYearV2::None => 0,
            };
            let year_string = if lowest_non_zero_year > 0 { lowest_non_zero_year.to_string() } else { String::new() };
            let mut title = record.title.clone().unwrap_or_default();
            // If option "add_serial_to_title" is set, append "serial_titles" field (array joined with a space) to the title joined with a space
            if config.options.add_serial_to_title {
                let serial_titles = edition.serial_titles.join(" ").trim().to_string();
                if !serial_titles.is_empty() {
                    title = format!("{} {}", title, serial_titles);
                }
            }
            // If option "add_edition_to_title" is set, append "edition_statement" field (Option<String>) to the title joined with a space
            if config.options.add_edition_to_title {
                if let Some(edition_str) = &edition.edition_statement {
                    if !edition_str.trim().is_empty() {
                        title = format!("{} {}", title, edition_str);
                    }
                }
            }

            let jsonrecord = JsonRecord {
                edition: edition_idx,
                title: title,
                author: record.author.clone().unwrap_or_default(),
                location: edition.place_of_publication.clone().join(" "),
                year: year_string,
                publication_type: publication_type_string.clone(),
                allowed_years: (&edition_years).into(),
            };
            jsonarray.push((basename.clone(), jsonrecord));
        }
        // Special handling for case where there are no editions. Here we set the edition to 9999999
        if record.editions.is_empty() && !record.invalid_json {
            let jsonrecord = JsonRecord {
                edition: 9999999,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: String::new(),
                year: String::new(),
                publication_type: publication_type_string.clone(),
                allowed_years: Vec::new(),
            };
            jsonarray.push((basename.clone(), jsonrecord));
        }
        if record.invalid_json {
            let jsonrecord = JsonRecord {
                edition: 9999998,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: String::new(),
                year: String::new(),
                publication_type: publication_type_string.clone(),
                allowed_years: Vec::new(),
            };
            jsonarray.push((basename.clone(), jsonrecord));
        }            
    }
    (systemprompt, jsonarray)
}

fn create_invalid_json_loader_record_v2() -> JsonRecordLoaderV2 {
    JsonRecordLoaderV2 {
        schema_version: None,
        title: Some("INVALID JSON".to_string()),
        author: Some("INVALID JSON".to_string()),
        publication_type: Some("INVALID JSON".to_string()),
        is_reference_card: false,
        editions: Vec::new(),
        invalid_json: true,
    }
}

fn extract_years(config: &Config, edition: &JsonEditionLoaderV2) -> JsonRecordEditionLoaderYearV2 {
    if config.options.parse_year_ranges {
        if let Some(year_string) = &edition.year_of_publication_compact_string {
            match parse_year_string(year_string) {
                Ok(years) => {
                    if config.options.use_first_parsed_year {
                        if let Some(first_year) = first_year(&years) {
                            return JsonRecordEditionLoaderYearV2::Single(first_year);
                        } else {
                            return JsonRecordEditionLoaderYearV2::None;
                        }
                    } else {
                        return JsonRecordEditionLoaderYearV2::Multiple(years);
                    }
                }
                Err(_) => {
                    return edition.year_of_publication.clone();
                }
            }
        } else {
            return edition.year_of_publication.clone();
        }
    } else {
        return edition.year_of_publication.clone();
    }
}

// Parse year string using YearParser.
// It will return a vec of u32 years from strings of style "1949", "1949-", "1949-1951", and comma-separated combinations of these, e.g. "1949, 1951-1954, 1956-"
// That example will return 1949, 1951, 1952, 1953, 1954, 1956
fn parse_year_string(year_string: &str) -> Result<Vec<u32>, pest::error::Error<Rule>> {
    let pairs = YearParser::parse(Rule::main, &year_string)?;
    let mut years = Vec::new();
    create_year_array(pairs, &mut years);
    Ok(years)
}

fn create_year_array(pairs: Pairs<Rule>, years: &mut Vec<u32>) {
    for pair in pairs {
        match pair.as_rule() {
            Rule::year => {
                let year_int = pair.as_str().parse::<u32>().unwrap();
                years.push(year_int);
            }
            Rule::year_range => {
                let mut inner_pairs = pair.into_inner();
                let start_year = inner_pairs.next().unwrap().as_str().parse::<i32>().unwrap();
                let end_year = inner_pairs.next().unwrap().as_str().parse::<i32>().unwrap();
                for year in start_year..=end_year {
                    years.push(year as u32);
                }
            }
            // Special case, only use start_year as a single year
            Rule::year_range_open => {
                let start_year = pair.into_inner().next().unwrap().as_str().parse::<i32>().unwrap();
                years.push(start_year as u32);
            }
            _ => {
                create_year_array(pair.into_inner(), years);
            }
        }
    }
}

fn first_year(year_array: &[u32]) -> Option<u32> {
    year_array.iter().cloned().min()
}