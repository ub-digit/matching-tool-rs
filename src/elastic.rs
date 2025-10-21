// Reqwest (blocking)

use reqwest::blocking::Client;
use serde_json::json;
use crate::args::Config;

const ELASTIC_URL: &str = "http://localhost:9200";
const INDEX_NAME: &str = "records";
const MAX_RECORDS: u32 = 10000000;

pub enum Pagination {
    Scroll(String),
    Initial,
    Done,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Record {
    pub id: String,
    pub source: String,
    pub title: String,
    pub author: String,
    pub location: String, // From publisher property
    pub year: String, // From first_year property
}

impl Record {
    // Combine main fields in the order author, title, location, year
    pub fn combined(&self) -> String {
        format!("{} {} {} {}", self.author, self.title, self.location, self.year)
    }
}

// Fetch all documents from the index where source:<source_name>
// Use the scroll API to fetch all documents in pages
pub fn fetch_source(config: &Config, source_name: &str, pagination: Pagination, total_count: u32) -> Result<(Vec<Record>, Pagination, u32), reqwest::Error> {
    match pagination {
        Pagination::Initial => fetch_initial(config, source_name),
        Pagination::Scroll(scroll_id) => fetch_scroll(config, &scroll_id, total_count),
        Pagination::Done => Ok((vec![], Pagination::Done, total_count)),
    }
}

fn get_as_string(value: &serde_json::Value) -> String {
    // If value is an array, join the strings with " "
    // If value is a string, return it
    // otherwise return an empty string
    match value {
        serde_json::Value::Array(array) => array.iter().map(|v| v.as_str().unwrap()).collect::<Vec<&str>>().join(" "),
        serde_json::Value::String(string) => string.to_string(),
        _ => "".to_string(),
    }
}

// Break out everything after the response since it is the same for both fetch_scroll and fetch_initial
fn handle_response(config: &Config, response: reqwest::blocking::Response, total_count: u32) -> Result<(Vec<Record>, Pagination, u32), reqwest::Error> {
    let response_json: serde_json::Value = response.json()?;
    let scroll_id = response_json["_scroll_id"].as_str().unwrap();
    let hits = response_json["hits"]["hits"].as_array().unwrap();

    // If there are no hits, return an empty vector and Pagination::Done
    if hits.is_empty() {
        return Ok((vec![], Pagination::Done, total_count));
    }

    if total_count >= MAX_RECORDS {
        return Ok((vec![], Pagination::Done, total_count));
    }

    let records = hits.iter().map(|hit| {
        let source = hit["_source"].clone();
        let year = match &source["first_year"] {
            serde_json::Value::String(year_str) => year_str.clone(),
            serde_json::Value::Number(year_num) => year_num.to_string(),
            _ => "".to_string(),
        };
        Record {
            id: source["id"].as_str().unwrap().to_string(),
            source: config.options.output_source_name.clone(),
            title: get_as_string(&source["title"]),
            author: get_as_string(&source["author"]),
            location: get_as_string(&source["publisher"]),
            year: year,
        }
    }).collect();

    Ok((records, Pagination::Scroll(scroll_id.to_string()), total_count + hits.len() as u32))
}

fn fetch_scroll(config: &Config, scroll_id: &str, total_count: u32) -> Result<(Vec<Record>, Pagination, u32), reqwest::Error> {
    let url = format!("{}/_search/scroll", ELASTIC_URL);
    let client = Client::new();
    let body = json!({
        "scroll": "1m",
        "scroll_id": scroll_id
    });

    let response = client.post(&url)
        .json(&body)
        .send()?;

    handle_response(config, response, total_count)
 }

fn fetch_initial(config: &Config, source_name: &str) -> Result<(Vec<Record>, Pagination, u32), reqwest::Error> {
    let url = format!("{}/{}/_search?scroll=1m", ELASTIC_URL, INDEX_NAME);
    let client = Client::new();

    // Size to fetch in each scroll is the minimum of MAX_RECORDS and 10000
    let size = MAX_RECORDS.min(10000);

    let body = json!({
        "query": {
            "match": {
                "source": source_name
            }
        },
        "size": size
    });

    let response = client.post(&url)
        .json(&body)
        .send()?;

    handle_response(config, response, 0)
}