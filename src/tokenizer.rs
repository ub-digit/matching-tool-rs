use std::collections::HashMap;

const STARTSYMBOL: char = '\u{0001}'; // vocab_id 1
const ENDSYMBOL: char = '\u{0002}';   // vocab_id 2
pub const UNKNOWN: char = '\u{0003}'; // vocab_id 0

// This will tokenize into 2 and 3-grams
pub fn tokenize_string(string: &str) -> HashMap<String, usize> {
    let string = normalize(string);
    let string = add_surrounding_tokens(&string);
    let mut tokens = HashMap::new();
    tokenize_ngram(&string, 2, &mut tokens);
    tokenize_ngram(&string, 3, &mut tokens);
    tokens
}

// Split the string into n-grams and tokenize each n-gram
fn tokenize_ngram(string: &str, n: usize, tokens: &mut HashMap<String, usize>) {
    // Collect n-grams into a vector. This means there's a running window of n characters to collect
    let ngrams: Vec<String> = string.chars().collect::<Vec<char>>().windows(n).map(|w| w.iter().collect::<String>()).collect();
    for ngram in ngrams {
        // Add or update the token count for the ngram in the tokens hashmap
        let count = tokens.entry(ngram).or_insert(0);
        *count += 1;
    }
}

// Year is special since it is a 4-digit number and it is its own single token
pub fn tokenize_year(year: &str) -> HashMap<String, usize> {
    let mut tokens = HashMap::new();
    // Abort if year is not a 4-digit number
    if year.len() != 4 || !year.chars().all(char::is_numeric) {
        return tokens;
    }
    // At this point we have a single 4-digit number, add or update the token count for the year in the tokens hashmap
    tokens.entry(year.to_string()).and_modify(|count| *count += 1).or_insert(1);
    tokens
}

fn normalize(text: &str) -> String {
    // Downcase text
    let text = text.to_lowercase();
    // Remove punctuation except for - and space
    let text = text.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != ' ', "");
    // Remove all characters above latin-1 range
    let text = text.replace(|c: char| c as u32 > 255, "");
    // Remove all control characters
    let text = text.replace(|c: char| c.is_control(), "");
    // Remove all trailing and leading whitespace
    let text = text.trim().to_string();
    text
  }
  
  // Put a ASCII 1 character before and ASCII 2 character after the text
  fn add_surrounding_tokens(text: &str) -> String {
    let mut text = text.to_string();
    text.insert(0, STARTSYMBOL);
    text.push(ENDSYMBOL);
    text
  }
  