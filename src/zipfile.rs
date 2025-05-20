use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use zip::read::ZipArchive;
use crate::matcher::{JsonRecord, JsonRecordLoader};

pub fn read_zip_file(file_path: &str) -> (String, Vec<(String, JsonRecord)>) {
    let zipdata = read_zip_to_btreemap(file_path);
    let (systemprompt, jsonarray) = convert_to_jsonarray(zipdata);
    (systemprompt, jsonarray)
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


// Return (systemprompt, Vec<JsonRecord>)
fn convert_to_jsonarray(zipdata: BTreeMap<String, String>) -> (String, Vec<(String, JsonRecord)>) {
    let mut jsonarray = Vec::new();
    let mut systemprompt = String::new();
    for (filename, content) in zipdata {
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
                panic!("Failed to parse JSON file {}: {}", filename, e);
            }
        };
        for (edition_idx, edition) in record.editions.iter().enumerate() {
            let jsonrecord = JsonRecord {
                edition: edition_idx,
                title: record.title.clone().unwrap_or_default(),
                author: record.author.clone().unwrap_or_default(),
                location: edition.place_of_publication.clone().unwrap_or_default(),
                year: edition.year_of_publication.clone().unwrap_or_default().to_string(),
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
            };
            jsonarray.push((filename.clone(), jsonrecord));
        }
    }
    (systemprompt, jsonarray)
}