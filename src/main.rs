mod elastic;
mod tokenizer;
mod vocab;
mod args;
mod cmd;
mod vectorize;
mod matcher;
mod source_data;
mod report;
mod output;
mod zipfile;
mod overlap;

fn main() {
    let config = args::Config::new();
    // Read the source name from the command line arguments
    config.cmd.run(&config);
}
