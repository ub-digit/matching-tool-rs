// Finns:

// 4 källor

// Varje källa har:
// - author (field: author, type: string)
// - title (field: title, type: string)
// - location (field: publisher, type: string)
// - year (field: first_year, type: string, format: yyyy)

// Vokabulär finns per varje enskild källa separat.

// Innehåll (per källa):

// - totalt antal dokument
// - lista på alla tokens
// - för varje token:
//   - antal dokument där token förekommer i "author" (2-3-gram)
//   - antal dokument där token förekommer i "title" (2-3-gram)
//   - antal dokument där token förekommer i "location" (2-3-gram)
//   - antal dokument där token förekommer i "year" (här är token bara hela året)
//   - antal dokument där token förekommer i "#{author} #{title} #{location} #{year}" (2-3-gram)

use crate::tokenizer;
use crate::elastic;
use crate::elastic::Pagination;
use crate::args::Config;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Vocab {
    pub source: String,
    pub total_docs: TotalDocs,
    pub words: Vec<String>,
    pub vocab_parts: HashMap<String, VocabPart>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VocabPartType {
    Ngram,
    Year,
}

type WordIndex = usize;
type DocCount = u32;
type TotalDocs = u32;

#[derive(Debug, Serialize, Deserialize)]
pub struct VocabPart {
    pub part_type: VocabPartType,
    pub tokens: HashMap<String, (WordIndex, DocCount)>, // (index_in_vocab_words, document_count_for_token)
    pub idf: Vec<f64>, // Same order as words, idf pre-calculated
}

impl VocabPart {
    pub fn new(part_type: VocabPartType) -> VocabPart {
        let mut tokens = HashMap::new();
        tokens.insert(tokenizer::UNKNOWN.to_string(), (0, 0));
        let idf = vec![];
        VocabPart {
            part_type,
            tokens,
            idf,
        }
    }
}

impl Vocab {
    pub fn new(config: &Config, source: &str) -> Vocab {
        let mut words_vec = vec![tokenizer::UNKNOWN.to_string()];
        let mut words_map = HashMap::new();
        words_map.insert(tokenizer::UNKNOWN.to_string(), 0);
        let mut vocab_parts = HashMap::new();
        vocab_parts.insert("author".to_string(), VocabPart::new(VocabPartType::Ngram));
        vocab_parts.insert("title".to_string(), VocabPart::new(VocabPartType::Ngram));
        vocab_parts.insert("location".to_string(), VocabPart::new(VocabPartType::Ngram));
        vocab_parts.insert("year".to_string(), VocabPart::new(VocabPartType::Year));
        vocab_parts.insert("all".to_string(), VocabPart::new(VocabPartType::Ngram));
        let total_docs = process_source(config, source, &mut words_vec, &mut words_map, &mut vocab_parts);
        // Loop through the vocab_parts hashmap to calculate the idf for each part
        for (_, vocab_part) in vocab_parts.iter_mut() {
            vocab_part.idf = calculate_idf(words_vec.len(), total_docs, &vocab_part.tokens);
        }
        Vocab {
            source: config.options.output_source_name.clone(),
            total_docs,
            words: words_vec,
            vocab_parts,
        }
    }

    pub fn save(&self, path: &str) {
        let file = std::fs::File::create(path).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn load(path: &str) -> Vocab {
        println!("Loading vocab from {}", path);
        let file = std::fs::File::open(path).unwrap();
        bincode::deserialize_from(file).unwrap()
    }

    pub fn print_vocab_stats(&self) {
        println!("Total documents: {}", self.total_docs);
        println!("Total words: {}", self.words.len());
        println!("Vocab parts:");
        for (part_name, vocab_part) in self.vocab_parts.iter() {
            println!(" - {} Tokens: {}", part_name, vocab_part.tokens.len());
        }
    }
}

pub fn build_vocab(config: &Config) {
    let source = &config.source;
    let output_filename = &config.vocab_file;
    let vocab = Vocab::new(config, source);
    vocab.print_vocab_stats();
    vocab.save(output_filename);
}

fn calculate_idf(vocab_size: usize, total_docs: TotalDocs, doc_counts: &HashMap<String, (WordIndex, DocCount)>) -> Vec<f64> {
    let mut idfs = vec![0.0; vocab_size];
    for (_, (index, doc_count)) in doc_counts.iter() {
        let idf = calculate_single_idf(total_docs, *doc_count);
        idfs[*index] = idf;
    }
    idfs
}

fn calculate_single_idf(total_docs: TotalDocs, doc_count: DocCount) -> f64 {
    if doc_count == 0 {
        return 0.0;
    }
    let doc_count = doc_count as f64;
    let total_docs = total_docs as f64;
    let idf = total_docs / doc_count;
    idf.log10()
}

fn process_source(config: &Config, source: &str, words_vec: &mut Vec<String>, words_map: &mut HashMap<String, usize>, vocab_parts: &mut HashMap<String, VocabPart>) -> TotalDocs {
    let mut counter = 0;
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
                process_record(&record, words_vec, words_map, vocab_parts);
            }
            records = elastic::fetch_source(config, source, new_pagination, total_count);
        }
    }
    println!("Processed {} records in {}", counter, config.options.output_source_name);
    counter
}

fn process_record(record: &elastic::Record, words_vec: &mut Vec<String>, words_map: &mut HashMap<String, usize>, vocab_parts: &mut HashMap<String, VocabPart>) {
    process_record_part(&record.author, words_vec, words_map, vocab_parts.get_mut("author").unwrap());
    process_record_part(&record.title, words_vec, words_map, vocab_parts.get_mut("title").unwrap());
    process_record_part(&record.location, words_vec, words_map, vocab_parts.get_mut("location").unwrap());
    process_record_part(&record.year, words_vec, words_map, vocab_parts.get_mut("year").unwrap());
    process_record_part(&record.combined(), words_vec, words_map, vocab_parts.get_mut("all").unwrap());
}

fn process_record_part(record_part: &str, words_vec: &mut Vec<String>, words_map: &mut HashMap<String, usize>, vocab_part: &mut VocabPart) {
    let tokens_count = 
        match vocab_part.part_type {
            VocabPartType::Ngram => tokenizer::tokenize_string(record_part),
            VocabPartType::Year => tokenizer::tokenize_year(record_part),
        };
    // Loop through the tokens_count hashmap.
    // For each token, check if it exists in the words vector and get its index.
    // If it doesn't exist, add it to the words vector and get its index.
    // Check the token in the vocab_part tokens hashmap.
    // If it doesn't exist, add it to the tokens hashmap with the index from the words vector and a document count of 1.
    // If it exists, increment the document count.
    for (token, _) in tokens_count {
        let index = 
            if let Some(&index) = words_map.get(&token) {
                index
            } else {
                words_vec.push(token.to_string());
                let last_index = words_vec.len() - 1;
                words_map.insert(token.to_string(), last_index);
                last_index
            };
        let (_, doc_count) = vocab_part.tokens.entry(token).or_insert((index, 0));
        *doc_count += 1;
    }
}
