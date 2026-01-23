use crate::args::Config;
use crate::vocab;
use crate::vectorize;
use crate::matcher;
use crate::source_data;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum Cmd {
    BuildVocab,
    BuildDatasetVectors,
    MatchJsonZip,
    BuildSourceData,
    DumpSourceData,
}

impl Cmd {
    pub fn run(&self, config: &Config) {
        match &config.cmd {
            Cmd::BuildVocab => vocab::build_vocab(config),
            Cmd::BuildDatasetVectors => vectorize::build_dataset_vectors(config),
            Cmd::MatchJsonZip => matcher::match_json_zip(config),
            Cmd::BuildSourceData => source_data::build_source_data(config),
            Cmd::DumpSourceData => source_data::dump_source_data(config),
        }
    }
}

impl Display for Cmd {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Cmd::BuildVocab => write!(f, "build-vocab"),
            Cmd::BuildDatasetVectors => write!(f, "build-dataset-vectors"),
            Cmd::MatchJsonZip => write!(f, "match-json-zip"),
            Cmd::BuildSourceData => write!(f, "build-source-data"),
            Cmd::DumpSourceData => write!(f, "dump-source-data"),
        }
    }
}
