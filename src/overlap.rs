use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct SuffixAutomaton {
    next: Vec<HashMap<char, usize>>, // transitions
    link: Vec<isize>,                 // suffix links
    len: Vec<usize>,                  // max length recognized by state
    last: usize,
}

impl SuffixAutomaton {
    fn new(cap: usize) -> Self {
        let mut sa = SuffixAutomaton {
            next: Vec::with_capacity(2 * cap),
            link: Vec::with_capacity(2 * cap),
            len: Vec::with_capacity(2 * cap),
            last: 0,
        };
        sa.next.push(HashMap::new());
        sa.link.push(-1);
        sa.len.push(0);
        sa
    }

    fn add_char(&mut self, c: char) {
        let cur = self.next.len();
        self.next.push(HashMap::new());
        self.len.push(self.len[self.last] + 1);
        self.link.push(0);

        let mut p = self.last as isize;
        while p != -1 && !self.next[p as usize].contains_key(&c) {
            self.next[p as usize].insert(c, cur);
            p = self.link[p as usize];
        }

        if p == -1 {
            self.link[cur] = 0;
        } else {
            let q = self.next[p as usize][&c];
            if self.len[p as usize] + 1 == self.len[q] {
                self.link[cur] = q as isize;
            } else {
                // clone q
                let clone = self.next.len();
                self.next.push(self.next[q].clone());
                self.len.push(self.len[p as usize] + 1);
                self.link.push(self.link[q]);

                let mut p2 = p;
                while p2 != -1 && self.next[p2 as usize].get(&c) == Some(&q) {
                    self.next[p2 as usize].insert(c, clone);
                    p2 = self.link[p2 as usize];
                }
                self.link[q] = clone as isize;
                self.link[cur] = clone as isize;
            }
        }
        self.last = cur;
    }

    fn build(s: &str) -> Self {
        let mut sa = SuffixAutomaton::new(s.chars().count());
        for ch in s.chars() {
            sa.add_char(ch);
        }
        sa
    }
}

/// Return all *maximal* common substrings of `a` and `b`, sorted by
/// decreasing length (ties broken lexicographically). A substring is
/// "non-redundant" here if it is **not** a substring of any other
/// returned substring.
pub fn maximal_overlaps(a: String, b: String) -> Vec<String> {
    let sa = SuffixAutomaton::build(&a);
    let b_chars: Vec<char> = b.chars().collect();

    // 1) Scan B through the automaton and keep only the longest match
    // ending at each position i.
    let mut v = 0usize;    // current state
    let mut l = 0usize;    // current match length
    let mut candidates: HashSet<String> = HashSet::new();

    for i in 0..b_chars.len() {
        let c = b_chars[i];
        if let Some(&to) = sa.next[v].get(&c) {
            v = to;
            l += 1;
        } else {
            while v != 0 && !sa.next[v].contains_key(&c) {
                v = sa.link[v] as usize;
            }
            if let Some(&to) = sa.next[v].get(&c) {
                l = sa.len[v] + 1;
                v = to;
            } else {
                v = 0;
                l = 0;
            }
        }

        if l > 0 {
            let start = i + 1 - l;
            let s: String = b_chars[start..=i].iter().collect();
            candidates.insert(s);
        }
    }

    // 2) Remove any candidate that is a substring of a longer candidate.
    // Sort by length desc, then lexicographically for determinism.
    let mut list: Vec<String> = candidates.into_iter().collect();
    list.sort_by(|x, y| match y.len().cmp(&x.len()) {
        std::cmp::Ordering::Equal => x.cmp(y),
        other => other,
    });

    let mut filtered: Vec<String> = Vec::new();
    'outer: for s in list.into_iter() {
        for kept in &filtered {
            if kept.contains(&s) {
                // s is redundant (contained in an already-kept longer string)
                continue 'outer;
            }
        }
        filtered.push(s);
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::maximal_overlaps;

    #[test]
    fn swedish_example() {
        let a = "Tal om läkare-vetenskapens grundläggning och tillväxt vid rikets älsta lärosäte i Uppsala".to_string();
        let b = "Tal, om läkare-vetenskapens grundläggning och tilväxt vid rikets älsta [!] lärosäte i Upsala".to_string();

        let out = maximal_overlaps(a, b);

        // The first (longest) overlap should start with the expected span:
        assert!(out[0].starts_with(" om läkare-vetenskapens grundläggning och til"));

        // And "grundläggning" alone should NOT be present since it's inside the longer one
        assert!(!out.iter().any(|s| s == "grundläggning"));
    }
}
