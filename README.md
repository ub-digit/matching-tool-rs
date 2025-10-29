# Requirements

* [Install Rust](https://www.rust-lang.org/tools/install)
* Create `data`
* Copy at least the following files to `data`:
    * `libris-vocab.bin` (ca 7MB)
    * `libris-dataset-vectors.bin` (ca 5.2GB)
    * `libris-source-data.bin` (ca 400MB)
* Run `cargo build --release` to make sure it compiles

# Input data
Input data is a ZIP-file with JSON-files in it. Every file with the extension `.json` will be included in the matching process.

The filename should be something that represents the source of the data. For example card number in a system or something like that.

An optional file with the extension `.prompt` can be included, which will represent the prompt that was used to create the data. This file is only part of the output report markdown file. It is not used in the matching process.

## Format
The JSON-files should have the following minimum format:

```
{
    "title": "title of the book" (string),
    "author": "author of the book" (string),
    "editions" [
        {
            "placeOfPublication": "place of publication" (string),
            "yearOfPublication": 1940 (integer),
        },
        {
            "placeOfPublication": "place of publication" (string),
            "yearOfPublication": 1941 (integer),
        }
    ]
}
```

Other data can be included in the JSON-files, but it will not be used in the matching process at this time.

The above example will result in two matching objects, loosely represented like this:

```json
[
    {
        "title": "title of the book",
        "author": "author of the book",
        "placeOfPublication": "place of publication",
        "yearOfPublication": 1940
    },
    {
        "title": "title of the book",
        "author": "author of the book",
        "placeOfPublication": "place of publication",
        "yearOfPublication": 1941
    }
]
```

These two will be matched separately. There will be an "edition" column in the output with the index (0-based) of the edition to separate them.

# Usage

The tool can be used to create the files in `data` from an Elasticsearch index, but this is not covered here for now.

The default operation is to match the data in the ZIP-file with a data source and its files in `data`. The default output will be an Excel file with the matches and a markdown file with a report.

**The following options are required:**

* `-s` or `--source` - the name of the source data in `data` ("libris" in the example above, the rest of the file names will be derived from this).
* `-i` or `--input` - the path to the ZIP-file with the data to match.
* `-o` or `--output` - the path to the output Excel file (the report will be in the same directory with the same name but with the extension `-report.md`).

**The following options are optional, but probably needed:**

* `-O force-year` - force the year to be an exact match in the matching process.
* `-O include-source-data` - include the source data in the output Excel file (shows the Libris title/author/place/year in the output along with the zip file data).
* `-O similarity-threshold=0.35` - the similarity threshold for matching of the vectors (0.35 is an example, goes from 0 (no similarity) to 1 (exact match)). Nothing will be matched if the similarity is below this threshold.
* `-O z-threshold=7` - the Z-score threshold for the matching process (7 is an example). This has no upper limit. The Z-score is a measure of how many standard deviations a data point is from the mean. The higher the Z-score, the more likely it is that the data point is an outlier.
* `-O min-single-similarity=0.5` - the minimum similarity for a single field in the matching process (0.5 is an example). This is used to filter out matches that resulted in only one match, but with a low similarity, making the match less reliable.
* `-v` - verbose output.

The command is run using `cargo run --release --` followed by the options.

**A full command can look like this:**
```
cargo run --release -- -s libris -i /tmp/inputfile.zip -o output-dir/outputfile.xlsx -O force-year -O include-source-data -O similarity-threshold=0.35 -O z-threshold=7 -O min-single-similarity=0.5 -v
```

This will create an Excel file in `output-dir` (the directory will be created if it does not exist) with the name `outputfile.xlsx` and a markdown file with the name `outputfile-report.md`.

The tool will load the vector data and pre-process that data at the beginning of every execution, so it is preferable to run it with multiple json-files in the zip-file to make the most of the pre-processing.

## Full list of options (-O)
* `-O force-year` - force the year to be an exact match in the matching process (see below for fuzzy year matching).
* `-O year-tolerance=1` - allow a tolerance of 1 year (must be 0 or positive integer) when matching the year (only used if `force-year` is set).
* `-O year-tolerance-penalty=0.25` - penalty to apply to the similarity (per year difference) when using `year-tolerance` (only used if `force-year` and `year-tolerance` are set).
* `-O include-source-data` - include the source data in the output Excel file (shows the Libris title/author/place/year in the output along with the zip file data).
* `-O similarity-threshold=0.35` - the minimum similarity threshold for matching of the vectors to be considered a match at all (between 0 and 1).
* `-O z-threshold=7` - the Z-score threshold for the matching process (no upper limit).
* `-O min-single-similarity=0.5` - the minimum similarity for a match to be considered a good single match (between 0 and 1, only relevant if same or higher than `similarity-threshold`).
* `-O min-multiple-similarity=0.5` - the minimum similarity for a match to be considered a useful multiple match (between 0 and 1, only relevant if same or higher than `similarity-threshold`). 
* `-O weights-file=path-to-weights-file` - path to a custom weights file (if not set, default weights are used).
* `-O extended-output` - include extended output in the Excel file (adding separate columns for box, card, cardID and several others used by GUB).
* `-O add-author-to-title` - add the author to the title field in the vector calculation (can improve matching in some cases), simulates a "245c" field, which is now included in the libris-v1_5 weights.
* `-O overlap-adjustment=10` - adjust the score based on large string overlaps (any positive integer, but small values are usually not very useful). This will reduce the similarity score for matches with that lack large overlaps, making matches with similar titles more likely to be singled out as good matches. It also adds the relevance for the order of words in the title. A value of 10 is usually a good starting point.
* `-O jaro-winkler-adjustment` - adjust the score based on Jaro-Winkler similarity of the titles (can improve matching for order of words in titles).
* `-O json-schema-version=2` - specify the JSON schema version of the input data (default is 1, version 2 is a newer format with some changes).
* `-O dataset-dir=path-to-data-directory` - specify the directory where the dataset files are located (default is `data`).
* `-O exclude-file=file1 -O exclude-file=file2` - exclude IDs listed in the specified file (one ID per line). Can be used multiple times to exclude multiple files. Useful for doing a second run excluding IDs that were matched in a first run.
* `-O input-exclude-file=file1 -O input-exclude-file=file2` - exclude IDs listed in the specified file (one ID per line) from the input data. The format of the IDs should be `jsonfilename:edition` (for example `003/12345.json:0` for the first edition in the file `003/12345.json`). Can be used multiple times to exclude multiple files. Useful for doing a second run excluding IDs that were matched in a first run.
