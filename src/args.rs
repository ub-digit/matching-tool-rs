use clap::Parser;
use crate::cmd::Cmd;
use crate::output::Output;
use std::fmt::{self, Display, Formatter};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::BufReader;

#[derive(Parser)]
struct Args {
    /// Command to run: Available commands: 
    /// 'build-vocab', 'build-dataset-vectors', 'match-json-zip', 'build-source-data' (Default: 'match-json-zip')
    #[clap(short = 'c', long = "command")]
    command: Option<String>,
    /// Source name, required with: 
    /// 'build-vocab', 'build-dataset-vectors', 'match-json-zip', 'build-source-data'
    #[clap(short = 's', long = "source")]
    source: Option<String>,
    /// File to save the vocab to with 'build-vocab' command, later for loading the vocab as well
    /// [Defaults to 'data/<source-name>-vocab.bin']
    #[clap(short = 'V', long = "vocab-file")]
    vocab_file: Option<String>,
    /// File to save the dataset vectors to with 'build-dataset-vectors' command, later for loading the dataset vectors as well
    /// [Defaults to 'data/<source-name>-dataset-vectors.bin']
    #[clap(short = 'D', long = "dataset-vector-file")]
    dataset_vector_file: Option<String>,
    /// File to save the source data to with 'build-source-data' command, later for loading the source data as well
    /// [Defaults to 'data/<source-name>-source-data.bin']
    #[clap(short = 'S', long = "source-data-file")]
    source_data_file: Option<String>,
    /// Input. File or directory to read input from. Format of input depends on the command.
    #[clap(short = 'i', long = "input")]
    input: Option<String>,
    /// Output. File to write output to. Format of output depends on the command. Defaults to stdout.
    #[clap(short = 'o', long = "output")]
    output: Option<String>,
    /// Output format. Format of the output. Available formats: 'text', 'csv', 'xlsx'
    /// [Defaults to 'text']
    #[clap(short = 'F', long = "output-format")]
    output_format: Option<String>,
    /// Print verbose output
    #[clap(short = 'v', long = "verbose")]
    verbose: bool,
    /// Options. Extra options for the command. Format of options depends on the command.
    /// For example, '--option force-year' for 'match-single-json' command (-O force-year)
    #[clap(short = 'O', long = "option")]
    options: Vec<String>,
    /// Load options and weights from a JSON file
    #[clap(short = 'C', long = "config-file")]
    config_file: Option<String>,
}   

#[allow(dead_code)]
#[derive(Debug)]
pub struct Config {
    pub cmd: Cmd,
    pub source: String,
    pub vocab_file: String,
    pub dataset_vector_file: String,
    pub source_data_file: String,
    pub input: String,
    pub output: Output,
    pub output_format: OutputFormat,
    pub verbose: bool,
    pub options: ConfigOptions,
    pub config_file: Option<String>,
    // Only relevant to reduce command output in report, empty in all other cases.
    pub default_args: FxHashMap<String, bool>,
}

pub const DEFAULT_YEAR_TOLERANCE_PENALTY: f32 = 0.25;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigOptions {
    pub force_year: bool,
    // When using force_year, allow for N years difference, otherwise this option is ignored
    pub year_tolerance: Option<i32>,
    // When using year_tolerance, the year difference causes a penalty of year_difference * M (M defaults to 0.25)
    pub year_tolerance_penalty: f32,
    pub include_source_data: bool,
    pub similarity_threshold: Option<f32>,
    pub z_threshold: Option<f32>,
    pub min_single_similarity: Option<f32>,
    pub min_multiple_similarity: Option<f32>,
    pub weights_file: Option<String>,
    pub extended_output: bool,
    pub add_author_to_title: bool,
    pub add_serial_to_title: bool,
    pub add_edition_to_title: bool,
    // Overlap adjustment, the value is the minimum number of characters that must overlap
    pub overlap_adjustment: Option<i32>,
    // Jaro-Winkler adjustment, multiplier to similarity for Jaro-Winkler similarity between titles
    pub jaro_winkler_adjustment: bool,
    // Jaro-Winkler author adjustment, multiplier to similarity for Jaro-Winkler similarity between authors
    pub jaro_winkler_author_adjustment: bool,
    // JSON schema version, version 2 is explicit, all others are version 1
    pub json_schema_version: i32,
    // Output source name (overriding the source parameter which is used for loading from the index). Only used when building vocab, vectors and source data.
    pub output_source_name: String,
    // Base directory for vocab/dataset-vectors/source-data, defaults to "data"
    pub dataset_dir: String,
    // List of files containing IDs (one per line) to exclude from matching
    pub exclude_files: Vec<String>,
    // List of IDs to exclude from matching, populated from exclude_files
    pub excluded_ids: Vec<String>,
    // Same as exclude_files, but for input data only
    pub input_exclude_files: Vec<String>,
    // Same as excluded_ids, but for input data only
    pub input_excluded_ids: Vec<String>,
}

impl ConfigOptions {
    fn f32_option(s: &str) -> f32 {
        s.split('=').collect::<Vec<&str>>()[1].parse::<f32>().unwrap()
    }

    fn i32_option(s: &str) -> i32 {
        s.split('=').collect::<Vec<&str>>()[1].parse::<i32>().unwrap()
    }
    
    fn string_option(s: &str) -> String {
        s.split('=').collect::<Vec<&str>>()[1].to_string()
    }
        
    fn option_name(s: &str) -> &str {
        s.split('=').collect::<Vec<&str>>()[0]
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    CSV,
    XLSX,
}

impl From<String> for OutputFormat {
    fn from(s: String) -> Self {
        match s.as_str() {
            "text" => OutputFormat::Text,
            "json" => OutputFormat::Json,
            "csv" => OutputFormat::CSV,
            "xlsx" => OutputFormat::XLSX,
            _ => OutputFormat::Text,
        }
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::CSV => write!(f, "csv"),
            OutputFormat::XLSX => write!(f, "xlsx"),
        }
    }
}

impl Config {
    pub fn new() -> Config {
        let args = Args::parse();
        let options = parse_options(&args);
        parse_command(&args, options)
    }
}

fn parse_options(args: &Args) -> ConfigOptions {
    let mut options = ConfigOptions {
        force_year: false,
        year_tolerance: None,
        year_tolerance_penalty: DEFAULT_YEAR_TOLERANCE_PENALTY,
        include_source_data: false,
        similarity_threshold: None,
        z_threshold: None,
        min_single_similarity: None,
        min_multiple_similarity: None,
        weights_file: None,
        extended_output: false,
        add_author_to_title: false,
        add_serial_to_title: false,
        add_edition_to_title: false,
        overlap_adjustment: None,
        jaro_winkler_adjustment: false,
        jaro_winkler_author_adjustment: false,
        json_schema_version: 1,
        output_source_name: args.source.clone().unwrap_or_default(),
        dataset_dir: "data".to_string(),
        exclude_files: vec![],
        excluded_ids: vec![],
        input_exclude_files: vec![],
        input_excluded_ids: vec![],
    };

    if let Some(config_file) = &args.config_file {
        // Load options from JSON file
        load_options_from_file(config_file, &mut options);
    }

    for option in args.options.clone() {
        match ConfigOptions::option_name(&option) {
            "force-year" => options.force_year = true,
            "year-tolerance" => {
                let value = ConfigOptions::i32_option(&option);
                options.year_tolerance = Some(value);
            },
            "year-tolerance-penalty" => {
                let value = ConfigOptions::f32_option(&option);
                options.year_tolerance_penalty = value;
            },
            "include-source-data" => options.include_source_data = true,
            "similarity-threshold" => {
                let value = ConfigOptions::f32_option(&option);
                options.similarity_threshold = Some(value);
            },
            "z-threshold" => {
                let value = ConfigOptions::f32_option(&option);
                options.z_threshold = Some(value);
            },
            "min-single-similarity" => {
                let value = ConfigOptions::f32_option(&option);
                options.min_single_similarity = Some(value);
            },
            "min-multiple-similarity" => {
                let value = ConfigOptions::f32_option(&option);
                options.min_multiple_similarity = Some(value);
            },
            "weights-file" => {
                let value = ConfigOptions::string_option(&option);
                options.weights_file = Some(value);
            },
            "extended-output" => options.extended_output = true,
            "add-author-to-title" => options.add_author_to_title = true,
            "add-serial-to-title" => options.add_serial_to_title = true,
            "add-edition-to-title" => options.add_edition_to_title = true,
            "overlap-adjustment" => {
                let value = ConfigOptions::i32_option(&option);
                options.overlap_adjustment = Some(value);
            },
            "jaro-winkler-adjustment" => options.jaro_winkler_adjustment = true,
            "jaro-winkler-author-adjustment" => options.jaro_winkler_author_adjustment = true,
            "json-schema-version" => {
                let value = ConfigOptions::i32_option(&option);
                options.json_schema_version = value;
            },
            "output-source-name" => {
                let value = ConfigOptions::string_option(&option);
                options.output_source_name = value;
            },
            "dataset-dir" => {
                let value = ConfigOptions::string_option(&option);
                options.dataset_dir = value;
            },
            "exclude-file" => { // Repeatable option
                let value = ConfigOptions::string_option(&option);
                options.exclude_files.push(value);
            },
            "input-exclude-file" => { // Repeatable option, similar to exclude-file but for input data only
                let value = ConfigOptions::string_option(&option);
                options.input_exclude_files.push(value);
            },
            _ => {
                eprintln!("Unknown option: {}", option);
                std::process::exit(1);
            }
        }
    }
    populate_excluded_ids(&mut options);
    populate_excluded_input_ids(&mut options);
    options
}

fn parse_command(args: &Args, options: ConfigOptions) -> Config {
    let command = args.command.clone().unwrap_or("match-json-zip".to_string());
    match command.as_str() {
        "build-vocab" => parse_command_build_vocab(args, options),
        "build-dataset-vectors" => parse_command_build_dataset_vectors(args, options),
        "match-json-zip" => parse_command_match_json_zip(args, options),
        "build-source-data" => parse_command_build_source_data(args, options),
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

fn parse_command_build_vocab(args: &Args, options: ConfigOptions) -> Config {
    if args.source.is_none() {
        eprintln!("Source name is required for build-vocab command");
        std::process::exit(1);
    }
    let source = args.source.clone().unwrap();
    let vocab_file = vocab_file_name(args, &options);
    let verbose = args.verbose;
    let config = Config {
        cmd: Cmd::BuildVocab,
        source,
        vocab_file,
        dataset_vector_file: "".to_string(),
        source_data_file: "".to_string(),
        input: "".to_string(),
        output: Output::Stdout,
        output_format: OutputFormat::Text,
        verbose,
        options,
        config_file: args.config_file.clone(),
        default_args: FxHashMap::default(),
    };
    config
}

fn parse_command_build_dataset_vectors(args: &Args, options: ConfigOptions) -> Config {
    if args.source.is_none() {
        eprintln!("Source name is required for build-dataset-vectors command");
        std::process::exit(1);
    }
    let source = args.source.clone().unwrap();
    let vocab_file = vocab_file_name(args, &options);
    let dataset_vector_file = dataset_vector_file_name(args, &options);
    let source_data_file = source_data_file_name(args, &options);
    let verbose = args.verbose;
    let config = Config {
        cmd: Cmd::BuildDatasetVectors,
        source,
        vocab_file,
        dataset_vector_file,
        source_data_file,
        input: "".to_string(),
        output: Output::Stdout,
        output_format: OutputFormat::Text,
        verbose,
        options,
        config_file: args.config_file.clone(),
        default_args: FxHashMap::default(),
    };
    config
}

// match-* requires source and input
// output is stdout unless given a file
// dataset_vector_file and vocab_file are not required,
// but if not given, they default to data/<source>-vocab.bin and data/<source>-dataset-vectors.bin
fn parse_command_match_json_zip(args: &Args, options: ConfigOptions) -> Config {
    if args.source.is_none() {
        eprintln!("Source name is required for match-single-zip command");
        std::process::exit(1);
    }
    if args.input.is_none() {
        eprintln!("Input file is required for match-single-zip command");
        std::process::exit(1);
    }
    let source = args.source.clone().unwrap();
    let input = args.input.clone().unwrap();
    let vocab_file = vocab_file_name(args, &options);
    let dataset_vector_file = dataset_vector_file_name(args, &options);
    let source_data_file = source_data_file_name(args, &options);
    let output = match &args.output {
        Some(filename) => Output::File(filename.clone()),
        None => Output::Stdout,
    };
    let output_format = args.output_format.clone().unwrap_or("xlsx".to_string()).into();
    let verbose = args.verbose;
    let mut config = Config {
        cmd: Cmd::MatchJsonZip,
        source,
        vocab_file,
        dataset_vector_file,
        source_data_file,
        input,
        output,
        output_format,
        verbose,
        options,
        config_file: args.config_file.clone(),
        default_args: FxHashMap::default(),
    };
    add_default_source_data_file(&mut config);
    add_default_vocab_file(&mut config);
    add_default_dataset_vector_file(&mut config);
    config
}

fn parse_command_build_source_data(args: &Args, options: ConfigOptions) -> Config {
    if args.source.is_none() {
        eprintln!("Source name is required for build-source-data command");
        std::process::exit(1);
    }
    let source = args.source.clone().unwrap();
    let source_data_file = source_data_file_name(args, &options);
    let verbose = args.verbose;
    let config = Config {
        cmd: Cmd::BuildSourceData,
        source,
        vocab_file: "".to_string(),
        dataset_vector_file: "".to_string(),
        source_data_file,
        input: "".to_string(),
        output: Output::Stdout,
        output_format: OutputFormat::Text,
        verbose,
        options,
        config_file: args.config_file.clone(),
        default_args: FxHashMap::default(),
    };
    config
}

// If config.source_data_file is equal to the default value, add "source-data-file" to default_args
fn add_default_source_data_file(config: &mut Config) {
    if config.source_data_file == format!("{}/{}-source-data.bin", config.options.dataset_dir, config.options.output_source_name) {
        config.default_args.insert("source-data-file".to_string(), true);
    }
}

fn add_default_vocab_file(config: &mut Config) {
    if config.vocab_file == format!("{}/{}-vocab.bin", config.options.dataset_dir, config.options.output_source_name) {
        config.default_args.insert("vocab-file".to_string(), true);
    }
}

fn add_default_dataset_vector_file(config: &mut Config) {
    if config.dataset_vector_file == format!("{}/{}-dataset-vectors.bin", config.options.dataset_dir, config.options.output_source_name) {
        config.default_args.insert("dataset-vector-file".to_string(), true);
    }
}


// Read excluded ids from one file
fn read_exclude_file(filename: &str) -> Vec<String> {
    let mut excluded_ids = Vec::new();
    match std::fs::read_to_string(filename) {
        Ok(content) => {
            for line in content.lines() {
                let id = line.trim();
                if id.starts_with('#') || id.is_empty() {
                    continue;
                }
                excluded_ids.push(id.to_string());
            }
        },
        Err(e) => {
            eprintln!("Failed to read exclude file {}: {}", filename, e);
            std::process::exit(1);
        }
    }
    excluded_ids
}

// Read all exclude files and populate options.excluded_ids with each line from those files
// Allow "#" for comments and ignore empty lines
fn populate_excluded_ids(options: &mut ConfigOptions) {
    let mut excluded_ids = Vec::new();
    for filename in &options.exclude_files {
        let mut ids = read_exclude_file(filename);
        excluded_ids.append(&mut ids);
    }
    options.excluded_ids = excluded_ids;
}

// Same as populate_excluded_ids, but for input_exclude_files and input_excluded_ids
fn populate_excluded_input_ids(options: &mut ConfigOptions) {
    let mut excluded_ids = Vec::new();
    for filename in &options.input_exclude_files {
        let mut ids = read_exclude_file(filename);
        excluded_ids.append(&mut ids);
    }
    options.input_excluded_ids = excluded_ids;
}

    // let vocab_file = args.vocab_file.clone().unwrap_or(format!("data/{}-vocab.bin", source));
    // let dataset_vector_file = args.dataset_vector_file.clone().unwrap_or(format!("data/{}-dataset-vectors.bin", source));
    // let source_data_file = args.source_data_file.clone().unwrap_or(format!("data/{}-source-data.bin", source));

fn vocab_file_name(args: &Args, options: &ConfigOptions) -> String {
    args.vocab_file.clone().unwrap_or(format!("{}/{}-vocab.bin", options.dataset_dir, options.output_source_name))
}

fn dataset_vector_file_name(args: &Args, options: &ConfigOptions) -> String {
    args.dataset_vector_file.clone().unwrap_or(format!("{}/{}-dataset-vectors.bin", options.dataset_dir, options.output_source_name))
}

fn source_data_file_name(args: &Args, options: &ConfigOptions) -> String {
    args.source_data_file.clone().unwrap_or(format!("{}/{}-source-data.bin", options.dataset_dir, options.output_source_name))
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigFileLoader {
    matching_config: Option<ConfigMatchingConfigLoader>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ConfigMatchingConfigLoader {
    // Just a simple serde Value
    weights: Option<serde_json::Value>,
    options: Option<serde_json::Value>,
}

fn load_options_from_file(filename: &str, options: &mut ConfigOptions) {
    let file = File::open(filename).unwrap_or_else(|e| {
        eprintln!("Failed to open config file {}: {}", filename, e);
        std::process::exit(1);
    });
    let reader = BufReader::new(file);
    let file_options: ConfigFileLoader = 
        match serde_json::from_reader(reader) {
            Ok(opts) => opts,
            Err(_e) => { return; }
        };
    // Overwrite options with those from the file if there is a matching_config section with an options field
    if let Some(matching_config) = file_options.matching_config {
        if let Some(file_opts) = matching_config.options {
            fill_options(options, file_opts);
        }
        // If there is a weights field, write it to a tempfile and set options.weights_file to that filename
        if let Some(weights) = matching_config.weights {
            let temp_dir = std::env::temp_dir();
            let random_number = rand::random::<u32>();
            let weights_file_path = temp_dir.join(format!("matching_weights_temp-{}.json", random_number));
            let weights_file = File::create(&weights_file_path).unwrap_or_else(|e| {
                eprintln!("Failed to create temporary weights file: {}", e);
                std::process::exit(1);
            });
            serde_json::to_writer_pretty(weights_file, &weights).unwrap_or_else(|e| {
                eprintln!("Failed to write weights to temporary file: {}", e);
                std::process::exit(1);
            });
            options.weights_file = Some(weights_file_path.to_str().unwrap().to_string());
        }
    }
}

fn fill_bool(option: &mut bool, option_value: &serde_json::Value) {
    *option = option_value.as_bool().unwrap_or(false)
}

fn fill_optional_i32(option: &mut Option<i32>, option_value: &serde_json::Value) {
    if option_value.is_null() {
        *option = None
    } else {
        *option = Some(option_value.as_i64().unwrap() as i32)
    }
}

fn fill_optional_f32(option: &mut Option<f32>, option_value: &serde_json::Value) {
    if option_value.is_null() {
        *option = None
    } else {
        *option = Some(option_value.as_f64().unwrap() as f32)
    }
}

fn fill_i32(option: &mut i32, option_value: &serde_json::Value) {
    *option = option_value.as_i64().unwrap_or(0) as i32
}

fn fill_f32(option: &mut f32, option_value: &serde_json::Value) {
    *option = option_value.as_f64().unwrap_or(0.0) as f32
}

fn fill_string(option: &mut String, option_value: &serde_json::Value) {
    *option = option_value.as_str().unwrap_or("").to_string()
}

fn fill_option(option_name: &str, option_value: &serde_json::Value, options: &mut ConfigOptions) {
    match option_name {
        "force_year" => fill_bool(&mut options.force_year, option_value),
        "year_tolerance" => fill_optional_i32(&mut options.year_tolerance, option_value),
        "year_tolerance_penalty" => fill_f32(&mut options.year_tolerance_penalty, option_value),
        "include_source_data" => fill_bool(&mut options.include_source_data, option_value),
        "similarity_threshold" => fill_optional_f32(&mut options.similarity_threshold, option_value),
        "z_threshold" => fill_optional_f32(&mut options.z_threshold, option_value),
        "min_single_similarity" => fill_optional_f32(&mut options.min_single_similarity, option_value),
        "min_multiple_similarity" => fill_optional_f32(&mut options.min_multiple_similarity, option_value),
        "extended_output" => fill_bool(&mut options.extended_output, option_value),
        "add_author_to_title" => fill_bool(&mut options.add_author_to_title, option_value),
        "add_serial_to_title" => fill_bool(&mut options.add_serial_to_title, option_value),
        "add_edition_to_title" => fill_bool(&mut options.add_edition_to_title, option_value),
        "overlap_adjustment" => fill_optional_i32(&mut options.overlap_adjustment, option_value),
        "jaro_winkler_adjustment" => fill_bool(&mut options.jaro_winkler_adjustment, option_value),
        "jaro_winkler_author_adjustment" => fill_bool(&mut options.jaro_winkler_author_adjustment, option_value),
        "json_schema_version" => fill_i32(&mut options.json_schema_version, option_value),
        "output_source_name" => fill_string(&mut options.output_source_name, option_value),
        "dataset_dir" => fill_string(&mut options.dataset_dir, option_value),
        _ => {},
    }
}

fn fill_options(options: &mut ConfigOptions, file_opts: serde_json::Value) {
    if let serde_json::Value::Object(map) = file_opts {
        for (key, value) in map {
            fill_option(&key, &value, options);
        }
    }
}