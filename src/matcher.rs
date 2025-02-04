use crate::args::Config;
use crate::vocab::Vocab;
use crate::vectorize::{self, Vectors, Document};
use crate::elastic::Record as ElasticRecord;
use crate::source_data::{self, SourceRecord};
use crate::report;
use crate::output;
use crate::zipfile;
use serde::{Serialize, Deserialize};
// use std::collections::{HashMap, BTreeMap};
use std::collections::BTreeMap;
use rustc_hash::FxHashMap;
use rayon::prelude::*;

pub const TOP_N: usize = 10;


// JSON input format:
// {
//     "title": "Fra A til Z. Tidsskrift for typografi og grafik",
//     "author": "Viggo Naae, Kai Pelt & Ib Hoy Petersen",
//     "editions": [
//         {
//             "part": "1",
//             "format": "8:o",
//             "placeOfPublication": "[K\u00f8benhavn]",
//             "yearOfPublication": 1948
//         }
//     ]
// }

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRecordLoader {
    #[serde(default)]
    pub title: Option<String>, // title in the vectors
    #[serde(default)]
    pub author: Option<String>, // author in the vectors
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

#[derive(Debug, Clone)]
pub struct JsonRecord {
    pub edition: usize,
    pub title: String,
    pub author: String,
    pub location: String,
    pub year: String,
}

impl From<&JsonRecord> for ElasticRecord {
    fn from(json_record: &JsonRecord) -> Self {
        ElasticRecord {
            id: "".to_string(),
            source: "json_record".to_string(),
            title: json_record.title.clone(),
            author: json_record.author.clone(),
            location: json_record.location.clone(),
            year: json_record.year.clone(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum MatchStat {
    SingleMatch,
    MultipleMatches,
    NoMatch,
    Unqualified, // Single not reaching min_single_similarity
    NoEdition, // No edition in the JSON record
    NA,
}

impl MatchStat {
    pub fn to_str(&self) -> &str {
        match self {
            MatchStat::SingleMatch => "Single",
            MatchStat::MultipleMatches => "Multiple",
            MatchStat::NoMatch => "No match",
            MatchStat::Unqualified => "Unqualified",
            MatchStat::NoEdition => "No edition",
            MatchStat::NA => "",
        }
    }
    pub fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

#[derive(Debug)] 
pub struct OutputRecord {
    pub card: String,
    pub record: JsonRecord,
    pub top: Vec<(SourceRecord, f32, f32)>,
    pub stats: MatchStat,
}

impl OutputRecord {
    pub fn new(_config: &Config, card: &str, record: &JsonRecord, top: &[(String, f32, f32)], stats: MatchStat, source_data_records: &FxHashMap<String, SourceRecord>) -> OutputRecord {
        // Remap top into a vector of (SourceRecord, f32, f32)
        let mut top_source_records = vec![];

        for (id, similarity, zscore) in top {
            if let Some(source_record) = source_data_records.get(id) {
                top_source_records.push((source_record.clone(), *similarity, *zscore));
            }
        }

        let mut new_record = record.clone();
        if let MatchStat::NoEdition = stats {
            new_record.edition = 0;
        }

        OutputRecord {
            card: card.to_string(),
            record: new_record,
            top: top_source_records,
            stats,
        }
    }
}

#[derive(Debug, Default)]
pub struct MatchStatistics {
    pub match_types: FxHashMap<MatchStat, usize>,
    pub number_of_records: usize,
    pub cards: FxHashMap<String, bool>,
    pub prompt_used: String,
}

impl MatchStatistics {
    pub fn update(&mut self, stat: &MatchStat, card: &str) {
        // If stat is NoEdition, we don't update any statistics other than the cards
        if let MatchStat::NoEdition = stat {
            self.cards.insert(card.to_string(), true);
            return;
        }
        // Add or increase stat to match_types
        let entry = self.match_types.entry(stat.clone()).or_insert(0);
        *entry += 1;
        self.number_of_records += 1;
        // Add card to cards
        self.cards.insert(card.to_string(), true);
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt_used = prompt.to_string();
    }

    pub fn number_of_cards(&self) -> usize {
        self.cards.len()
    }

    pub fn match_stat(&self, stat: &MatchStat) -> usize {
        *self.match_types.get(stat).unwrap_or(&0)
    }

    pub fn match_stat_percent(&self, stat: &MatchStat) -> f32 {
        let total = self.number_of_records as f32;
        let matches = self.match_stat(stat) as f32;
        (matches / total) * 100.0
    }
}

struct DatasetWeightedVector {
    id: String,
    vector: Vec<(u32, f32)>,
    dot: f32,
}

fn precalc_weighted_average_vectors_for_source(config: &Config, dataset_vectors: &Vectors, weights: &FxHashMap<String, f32>) -> Vec<DatasetWeightedVector> {
    if config.verbose {
        println!("Calculating weighted average vectors for {}", config.source);
    }
    // dataset_vectors.documents.iter()
    dataset_vectors.documents.par_iter()
        .map(|document| {
            let combined_vector = weighted_averaged_vector(&document, &weights);
            let dot = dot_product(&combined_vector, &combined_vector);
            DatasetWeightedVector {
                id: document.id.clone(),
                vector: combined_vector,
                dot: dot.sqrt(),
            }
        })
        .collect()
}

// Reads a zip file with json-files into Vec<JsonRecord>
// via a Vec<JsonRecordLoader>
pub fn match_json_zip(config: &Config) {
    let vocab = Vocab::load(&config.vocab_file);
    let dataset_vectors = Vectors::load(&config.dataset_vector_file);
    let source_data = source_data::SourceData::load(&config.source_data_file);
    let source_data_records = source_data.records;
    let mut statistics = MatchStatistics::default();
    let mut output_records = Vec::new();

    let weights = vector_weights(config);
    // let weights = unit_weights();
    let dataset_weighted_vectors = precalc_weighted_average_vectors_for_source(config, &dataset_vectors, &weights);
    let (prompt, records) = read_json_zip_file(config, &config.input);
    statistics.set_prompt(&prompt);
    for (card, record) in records {
        if config.verbose {
            print!("Processing record: {} {} => ", card, record.edition);
        }
        if record.edition > 100000 {
            if config.verbose {
                println!("No edition");
            }
            statistics.update(&MatchStat::NoEdition, &card);
            output_records.push(OutputRecord::new(config, &card, &record, &vec![], MatchStat::NoEdition, &source_data_records));
            continue;
        }
        let top = process_record(&config, &record, &vocab, &dataset_weighted_vectors, &weights, &source_data_records);
        let stats = get_stats(&config, &top);
        if config.verbose {
            if let MatchStat::NoMatch = stats {
                println!("{}", stats.to_str());
            } else {
                let topmost_similarity = match top.first() {
                    Some((_, similarity, _)) => *similarity,
                    None => 0.0,
                };
                println!("{} ({})", stats.to_str(), topmost_similarity);
            }
        }
        statistics.update(&stats, &card);
        let record_result = OutputRecord::new(config, &card, &record, &top, stats, &source_data_records);
        output_records.push(record_result);
    }
    // Write output
    output::output_records(&config, &output_records);
    // Write report.
    report::output_report(config, &statistics);
}


// Only relevant if similarity-threshold is set
fn get_stats(config: &Config, top: &[(String, f32, f32)]) -> MatchStat {
    // Check if similarity-threshold is set, return NA if not
    if let Some(_) = config.options.similarity_threshold {
        if top.len() == 0 {
            MatchStat::NoMatch
        } else if top.len() == 1 {
            if let Some(min_single_similarity) = config.options.min_single_similarity {
                if top[0].1 < min_single_similarity {
                    MatchStat::Unqualified
                } else {
                    MatchStat::SingleMatch
                }
            } else {
                MatchStat::SingleMatch
            }
        } else {
            MatchStat::MultipleMatches
        }
    } else {
        MatchStat::NA
    }
}

fn process_record(config: &Config, record: &JsonRecord, vocab: &Vocab, dataset_vectors: &[DatasetWeightedVector], weights: &FxHashMap<String, f32>, source_data_records: &FxHashMap<String, SourceRecord>) -> Vec<(String, f32, f32)> {
    // Tokenize each of author, title, location, year and combined (all)
    // Calculate the tf-idf for each word in each part
    // There should be a tf-idf vector for each part
    let input_document = vectorize::process_record(&record.into(), vocab);
    let input_combined_vector = weighted_averaged_vector(&input_document, &weights);
    let self_dot = dot_product(&input_combined_vector, &input_combined_vector).sqrt();
    // Now we loop over all the dataset vectors and calculate the cosine similarity for their weighted average vector
    // We will keep the TOP_N most similar vectors
    // let mut top_n: Vec<(String, f32)> = dataset_vectors.iter()
    let mut top_n: Vec<(String, f32)> = dataset_vectors.par_iter()
        .map(|document| {
            let mut similarity = 
                if config.options.force_year {
                    if let Some(source_record) = source_data_records.get(&document.id) {
                        if record.year == source_record.year || record.year == "0" {
                            cosine_similarity(&input_combined_vector, self_dot, &document.vector, document.dot)
                        } else {
                            0.0
                        }
                    } else {
                        cosine_similarity(&input_combined_vector, self_dot, &document.vector, document.dot)
                    }
                } else {
                    cosine_similarity(&input_combined_vector, self_dot, &document.vector, document.dot)
                };
            if let Some(threshold) = config.options.similarity_threshold {
                if similarity < threshold {
                    similarity = 0.0;
                }
            }
            (document.id.clone(), similarity)
        })
        .collect();
    top_n.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    // Keep only the top N*10 (used for Z-scores)
    top_n.truncate(TOP_N*20);
    // Calculate z-scores for the top N*10
    let mut z_scores = calculate_z_scores(top_n);
    // Sort by z-score and keep the top N
    z_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    // If z-threshold is set, filter out all below the threshold
    if let Some(z_threshold) = config.options.z_threshold {
        z_scores.retain(|(_, _, zscore)| *zscore > z_threshold);
    }
    z_scores.truncate(TOP_N);
    // Filter all where similarity is 0.0
    z_scores.retain(|(_, similarity, _)| *similarity > 0.0);
    // If there is only one match left and min_single_similarity is set, filter out if below threshold
    // if z_scores.len() == 1 {
    //     if let Some(min_single_similarity) = config.options.min_single_similarity {
    //         if z_scores[0].1 < min_single_similarity {
    //             z_scores.clear();
    //         }
    //     }
    // }
    z_scores
}

fn cosine_similarity(vector1: &[(u32, f32)], vector1_selfdot: f32, vector2: &[(u32, f32)], vector2_selfdot: f32) -> f32 {
    let dot = dot_product(vector1, vector2);
    dot / (vector1_selfdot * vector2_selfdot)
}

fn dot_product(vector1: &[(u32, f32)], vector2: &[(u32, f32)]) -> f32 {
    let mut sum = 0.0;
    let mut i = 0;
    let mut j = 0;
    while i < vector1.len() && j < vector2.len() {
        let (index1, value1) = vector1[i];
        let (index2, value2) = vector2[j];
        if index1 == index2 {
            sum += value1 * value2;
            i += 1;
            j += 1;
        } else if index1 < index2 {
            i += 1;
        } else {
            j += 1;
        }
    }
    sum
}

// Document contains a: vectors: HashMap<String, Vec<(VectorIndex, f32)>> with a sparse vector for each part
// The sparse vectors are weighted by the values from the weights hashmap with a simple multiplication
// The return vector is a sparse vector with the weighted average of all the vectors.
// If one part is missing, it is ignored, it is NOT treated as a zero vector or that would skew the result.
fn weighted_averaged_vector(document: &Document, weights: &FxHashMap<String, f32>) -> Vec<(u32, f32)> {
    let mut active_parts = 0;
    let mut intermediate_vector = BTreeMap::new();
    for (part, vector) in &document.vectors {
        // If the vector is of length 0, it is ignored, otherwise it is used and active_parts is incremented
        if vector.len() == 0 {
            continue;
        }
        active_parts += 1;
        let weight = weights.get(part).unwrap();

        // If active_parts is 1, we initialize the intermediate_vector map with the first vector
        if active_parts == 1 {
            intermediate_vector = vector.iter().cloned().map(|(index, value)| (index, value * weight)).collect();
        } else {
            // If active_parts is more than 1, we add the vector to the intermediate_vector
            for (index, value) in vector {
                let entry = intermediate_vector.entry(*index).or_insert(0.0);
                *entry += value * weight;
            }
        }
    }
    let mut combined_vector = vec![];
    // Build as sorted (by key) vector from the intermediate_vector map,
    // and divide by the number of active_parts to get the average
    for (index, value) in intermediate_vector {
        combined_vector.push((index, value / active_parts as f32));
    }
    combined_vector
}

pub fn vector_weights(config: &Config) -> FxHashMap<String, f32> {
    // WeightsFile is a JSON file with a hashmap of part -> weight
    if let Some(ref filename) = config.options.weights_file {
        let file = std::fs::File::open(filename).unwrap();
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    } else {
        default_weights()
    }
}

fn default_weights() -> FxHashMap<String, f32> {
    // Create a hashmap with all parts and a weight of 1.0
    let mut weights = FxHashMap::default();
    weights.insert("author".to_string(), 0.75);
    weights.insert("title".to_string(), 1.5);
    weights.insert("location".to_string(), 1.0);
    weights.insert("year".to_string(), 1.0);
    weights.insert("all".to_string(), 0.0);
    weights
}

// Read from ZIP-file into a Vec<JsonRecord>
// The ZIP-file optionally contains a prompt file.
// Therefor the return type is (String, Vec<(String, JsonRecord)>)
// where the first String is the prompt used, if provided, and the list is ("card", "record")
fn read_json_zip_file(config: &Config, filename: &str) -> (String, Vec<(String, JsonRecord)>) {
    // If filename has extension .zip, read from zip file, otherwise read as normal with an empty prompt
    if filename.ends_with(".zip") {
        if config.verbose {
            println!("Reading zip file: {}", filename);
        }
        return zipfile::read_zip_file(filename);
    }
    // Only support zip-files.
    panic!("Only zip-files are supported as input for match-json-zip");
}

/// Calculate z-scores for a vector of (ID, similarity) pairs.
/// Returns a vector of (ID, similarity, z-score) tuples.
fn calculate_z_scores(data: Vec<(String, f32)>) -> Vec<(String, f32, f32)> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    // Calculate mean
    let mean: f32 = data.iter().map(|(_, similarity)| similarity).sum::<f32>() / n as f32;

    // Calculate standard deviation
    let variance: f32 = data
        .iter()
        .map(|(_, similarity)| (similarity - mean).powi(2))
        .sum::<f32>()
        / n as f32;
    let std_dev = variance.sqrt();

    // Calculate z-scores
    data.into_iter()
        .map(|(id, similarity)| {
            let z_score = if std_dev == 0.0 {
                0.0 // Handle case where std_dev is 0 to avoid division by zero
            } else {
                (similarity - mean) / std_dev
            };
            (id, similarity, z_score)
        })
        .collect()
}
