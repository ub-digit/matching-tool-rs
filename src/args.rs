use clap::Parser;
use crate::cmd::Cmd;
use crate::output::Output;
use std::fmt::{self, Display, Formatter};
use rustc_hash::FxHashMap;

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
    // Only relevant to reduce command output in report, empty in all other cases.
    pub default_args: FxHashMap<String, bool>,
}

#[derive(Debug)]
pub struct ConfigOptions {
    pub force_year: bool,
    pub include_source_data: bool,
    pub similarity_threshold: Option<f32>,
    pub z_threshold: Option<f32>,
    pub min_single_similarity: Option<f32>,
    pub min_multiple_similarity: Option<f32>,
    pub weights_file: Option<String>,
    pub extended_output: bool,
    pub add_author_to_title: bool,
}

impl ConfigOptions {
    fn f32_option(s: &str) -> f32 {
        s.split('=').collect::<Vec<&str>>()[1].parse::<f32>().unwrap()
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
        include_source_data: false,
        similarity_threshold: None,
        z_threshold: None,
        min_single_similarity: None,
        min_multiple_similarity: None,
        weights_file: None,
        extended_output: false,
        add_author_to_title: false,
    };
    for option in args.options.clone() {
        match ConfigOptions::option_name(&option) {
            "force-year" => options.force_year = true,
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
            _ => {
                eprintln!("Unknown option: {}", option);
                std::process::exit(1);
            }
        }
    }
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
    let vocab_file = args.vocab_file.clone().unwrap_or(format!("data/{}-vocab.bin", source));
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
    let vocab_file = args.vocab_file.clone().unwrap_or(format!("data/{}-vocab.bin", source));
    let dataset_vector_file = args.dataset_vector_file.clone().unwrap_or(format!("data/{}-dataset-vectors.bin", source));
    let source_data_file = args.source_data_file.clone().unwrap_or(format!("data/{}-source-data.bin", source));
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
    let vocab_file = args.vocab_file.clone().unwrap_or(format!("data/{}-vocab.bin", source));
    let dataset_vector_file = args.dataset_vector_file.clone().unwrap_or(format!("data/{}-dataset-vectors.bin", source));
    let source_data_file = args.source_data_file.clone().unwrap_or(format!("data/{}-source-data.bin", source));
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
    let source_data_file = args.source_data_file.clone().unwrap_or(format!("data/{}-source-data.bin", source));
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
        default_args: FxHashMap::default(),
    };
    config
}

// If config.source_data_file is equal to the default value, add "source-data-file" to default_args
fn add_default_source_data_file(config: &mut Config) {
    if config.source_data_file == format!("data/{}-source-data.bin", config.source) {
        config.default_args.insert("source-data-file".to_string(), true);
    }
}

fn add_default_vocab_file(config: &mut Config) {
    if config.vocab_file == format!("data/{}-vocab.bin", config.source) {
        config.default_args.insert("vocab-file".to_string(), true);
    }
}

fn add_default_dataset_vector_file(config: &mut Config) {
    if config.dataset_vector_file == format!("data/{}-dataset-vectors.bin", config.source) {
        config.default_args.insert("dataset-vector-file".to_string(), true);
    }
}