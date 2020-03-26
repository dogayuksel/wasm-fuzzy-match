use std::collections::{ BTreeMap, HashSet, HashMap };

use wasm_bindgen::prelude::*;
use js_sys::try_iter;

use fst::{IntoStreamer, Streamer, Set, SetBuilder};
use fst::automaton::Levenshtein;

mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub struct FuzzyMatcherBuilder {
    set_builder: SetBuilder<Vec<u8>>, 
}

impl FuzzyMatcherBuilder {
    pub fn new() -> FuzzyMatcherBuilder {
        let set_builder = SetBuilder::memory();
        FuzzyMatcherBuilder { set_builder }
    }

    pub fn insert(&mut self, key: &str) -> () {
        self.set_builder.insert(key).unwrap();
    }

    pub fn pack(self) -> Set<Vec<u8>> {
        let bytes = self.set_builder.into_inner().unwrap();
        Set::new(bytes).unwrap()
    }
}

pub struct WordIndex {
    collection: BTreeMap::<String, HashSet<usize>>,
}

impl WordIndex {
    pub fn new() -> WordIndex {
        WordIndex {
            collection: BTreeMap::<String, HashSet<usize>>::new(),
        }
    }

    pub fn add_key(&mut self, key: String, index: usize) -> () {
        match self.collection.get_mut(&key) {
            Some(value) => {
                value.insert(index);
                ();
            },
            None => {
                let mut new_set = HashSet::new();
                new_set.insert(index);
                self.collection.insert(key, new_set);
                ()
            }
        }
    }

    pub fn get(&self, key: &str) -> &HashSet<usize> {
        self.collection.get(key).unwrap()
    }
}

#[wasm_bindgen]
pub struct FuzzyMatcher {
    set: Set<Vec<u8>>,    
    word_index: WordIndex,
}

#[wasm_bindgen]
impl FuzzyMatcher {
    #[wasm_bindgen(constructor)]
    pub fn new(js_phrases: &JsValue) -> FuzzyMatcher {
        utils::set_panic_hook();
        let mut word_index = WordIndex::new();
        let iterator = try_iter(js_phrases).unwrap().unwrap();
        for (ind, x) in iterator.enumerate() {
            let phrase = x.unwrap().as_string().unwrap();
            for word in phrase.split_whitespace() {
                word_index.add_key(word.to_string(), ind);
            }
        }
        let mut builder = FuzzyMatcherBuilder::new();
        for key in word_index.collection.keys() {
            builder.insert(key);
        }
        FuzzyMatcher {
            set: builder.pack(),
            word_index,
        }
    }

    pub fn query(&self, keywords: String) -> String {
        let mut count_matches: HashMap<usize, u32> = HashMap::new();
        for keyword in keywords.split_whitespace() {
            for distance in 0..=2 {
                match Levenshtein::new(&keyword, distance) {
                    Ok(lev) => {
                        let mut stream = self.set.search(&lev).into_stream();
                        while let Some(k) = stream.next() {
                            let match_as_string = std::str::from_utf8(k).unwrap();
                            let indexes = self.word_index.get(&match_as_string);
                            let increment = match distance {
                                0 => 7,
                                1 => 3,
                                2 => 1,
                                _ => 0,
                            };
                            for index in indexes {
                                *count_matches.entry(*index).or_insert(0) += increment;
                            }
                        }
                    }
                    Err(_) => { break; }
                };
            }
        }
        let mut count_vec: Vec<(&usize, &u32)> = count_matches.iter().collect();
        count_vec.sort_by(|a, b| b.1.cmp(a.1));
        let mut response = vec!();
        for (index, _) in &count_vec {
            response.push(index.to_string());
        }
        response.join(" ")
    }
}
