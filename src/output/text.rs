use std::io::Write;
use crate::args::Config;
use crate::output::Output;
use crate::matcher::OutputRecord;
use crate::matcher::TOP_N;

pub fn output_records(config: &Config, records: &[OutputRecord]) {
    let mut writer: Box<dyn Write> = match &config.output {
        Output::Stdout => Box::new(std::io::stdout()),
        Output::File(filename) => {
            let file = std::fs::File::create(&filename).expect("Unable to create file");
            Box::new(std::io::BufWriter::new(file))
        }
    };
    write_text_file(config, &mut writer, records);
}

fn write_text_file(config: &Config, writer: &mut dyn Write, records: &[OutputRecord]) {
    output_header_text(config, writer);
    for record in records {
        output_record_text(config, writer, record);
    }
}

fn output_header_text(_config: &Config, output: &mut dyn Write) {
    let _ = writeln!(output, "Output in text format");
}

fn output_record_text(config: &Config, output: &mut dyn Write, record: &OutputRecord) {
    writeln!(output, "\n\nTop {} matches for record {} {}: {:?}", TOP_N, record.card, record.record.edition, record.record).unwrap();
    for (source_record, similarity, zscore) in &record.top {
        if config.options.include_source_data {
            let _ = writeln!(output, "{}: {}  /  {}  ==>  Title: {}, Author: {}, Location: {}, Year: {}", source_record.id, similarity, zscore, source_record.title, source_record.author, source_record.location, source_record.year);
        } else {
            println!("{}: {}  /  {}", source_record.id, similarity, zscore);
        }
    }
}
