use crate::args::Config;
use crate::vocab::Vocab;
use crate::vectorize::{self, Vectors, Document};
use crate::elastic::Record as ElasticRecord;
use crate::source_data::{self, SourceRecord};
use crate::report;
use crate::output;
use crate::zipfile;
use crate::overlap::maximal_overlaps;
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
    pub year_of_publication: Vec<u32>, // year in the vectors (only the lowest year value that is not 0 will be used and converted to string, or empty string if all values are 0 or there are no values)
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub volume_enumeration: Option<String>,
    #[serde(default)]
    pub serial_titles: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct JsonRecord {
    pub edition: usize,
    pub title: String,
    pub author: String,
    pub location: String,
    pub year: String,
    pub publication_type: String, // Not used for matching
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

#[derive(Debug, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub enum MatchStat {
    SingleMatch,
    MultipleMatches,
    UnqualifiedMultipleMatches,
    NoMatch,
    Unqualified, // Single not reaching min_single_similarity
    NoEdition, // No edition in the JSON record
    Excluded, // Excluded by id
    NA,
}

impl MatchStat {
    pub fn to_str(&self) -> &str {
        match self {
            MatchStat::SingleMatch => "Single",
            MatchStat::MultipleMatches => "Multiple",
            MatchStat::UnqualifiedMultipleMatches => "Unqualified multiple",
            MatchStat::NoMatch => "No match",
            MatchStat::Unqualified => "Unqualified",
            MatchStat::NoEdition => "No edition",
            MatchStat::Excluded => "Excluded",
            MatchStat::NA => "",
        }
    }
    pub fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

// Struct to hold each candidate during match processing. Unused values (based on options) are set to 0.0
#[derive(Debug, Clone, Default)]
pub struct MatchCandidate {
    pub id: String,
    pub source_record: Option<SourceRecord>, // Added after all filters have been applied to reduce cloning
    pub similarity: f32,
    pub original_similarity: f32, // Before any adjustments
    pub zscore: f32,
    pub overlap_score: f32,
    pub adjusted_overlap_score: f32,
    pub jaro_winkler_score: f32,
}

impl MatchCandidate {
    pub fn new(id: &str, similarity: f32) -> MatchCandidate {
        MatchCandidate {
            id: id.to_string(),
            similarity,
            original_similarity: similarity,
            ..Default::default()
        }
    }
}

#[derive(Debug)] 
pub struct OutputRecord {
    pub card: String,
    pub record: JsonRecord,
    pub top: Vec<MatchCandidate>,
    pub stats: MatchStat,
}

impl OutputRecord {
    pub fn new(_config: &Config, card: &str, record: &JsonRecord, top: &[MatchCandidate], stats: MatchStat, source_data_records: &FxHashMap<String, SourceRecord>) -> OutputRecord {
        // Remap top into a vector of (SourceRecord, f32, f32)
        let mut top_source_records = vec![];

        for candidate in top {
            let mut new_candidate = candidate.clone();
            if let Some(source_record) = source_data_records.get(&candidate.id) {
                new_candidate.source_record = Some(source_record.clone());
                top_source_records.push(new_candidate);
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

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
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
    let (prompt, records) = read_json_zip_file(config, &config.input);
    let vocab = Vocab::load(&config.vocab_file);
    let dataset_vectors = Vectors::load(&config.dataset_vector_file);
    let source_data = source_data::SourceData::load(&config.source_data_file);
    let source_data_records = source_data.records;
    let mut statistics = MatchStatistics::default();
    let mut output_records = Vec::new();

    let weights = vector_weights(config);
    // let weights = unit_weights();
    let dataset_weighted_vectors = precalc_weighted_average_vectors_for_source(config, &dataset_vectors, &weights);
    
    statistics.set_prompt(&prompt);
    for (card, mut record) in records {
        if config.options.add_author_to_title {
            // If config.add_author_to_title is true, we add the author to the title
            // This is used for matching with the source data
            record.title = combine_title_and_author(&record.title, &record.author);
        }
        if config.verbose {
            print!("Processing record: {} {} => ", card, record.edition);
        }
        // Check if id is in input_excluded_ids of format
        // jsonfilename:edition (as one string)
        if input_is_excluded(config, &card, record.edition) {
            if config.verbose {
                println!("Excluded by id");
            }
            statistics.update(&MatchStat::Excluded, &card);
            output_records.push(OutputRecord::new(config, &card, &record, &vec![], MatchStat::Excluded, &source_data_records));
            continue;
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
                    Some(candidate) => candidate.similarity,
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

fn input_is_excluded(config: &Config, card: &str, edition: usize) -> bool {
    let id = format!("{}:{}", card, edition).trim().to_string();
    config.options.input_excluded_ids.contains(&id)
}

// Only relevant if similarity-threshold is set
fn get_stats(config: &Config, top: &[MatchCandidate]) -> MatchStat {
    // Check if similarity-threshold is set, return NA if not
    if let Some(_) = config.options.similarity_threshold {
        if top.len() == 0 {
            MatchStat::NoMatch
        } else if top.len() == 1 {
            if let Some(min_single_similarity) = config.options.min_single_similarity {
                if top[0].similarity < min_single_similarity {
                    MatchStat::Unqualified
                } else {
                    MatchStat::SingleMatch
                }
            } else {
                MatchStat::SingleMatch
            }
        } else {

            if let Some(min_multiple_similarity) = config.options.min_multiple_similarity {
                if top.iter().all(|candidate| candidate.similarity >= min_multiple_similarity) {
                    MatchStat::MultipleMatches
                } else {
                    MatchStat::UnqualifiedMultipleMatches
                }
            } else {
                MatchStat::MultipleMatches
            }
        }
    } else {
        MatchStat::NA
    }
}

fn process_record(config: &Config, record: &JsonRecord, vocab: &Vocab, dataset_vectors: &[DatasetWeightedVector], weights: &FxHashMap<String, f32>, source_data_records: &FxHashMap<String, SourceRecord>) -> Vec<MatchCandidate> {
    // Tokenize each of author, title, location, year and combined (all)
    // Calculate the tf-idf for each word in each part
    // There should be a tf-idf vector for each part
    let input_document = vectorize::process_record(&record.into(), vocab);
    let input_combined_vector = weighted_averaged_vector(&input_document, &weights);
    let self_dot = dot_product(&input_combined_vector, &input_combined_vector).sqrt();
    // Now we loop over all the dataset vectors and calculate the cosine similarity for their weighted average vector
    // We will keep the TOP_N most similar vectors
    // let mut top_n: Vec<(String, f32)> = dataset_vectors.iter()
    let mut top_n: Vec<MatchCandidate> = dataset_vectors.par_iter()
        .map(|document| {
            process_one_item(config, &input_combined_vector, self_dot, record, document, source_data_records)
        })
        .collect();
    top_n.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    // Keep only the top N*10 (used for Z-scores)
    top_n.truncate(TOP_N*20);
    // Apply overlap score to each top_n item (only if option is set)
    apply_overlap_score(config, &mut top_n, &record, source_data_records);
    // Apply Jaro-Winkler to each top_n item (only if option is set)
    apply_jaro_winkler(config, &mut top_n, &record, source_data_records);
    // Calculate z-scores for the top N*10
    let mut z_scores = calculate_z_scores(top_n);
    // Sort by z-score and keep the top N
    z_scores.sort_by(|a, b| b.zscore.partial_cmp(&a.zscore).unwrap());
    // If z-threshold is set, filter out all below the threshold
    if let Some(z_threshold) = config.options.z_threshold {
        z_scores.retain(|candidate| candidate.zscore > z_threshold);
    }
    z_scores.truncate(TOP_N);
    // Filter all where similarity is 0.0
    z_scores.retain(|candidate| candidate.similarity > 0.0);
    // Filter all where similarity is below similarity_threshold and if overlap_adjustment or jaro_winkler_adjustment is set
    if let Some(similarity_threshold) = config.options.similarity_threshold {
        match (config.options.overlap_adjustment, config.options.jaro_winkler_adjustment) {
            (Some(_), _) | (_, true) => {
                z_scores.retain(|candidate| candidate.similarity >= similarity_threshold);
            },
            _ => {}
        }
    }

    z_scores
}

fn process_one_item(config: &Config, input_combined_vector: &[(u32, f32)], self_dot: f32, record: &JsonRecord, document: &DatasetWeightedVector, source_data_records: &FxHashMap<String, SourceRecord>) -> MatchCandidate {
    if config.options.excluded_ids.contains(&document.id) {
        MatchCandidate::new(&document.id, 0.0) // Exclude this id by setting similarity to 0.0
    } else {
        let mut similarity = 
            if config.options.force_year {
                if let Some(source_record) = source_data_records.get(&document.id) {
                    // Case where the year matches exactly (year_tolerance is not set or set to 0)
                    if config.options.year_tolerance.is_none() || config.options.year_tolerance == Some(0) {
                        if record.year == source_record.year || record.year == "0" {
                            cosine_similarity(input_combined_vector, self_dot, &document.vector, document.dot)
                        } else {
                            0.0
                        }
                    // Case where year_tolerance is set to a positive integer
                    } else if let Some(tolerance) = config.options.year_tolerance {
                        if let Ok(record_year) = record.year.parse::<i32>() {
                            if let Ok(source_year) = source_record.year.parse::<i32>() {
                                let year_diff = (record_year - source_year).abs();
                                if year_diff <= tolerance {
                                    let base_similarity = cosine_similarity(input_combined_vector, self_dot, &document.vector, document.dot);
                                    // Apply a penalty based on how far the year is from the source year
                                    let penalty = 1.0 - (year_diff as f32 * config.options.year_tolerance_penalty);
                                    base_similarity * penalty.max(0.0) // Ensure penalty does not go below 0.0
                                } else {
                                    0.0
                                }
                            } else {
                                0.0 // Source year is not a valid number
                            }
                        } else {
                            0.0 // Record year is not a valid number
                        }
                    } else {
                        unreachable!() // This should not happen
                    }
                } else {
                    cosine_similarity(input_combined_vector, self_dot, &document.vector, document.dot)
                }
            } else {
                cosine_similarity(input_combined_vector, self_dot, &document.vector, document.dot)
            };
        if let Some(threshold) = config.options.similarity_threshold {
            if similarity < threshold {
                similarity = 0.0;
            }
        }
        MatchCandidate::new(&document.id.clone(), similarity)
    }
}

// If author has a single comma, split it and join in reverse order with a space
fn swap_author(author: &str) -> String {
    let parts: Vec<&str> = author.split(',').collect();
    if parts.len() == 2 {
        format!("{} {}", parts[1].trim(), parts[0].trim())
    } else {
        author.to_string()
    }
}

fn combine_title_and_author(title: &str, author: &str) -> String {
    // Combine title and author with a slash
    if title.is_empty() && author.is_empty() {
        return "".to_string();
    }
    if title.is_empty() {
        return author.to_string();
    }
    if author.is_empty() {
        return title.to_string();
    }
    // If both title and author are present, strip any trailing whitespace and punctuation from the title
    let title = title.trim_end_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
    // Swap author if it has a single comma
    format!("{} / {}", title, swap_author(author))
}

fn apply_overlap_score(config: &Config, top_n: &mut Vec<MatchCandidate>, input_record: &JsonRecord, source_data_records: &FxHashMap<String, SourceRecord>) {
    if config.options.overlap_adjustment.is_none() {
        return; // No overlap adjustment configured, so return
    }
    // Calculate the overlap score for each top_n item
    for candidate in top_n.iter_mut() {
        if let Some(source_record) = source_data_records.get(&candidate.id) {
            let score = overlap_score(config, &source_record.title, &input_record.title);
            candidate.overlap_score = score;
            let score = overlap_score_adjust(score);
            candidate.adjusted_overlap_score = score;
            candidate.similarity *= score; // Adjust similarity by overlap score
        }
    }
}

fn apply_jaro_winkler(config: &Config, top_n: &mut Vec<MatchCandidate>, input_record: &JsonRecord, source_data_records: &FxHashMap<String, SourceRecord>) {
    if !config.options.jaro_winkler_adjustment {
        return; // No Jaro-Winkler adjustment configured, so return
    }
    // Calculate the Jaro-Winkler score for each top_n item
    for candidate in top_n.iter_mut() {
        if let Some(source_record) = source_data_records.get(&candidate.id) {
            let jw_score = jaro_winkler::jaro_winkler(&source_record.title.to_lowercase(), &input_record.title.to_lowercase());
            candidate.jaro_winkler_score = jw_score as f32;
            candidate.similarity *= jw_score as f32; // Adjust similarity by Jaro-Winkler score
        }
    }
}

// 1-\frac{1}{1+e^{\left(7.5x-2.8\right)}}+0.009
// x == input score
fn overlap_score_adjust(score: f32) -> f32 {
    if score < 0.0 {
        return 0.0;
    }
    if score >= 1.0 {
        return 1.0;
    }
    // Apply the modified sigmoid function to the score
    let exponent = (7.5 * score - 2.8).exp();
    1.0 - 1.0 / (1.0 + exponent) + 0.009
}

// Calculate the overlap score from the pair of source_string and input_string
fn overlap_score(config: &Config, source_string: &str, input_string: &str) -> f32 {
    if config.options.overlap_adjustment.is_none() {
        return 1.0; // No overlap adjustment configured, so return 1.0 keeping the similarity score unchanged
    }
    let overlap_threshold = config.options.overlap_adjustment.unwrap() as usize;
    // If input_string is shorter than overlap_threshold, reduce the threshold to the length of input_string
    let overlap_threshold = overlap_threshold.min(input_string.len());
    if overlap_threshold == 0 {
        return 1.0; // If threshold is 0, return 1.0
    }
    // Calculate the overlap score between source_string and input_string
    let overlap = maximal_overlaps(source_string.to_lowercase(), input_string.to_lowercase());
    // Remove overlaps that are too short (less than N characters)
    let filtered_overlap: Vec<String> = overlap.iter().filter(|o| o.len() >= overlap_threshold).cloned().collect();
    // If there are no overlaps, return 0.0
    if filtered_overlap.is_empty() || input_string.is_empty() {
        return 0.0;
    }
    // Calculate the overlap score as the combined length of the retained overlaps in relation to the input string length
    filtered_overlap.iter().map(|o| o.len() as f32).sum::<f32>() / input_string.len() as f32
}

#[allow(dead_code)]
fn debug_overlap(source_data_records: &FxHashMap<String, SourceRecord>, top: &[(String, f32, f32)], input_document: &JsonRecord) {
    if top.is_empty() {
        return; // No overlaps to debug
    }
    // Debug function to print overlaps of titles
    println!("\nDEBUG: Title: {}", input_document.title);
    for (id, sim, z) in top {
        let mut overlap = vec![];
        if let Some(source_record) = source_data_records.get(id) {
            overlap = maximal_overlaps(source_record.title.to_lowercase(), input_document.title.to_lowercase());
            // Remove overlaps that are too short (less than N characters)
            overlap.retain(|o| o.len() >= 10);
        }
        // Overlap score is the combined length of the retained overlaps in relation to the input title length
        // This will calculate a score between 0.0 and 1.0 where large overlaps indicate a high score
        let overlap_score = if overlap.is_empty() || input_document.title.is_empty() {
            0.0
        } else {
            overlap.iter().map(|o| o.len() as f32).sum::<f32>() / input_document.title.len() as f32
        };
        println!("Overlaps: [{} / {}: {} [{}] / {}], {}: {:?}", overlap_score, overlap_score_adjust(overlap_score), sim, overlap_score_adjust(overlap_score)*sim, z, id, overlap);
    }
    print!("Result: ");
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
        return zipfile::read_zip_file(filename, config.options.json_schema_version);
    }
    // Only support zip-files.
    panic!("Only zip-files are supported as input for match-json-zip");
}

/// Calculate z-scores for a vector of (ID, similarity) pairs.
/// Returns a vector of (ID, similarity, z-score) tuples.
fn calculate_z_scores(mut data: Vec<MatchCandidate>) -> Vec<MatchCandidate> {
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    // Calculate mean
    let mean: f32 = data.iter().map(|candidate| candidate.similarity).sum::<f32>() / n as f32;

    // Calculate standard deviation
    let variance: f32 = data
        .iter()
        .map(|candidate| (candidate.similarity - mean).powi(2))
        .sum::<f32>()
        / n as f32;
    let std_dev = variance.sqrt();

    // Calculate z-scores
    data.iter_mut()
        .for_each(|candidate| {
            let z_score = if std_dev == 0.0 {
                0.0 // Handle case where std_dev is 0 to avoid division by zero
            } else {
                (candidate.similarity - mean) / std_dev
            };
            candidate.zscore = z_score;
        });
    data
}
