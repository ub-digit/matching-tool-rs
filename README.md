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

This will create an Excel file in `output-dir` (the directory is assumed to exist) with the name `outputfile.xlsx` and a markdown file with the name `outputfile-report.md`.

The tool will load the vector data and pre-process that data at the beginning of every execution, so it is preferable to run it with multiple json-files in the zip-file to make the most of the pre-processing.
