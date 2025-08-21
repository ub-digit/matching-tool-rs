use crate::args::Config;
use crate::matcher::{vector_weights, MatchStatistics, MatchStat};
use crate::output::Output;
use std::io::Write;

// Write a markdown report file with stats used for running the matcher
// If the output is stdout, skip this step.
// Otherwise the report is written to a file with the same name as the output file, 
// but with the suffix -report.md instead of the original extension.
pub fn output_report(config: &Config, stats: &MatchStatistics) {
    // Check if output is stdout, if so, skip this step
    if let Output::Stdout = config.output {
        return;
    }

    // Create filename from output filename with -report.md suffix
    let mut report_filename;
    if let Output::File(filename) = &config.output {
        report_filename = filename.clone();
    } else {
        panic!("Output is not a file");
    }
    // Remove the extension from the filename so that filename.csv or filename.txt becomes filename-report.md
    // It is not certain that there is an extension, so we need to check for that as well
    if let Some(pos) = report_filename.rfind('.') {
        report_filename = report_filename[..pos].to_string();
    }
    report_filename.push_str("-report.md");
    let mut report_file = std::fs::File::create(report_filename).unwrap();
    // Write the report to the file
    let markdown = create_markdown(config, stats);
    report_file.write_all(markdown.as_bytes()).unwrap();
}

// The markdown will contain the following:
// The source used.
// The weights used.
// All the options used.
fn create_markdown(config: &Config, stats: &MatchStatistics) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Report\n\n");
    markdown.push_str("## Data\n\n");
    // Output a table of data values, source, input file, output file, vocab file, vector file
    markdown.push_str(&format!("| {} | {} |\n", "Field", "Value"));
    markdown.push_str("| --- | --- |\n");
    markdown.push_str(&format!("| {} | {} |\n", "source", config.source));
    markdown.push_str(&format!("| {} | {} |\n", "input file", config.input));
    if let Output::File(filename) = &config.output {
        markdown.push_str(&format!("| {} | {} |\n", "output file", filename));
    } else {
        markdown.push_str(&format!("| {} | {} |\n", "output file", "stdout"));
    }
    markdown.push_str(&format!("| {} | {} |\n", "vocab file", config.vocab_file));
    markdown.push_str(&format!("| {} | {} |\n", "vector file", config.dataset_vector_file));
    markdown.push_str(&format!("| {} | {} |\n", "source data file", config.source_data_file));
    markdown.push_str("\n");
    markdown.push_str("## Weights\n\n");
    // Output the weights in a table
    markdown.push_str(&format!("| {} | {} |\n", "Field", "Weight"));
    markdown.push_str("| --- | --- |\n");
    let weights = vector_weights(config);
    markdown.push_str(&format!("| {} | {} |\n", "author", weights.get("author").unwrap()));
    markdown.push_str(&format!("| {} | {} |\n", "title", weights.get("title").unwrap()));
    markdown.push_str(&format!("| {} | {} |\n", "location", weights.get("location").unwrap()));
    markdown.push_str(&format!("| {} | {} |\n", "year", weights.get("year").unwrap()));
    markdown.push_str(&format!("| {} | {} |\n", "all", weights.get("all").unwrap()));
    markdown.push_str("\n");
    markdown.push_str("## Options\n\n");
    // Output the options in a table
    markdown.push_str(&format!("| {} | {} |\n", "Option", "Value"));
    markdown.push_str("| --- | --- |\n");
    markdown.push_str(&format!("| {} | {} |\n", "force_year", config.options.force_year));
    markdown.push_str(&format!("| {} | {} |\n", "include_source_data", config.options.include_source_data));
    markdown.push_str(&format!("| {} | {} |\n", "similarity_threshold", config.options.similarity_threshold.unwrap_or(0.0)));
    markdown.push_str(&format!("| {} | {} |\n", "z_threshold", config.options.z_threshold.unwrap_or(0.0)));
    markdown.push_str(&format!("| {} | {} |\n", "min_single_similarity", config.options.min_single_similarity.unwrap_or(0.0)));
    markdown.push_str(&format!("| {} | {} |\n", "weights_file", config.options.weights_file.as_ref().unwrap_or(&"default weights".to_string())));
    markdown.push_str(&format!("| {} | {} |\n", "extended_output", config.options.extended_output));
    markdown.push_str(&format!("| {} | {} |\n", "add_author_to_title", config.options.add_author_to_title));
    markdown.push_str(&format!("| {} | {} |\n", "overlap_adjustment", config.options.overlap_adjustment.unwrap_or(-1)));
    markdown.push_str(&format!("| {} | {} |\n", "min-multiple_similarity", config.options.min_multiple_similarity.unwrap_or(0.0)));
    markdown.push_str("\n");
    markdown.push_str("## Statistics\n\n");
    // Output the statistics in a table
    markdown.push_str(&format!("| {} | {} |\n", "Field", "Value"));
    markdown.push_str("| --- | --- |\n");
    markdown.push_str(&format!("| {} | {} |\n", "Number of cards", stats.number_of_cards()));
    markdown.push_str(&format!("| {} | {} |\n", "Number of match entities", stats.number_of_records));
    if stats.match_stat(&MatchStat::SingleMatch) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Number of single matches", stats.match_stat(&MatchStat::SingleMatch)));
    }
    if stats.match_stat(&MatchStat::Unqualified) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Number of unqualified single matches", stats.match_stat(&MatchStat::Unqualified)));
    }
    if stats.match_stat(&MatchStat::MultipleMatches) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Number of multiple matches", stats.match_stat(&MatchStat::MultipleMatches)));
    }
    if stats.match_stat(&MatchStat::UnqualifiedMultipleMatches) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Number of unqualified multiple matches", stats.match_stat(&MatchStat::UnqualifiedMultipleMatches)));
    }
    if stats.match_stat(&MatchStat::NoMatch) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Number of no matches", stats.match_stat(&MatchStat::NoMatch)));
    }
    if stats.match_stat(&MatchStat::NoEdition) > 0 {
        markdown.push_str(&format!("| {} | {} |\n", "Cards without editions", stats.match_stat(&MatchStat::NoEdition)));
    }
    if stats.match_stat(&MatchStat::SingleMatch) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "Single match percentage", stats.match_stat_percent(&MatchStat::SingleMatch)));
    }
    if stats.match_stat(&MatchStat::Unqualified) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "Unqualified single match percentage", stats.match_stat_percent(&MatchStat::Unqualified)));
    }
    if stats.match_stat(&MatchStat::MultipleMatches) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "Multiple match percentage", stats.match_stat_percent(&MatchStat::MultipleMatches)));
    }
    if stats.match_stat(&MatchStat::UnqualifiedMultipleMatches) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "Unqualified multiple match percentage", stats.match_stat_percent(&MatchStat::UnqualifiedMultipleMatches)));
    }
    if stats.match_stat(&MatchStat::NoMatch) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "No match percentage", stats.match_stat_percent(&MatchStat::NoMatch)));
    }
    if stats.match_stat(&MatchStat::NoEdition) > 0 {
        markdown.push_str(&format!("| {} | {:.2} |\n", "No edition percentage", stats.match_stat_percent(&MatchStat::NoEdition)));
    }
    cmdline_to_run(&mut markdown, config);
    if stats.prompt_used.len() > 0 {
        prompt_markdown(&mut markdown, &stats.prompt_used);
    }

    markdown
}

fn prompt_markdown(markdown: &mut String, prompt: &str) {
    markdown.push_str("\n");
    markdown.push_str("## Prompt\n\n");
    // Output the prompt as a quote block
    // The prompt can have multiple lines, so we need to split it into lines and add a > in front of each line, including newlines
    for line in prompt.lines() {
        markdown.push_str(&format!("> {}\n", line));
    }
}

// Replicate a cargo run command line from the config
fn cmdline_to_run(markdown: &mut String, config: &Config) {
    let command = format!("-c {}", config.cmd);
    let source = format!("-s {}", config.source);
    let input = format!("-i {}", config.input);
    let output = match &config.output {
        Output::Stdout => "".to_string(),
        Output::File(filename) => format!("-o {}", filename),
    };
    let output_format = format!("-F {}", config.output_format);
    let vocab_file = if config.default_args.contains_key("vocab-file") { "".to_string() } else {format!("-V {}", config.vocab_file) };
    let vector_file = if config.default_args.contains_key("dataset-vector-file") { "".to_string() } else {format!("-D {}", config.dataset_vector_file) };
    let source_data_file = if config.default_args.contains_key("source-data-file") { "".to_string() } else {format!("-S {}", config.source_data_file) };
    let force_year = if config.options.force_year { "-O force-year".to_string() } else { "".to_string() };
    let include_source_data = if config.options.include_source_data { "-O include-source-data".to_string() } else { "".to_string() };
    let similarity_threshold = config.options.similarity_threshold.map_or("".to_string(), |x| format!("-O similarity-threshold={}", x));
    let z_threshold = config.options.z_threshold.map_or("".to_string(), |x| format!("-O z-threshold={}", x));
    let min_single_similarity = config.options.min_single_similarity.map_or("".to_string(), |x| format!("-O min-single-similarity={}", x));
    let min_multiple_similarity = config.options.min_multiple_similarity.map_or("".to_string(), |x| format!("-O min-multiple-similarity={}", x));
    let weights_file = config.options.weights_file.as_ref().map_or("".to_string(), |x| format!("-O weights-file={}", x));
    let extended_output = if config.options.extended_output { "-O extended-output".to_string() } else { "".to_string() };
    let add_author_to_title = if config.options.add_author_to_title { "-O add-author-to-title".to_string() } else { "".to_string() };
    let overlap_adjustment = config.options.overlap_adjustment.map_or("".to_string(), |x| format!("-O overlap-adjustment={}", x));
    let verbose = if config.verbose { "-v".to_string() } else { "".to_string() };
    // Combine them in order above
    let combined_options = vec![command, source, input, output, output_format, vocab_file, vector_file, source_data_file, force_year, include_source_data, similarity_threshold, z_threshold, min_single_similarity, min_multiple_similarity, weights_file, extended_output, add_author_to_title, overlap_adjustment, verbose];
    let options = combined_options.iter().filter(|x| x.len() > 0).map(|x| x.to_string()).collect::<Vec<String>>().join(" ");
    let cmdline = format!("cargo run --release -- {}", options);
    markdown.push_str("\n");
    markdown.push_str("## Command line\n\n");
    markdown.push_str(&format!("```\n{}\n```\n", cmdline));
}

// #[derive(Debug)]
// pub struct ConfigOptions {
//     pub force_year: bool,
//     pub include_source_data: bool,
//     pub similarity_threshold: Option<f32>,
//     pub z_threshold: Option<f32>,
//     pub min_single_similarity: Option<f32>,
//     pub weights_file: Option<String>,
// }