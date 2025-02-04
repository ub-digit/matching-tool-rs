use crate::vocab::Vocab;
use crate::elastic::{self, Pagination, Record};
use crate::tokenizer;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Vectors {
    pub source: String,
    pub total_docs: u32,
    pub documents: Vec<Document>,
}

impl Vectors {
    pub fn new(source: &str, total_docs: u32) -> Vectors {
        Vectors {
            source: source.to_string(),
            total_docs,
            documents: vec![],
        }
    }

    pub fn save(&self, file: &str) {
        let mut writer = std::io::BufWriter::new(std::fs::File::create(file).unwrap());
        bincode::serialize_into(&mut writer, self).unwrap();
    }

    pub fn load(file: &str) -> Vectors {
        println!("Loading vectors from {}", file);
        let reader = std::io::BufReader::new(std::fs::File::open(file).unwrap());
        bincode::deserialize_from(reader).unwrap()
    }
}

type VectorIndex = u32;

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub vectors: HashMap<String, Vec<(VectorIndex, f32)>>,
}

pub fn build_dataset_vectors(config: &crate::args::Config) {
    let vocab = Vocab::load(&config.vocab_file);
    if config.verbose {
        println!("Loaded vocab from {}", config.vocab_file);
    }
    let vectors = process_source(&config.source, &vocab);
    vectors.save(&config.dataset_vector_file);
}

fn process_source(source: &str, vocab: &Vocab) -> Vectors {
    let mut vectors = Vectors::new(source, 0);
    let mut counter = 0;
    let mut records = elastic::fetch_source(source, Pagination::Initial, 0);
    loop {
        if let Ok((_, Pagination::Done, _)) = records {
            break;
        }
        if let Ok((new_records, new_pagination, total_count)) = records {
            counter += new_records.len() as u32;
            if counter % 10000 == 0 {
                println!("Processing {} records from {}", counter, source);
                // if counter >= 100000 {
                //     return counter;
                // }
            }

            // if counter >= 100000 {
            //     break;
            // }

            for record in new_records {
                // println!("Record: {:?}", record);
                let doc = process_record(&record, vocab);
                vectors.documents.push(doc);
                // println!("Document: {:?}", doc);
                // std::process::exit(1);
            }
            records = elastic::fetch_source(source, new_pagination, total_count);
        }
    }
    println!("Processed {} records in {}", counter, source);
    vectors.total_docs = counter;
    vectors
}

// Tokenize each of author, title, location, year and combined (all)
// Calculate the tf-idf for each word in each part
// There should be a tf-idf vector for each part
pub fn process_record(record: &Record, vocab: &Vocab) -> Document {
    let id = record.id.clone();
    let author_vec = process_part("author", &tokenizer::tokenize_string(&record.author), vocab);
    let title_vec = process_part("title", &tokenizer::tokenize_string(&record.title), vocab);
    let location_vec = process_part("location", &tokenizer::tokenize_string(&record.location), vocab);
    let year_vec = process_part("year", &tokenizer::tokenize_year(&record.year), vocab);
    let all_vec = process_part("all", &tokenizer::tokenize_string(&record.combined()), vocab);
    let mut vectors = HashMap::new();
    vectors.insert("author".to_string(), author_vec);
    vectors.insert("title".to_string(), title_vec);
    vectors.insert("location".to_string(), location_vec);
    vectors.insert("year".to_string(), year_vec);
    vectors.insert("all".to_string(), all_vec);
    Document { id, vectors }
}

fn process_part(part: &str, tokens: &HashMap<String, usize>, vocab: &Vocab) -> Vec<(VectorIndex, f32)> {
    let vocab_part = &vocab.vocab_parts[part];
    let mut tf = vec![0.0; vocab.words.len()];
    for (token, _) in tokens {
        if let Some((index, _)) = vocab_part.tokens.get(token) {
            tf[*index] += 1.0;
        } else {
            tf[0] += 1.0;
        }
    }
    tfraw(&mut tf);
    let mut sparse_tf_idf = vec![];
    for (index, count) in tf.iter().enumerate() {
        if *count <= 0.0 {
            continue;
        }
        let idf = vocab_part.idf[index];
        // Alternatively: use:
        // (*count as f64 * idf).sqrt() as f32
        sparse_tf_idf.push((index as VectorIndex, (*count as f64 * idf) as f32));
    }

    sparse_tf_idf
}

#[allow(dead_code)]
fn tfraw(vector: &mut Vec<f64>) {
    for value in vector.iter_mut() {
        *value = *value;
    }
}

#[allow(dead_code)]
fn tflog(vector: &mut Vec<f64>) {
    for value in vector.iter_mut() {
        *value = (1.0 + *value).log10();
    }
}

#[allow(dead_code)]
fn tfmax(vector: &mut Vec<f64>) {
    let max = vector.iter().cloned().fold(0. / 0., f64::max);
    for value in vector.iter_mut() {
        *value = 0.5 + 0.5 * (*value / max);
    }
}