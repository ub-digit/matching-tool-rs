use crate::args::Config;
use crate::elastic::{self, Pagination};
// use std::collections::HashMap;
use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct SourceData {
    pub source: String,
    pub records: FxHashMap<String, SourceRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceRecord {
    pub id: String,
    pub title: String,
    pub author: String,
    pub location: String,
    pub year: String,
}

impl SourceData {
    pub fn save(&self, path: &str) {
        let file = std::fs::File::create(path).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn load(path: &str) -> Self {
        println!("Loading source data from {}", path);
        let file = std::fs::File::open(path).unwrap();
        bincode::deserialize_from(file).unwrap()
    }}

pub fn build_source_data(config: &Config) {
    let source_data = process_source(config, &config.source);
    source_data.save(&config.source_data_file);
}

fn process_source(config: &Config, source: &str) -> SourceData {
    let mut counter = 0;
    let mut source_records = FxHashMap::default();
    let mut records = elastic::fetch_source(config, source, Pagination::Initial, 0);
    loop {
        if let Ok((_, Pagination::Done, _)) = records {
            break;
        }
        if let Ok((new_records, new_pagination, total_count)) = records {
            counter += new_records.len() as u32;
            if counter % 10000 == 0 {
                println!("Processing {} records from {}", counter, config.options.output_source_name);
                // if counter >= 100000 {
                //     return counter;
                // }
            }
            for record in new_records {
                let source_record = SourceRecord {
                    id: record.id.clone(),
                    title: record.title,
                    author: record.author,
                    location: record.location,
                    year: record.year,
                };
                source_records.insert(record.id, source_record);
            }
            records = elastic::fetch_source(config, source, new_pagination, total_count);
        }
    }
    println!("Processed {} records in {}", counter, config.options.output_source_name);
    SourceData {
        source: config.options.output_source_name.clone(),
        records: source_records,
    }
}