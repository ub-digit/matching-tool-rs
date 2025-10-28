use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use zip::read::ZipArchive;
use crate::matcher::{JsonRecord, JsonRecordLoader, JsonRecordLoaderV2};
use crate::args::Config;

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
                        panic!("Expected one record in JSON array, found {}", json_array.len());
                    }
                } else {
                    panic!("Failed to parse JSON file {}: {}", filename, e);
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
            let lowest_non_zero_year = edition.year_of_publication.iter().filter(|y| **y > 0).min().cloned().unwrap_or(0);
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
            };
            jsonarray.push((basename.clone(), jsonrecord));
        }
        // Special handling for case where there are no editions. Here we set the edition to 9999999
        if record.editions.is_empty() {
            let jsonrecord = JsonRecord {
                edition: 9999999,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: String::new(),
                year: String::new(),
                publication_type: publication_type_string.clone(),
            };
            jsonarray.push((basename, jsonrecord));
        }
    }
    (systemprompt, jsonarray)
}