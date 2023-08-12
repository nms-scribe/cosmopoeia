use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::BufReader;

use rand::Rng;
use serde::Serialize;
use serde::Deserialize;
use serde_json;

// TODO: *** I'M NOT GOING TO LOAD THE NAMER STUFF INTO THE DATABASE. The user simply has to get a hold of a namer list.
// TODO: But, what I do want to do is add on another option, below, where the user just supplies a list of words to use.
// - there are new structs:
//   MarkovSource: min_len: usize, cutoff_len: usize,  duplicatable_letters: Vec<char>, seed_words
//   MarkovGenerator: min_len: usize, cutoff_len: usize,  duplicatable_letters: Vec<char>, seed_words, chain,
// - There are two new enums:
//   NameSource:
//     Markov(MarkovSource)
//     List(Vec<String>)
//   NameGenerator
//     Markov(MarkovGenerator)
//     List(Vec<String>,usize) // the list and the next index to choose from
// - the Namers and the NamerSource now use these enums to define their things.
// - make_word is redesigned so that if the namer is a list, it chooses the next one from the list instead... TODO: Or do I want it to choose randomly?
// TODO: Also need a way to load multiple files
// TODO: Also need a way to load just a text file with a list of words, the name of the file is the name of the language
// TODO: I may want to supply a set of namer sources of my own design. Just a little simplified from AFMG.
// TODO: Make sure to ask AFMG about accessing the lists there, I'm not sure what their source is or if they're copyrighted.


    


use crate::utils::ToTitleCase;
use crate::utils::namers_pretty_print::PrettyFormatter;

// This was almost directly ported from AFMG.

fn is_ref_vowel(c: &char) -> bool {
    // FUTURE: Are these *all* the vowels? I guess we're probably just dealing with latin characters, trying to support other character sets might be a bad idea.
    matches!(c,'a'|'e'|'i'|'o'|'u'|'y'|'ɑ'|'\''|'ə'|'ø'|'ɛ'|'œ'|'æ'|'ɶ'|'ɒ'|'ɨ'|'ɪ'|'ɔ'|'ɐ'|'ʊ'|'ɤ'|'ɯ'|'а'|'о'|'и'|'е'|'ё'|'э'|'ы'|'у'|'ю'|'я'|'à'|'è'|'ì'|'ò'|'ù'|'ỳ'|'ẁ'|'ȁ'|'ȅ'|'ȉ'|'ȍ'|'ȕ'|'á'|'é'|'í'|'ó'|'ú'|'ý'|'ẃ'|'ő'|'ű'|'â'|'ê'|'î'|'ô'|'û'|'ŷ'|'ŵ'|'ä'|'ë'|'ï'|'ö'|'ü'|'ÿ'|'ẅ'|'ã'|'ẽ'|'ĩ'|'õ'|'ũ'|'ỹ'|'ą'|'ę'|'į'|'ǫ'|'ų'|'ā'|'ē'|'ī'|'ō'|'ū'|'ȳ'|'ă'|'ĕ'|'ĭ'|'ŏ'|'ŭ'|'ǎ'|'ě'|'ǐ'|'ǒ'|'ǔ'|'ȧ'|'ė'|'ȯ'|'ẏ'|'ẇ'|'ạ'|'ẹ'|'ị'|'ọ'|'ụ'|'ỵ'|'ẉ'|'ḛ'|'ḭ'|'ṵ'|'ṳ')
}

fn is_vowel(c: char) -> bool {
    is_ref_vowel(&c)
}

fn choose<'array, Random: Rng, ItemType>(rng: &mut Random, array: &'array [ItemType]) -> &'array ItemType {
    &array[rng.gen_range(0..array.len())] 
}

#[derive(Clone,Serialize,Deserialize)]
enum StateNameBehavior {
    TrimSuffixes(Vec<String>), // if any of the specified strings appear at the end, get rid of them.
    TrimSuffixesIfLonger(Vec<String>,usize), // if any of the specified strings appear at the end, get rid of them if the word is longer than a specific size
    ForceVowel(String), // if the word does not end with a vowel, add the specified character
    #[allow(dead_code)] ForcePrefix(String),
    ForcePrefixByLetterClass(String, String), // the first is if it's a consonant, the second if it's a vowel
}

impl StateNameBehavior {

    fn trim_suffixes(name: String, suffixes: &Vec<String>) -> String {
        for suffix in suffixes {
            // no, this doesn't keep trimming until they're gone, AFMG didn't either.
            if let Some(name) = name.strip_suffix(suffix) {
                return name.to_owned();
            } 
        }
        name

    }

    fn apply(&self, name: String) -> String {
        match self {
                StateNameBehavior::TrimSuffixes(suffixes) => {
                    Self::trim_suffixes(name, suffixes)
                },
                StateNameBehavior::TrimSuffixesIfLonger(suffixes, len) => {
                    if name.len() > *len {
                        Self::trim_suffixes(name, suffixes)
                    } else {
                        name
                    }
                },
                StateNameBehavior::ForceVowel(suffix) => {
                    if name.ends_with(is_vowel) {
                        name
                    } else {
                        name + suffix
                    }
                },
                StateNameBehavior::ForcePrefix(prefix) => {
                    let mut name = name;
                    name.insert_str(0, prefix);
                    name
                },
                StateNameBehavior::ForcePrefixByLetterClass(cons_prefix,vowel_prefix) => {
                    let mut name = name;
                    if name.starts_with(is_vowel) {
                        name.insert_str(0, vowel_prefix)
                    } else {
                        name.insert_str(0, &cons_prefix)
                    }
                    name
                }
        }

    }
}

#[derive(Clone,Serialize,Deserialize)]
enum StateSuffixBehavior {
    NoSuffix, // do not apply any suffix, not even the default one
    Default,
    Suffix(String), // use the specified suffix instead of default
    ProbableSuffix(f64,String), // if a random number is less than the specified apply the specified suffix
    ProbableSuffixIfShorter(usize,f64,String), // if the word is less than the specified length, and a random number is less than specified, apply the specified suffix
    Choice(Vec<StateSuffixBehavior>), // each choice is tried in turn until one returns a suffix or the end is reached.
}

impl StateSuffixBehavior {

    // I'm using a Result here not for errors, but to indicate a third option to stop the processing and do not return a suffix at all.
    // If an Ok(None) result is returned from the algorithm, a default suffix will be applied. If Err(()) is returned, no suffix
    // will be applied at all.
    fn apply<Random: Rng>(&self, rng: &mut Random, name: &String) -> Result<Option<String>,()> { 
        match self {
                StateSuffixBehavior::NoSuffix => Err(()),
                StateSuffixBehavior::Default => Ok(None), // let the caller apply the default.
                StateSuffixBehavior::Suffix(suffix) => Ok(Some(suffix.to_owned().to_owned())),
                StateSuffixBehavior::ProbableSuffix(prob, suffix) => if rng.gen_bool(*prob) {
                    Ok(Some(suffix.to_owned().to_owned()))
                } else {
                    Ok(None)
                },
                StateSuffixBehavior::ProbableSuffixIfShorter(len, prob, suffix) => if (&name.len() < len) && rng.gen_bool(*prob) {
                    Ok(Some(suffix.to_owned().to_owned()))
                } else {
                    Ok(None)
                },
                StateSuffixBehavior::Choice(list) => {
                    for choice in list {
                        match choice.apply(rng,name) {
                                Ok(Some(suffix)) => return Ok(Some(suffix)),
                                Err(()) => return Err(()),
                                _ => ()
                        }
                    }
                    Ok(None)
                },
        }
    }
}



#[derive(Serialize,Deserialize)] 
struct NamerSource {
    name: String,
    min_len: usize,
    cutoff_len: usize,
    duplicatable_letters: Vec<char>,
    seed_words: Vec<String>,
    state_name: Vec<StateNameBehavior>,
    state_suffix: StateSuffixBehavior,
}


pub(crate) struct Namer {
    chain: HashMap<Option<char>, Vec<String>>,
    min_len: usize,
    cutoff_len: usize,
    duplicatable_letters: Vec<char>,
    state_name: Vec<StateNameBehavior>,
    state_suffix: StateSuffixBehavior,
    seed_words: Vec<String>
}

impl Namer {

    // calculate Markov chain for a namesbase
    fn calculate_chain(array: &Vec<String>) -> HashMap<Option<char>, Vec<std::string::String>> {
        let mut chain = HashMap::new();

        for n in array {
            let name: Vec<char> = n.trim().to_lowercase().chars().collect();
            let basic = name.iter().all(|c| match c {
                '\u{0000}'..='\u{007f}' => true,
                _ => false
            }); // basic chars and English rules can be applied

            // split word into pseudo-syllables
            let mut syllable = String::new();
            let mut i = 0; 
            while i < name.len() {
                let prev_char = if i == 0 { None } else { name.get(i-1).map(|c| *c) }; // pre-onset letter
                let mut vowel_found = false; 

                for c in i..name.len() {
                    let current_char = name[c];
                    let next_char = name.get(c + 1); // next char
                    syllable.push(current_char);
                    if (syllable == " ") || (syllable == "-") { 
                        // syllable starts with space or hyphen
                        break 
                    }; 
                    let next_char = match next_char {
                        Some(' ') | Some('-') | None => break, // definitely the end of a syllable, no need to check.
                        Some(next_char) => *next_char
                    };

                    if is_vowel(current_char) {
                        vowel_found = true
                    }; // check if letter is vowel

                    // do not split some digraphs // FUTURE: NMS: These rules should depend on the language, which should provide a list of diphthongs
                    let is_digraph = if current_char == 'y' && next_char == 'e' {
                        // 'ye' 
                        true
                    } else if basic {
                        // English-like 
                        if (current_char == 'o' && next_char == 'o') || // 'oo'
                           (current_char == 'e' && next_char == 'e') || // 'ee'
                           (current_char == 'a' && next_char == 'e') || // 'ae'
                           (current_char == 'c' && next_char == 'h') { // 'ch'
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    
                    if !is_digraph {
                        if is_vowel(current_char) && next_char == current_char {
                            // two same vowels in a row
                            break
                        }; 
                        if vowel_found && name.get(c + 2).map(is_ref_vowel).unwrap_or_else(|| false) {
                            // syllable has vowel and additional vowel is expected soon
                            break
                        }; 
                    }

                    if syllable.len() >= 5 {
                        // syllable is long enough
                        break;
                    }
                }

                i += syllable.len().min(1); 
                match chain.entry(prev_char) {
                    Entry::Vacant(entry) => {entry.insert(vec![syllable]);},
                    Entry::Occupied(mut entry) => entry.get_mut().push(syllable),
                }
                
                syllable = String::new();
            }
        }

        return chain;
    }

    fn default_state_name_behavior() -> Vec<StateNameBehavior> {
        vec![
            // remove -berg for any // FUTURE:  NMS: This should be language dependent 
            StateNameBehavior::TrimSuffixesIfLonger(vec!["berg".to_owned()], 6),
            // remove -ton for any // FUTURE:  NMS: This should be language dependent
            StateNameBehavior::TrimSuffixesIfLonger(vec!["ton".to_owned()], 5)
        ]
     
    }

    fn new(base: NamerSource) -> Self {
        let mut state_name = Self::default_state_name_behavior();
        state_name.extend(base.state_name.iter().cloned());
        let chain = Self::calculate_chain(&base.seed_words);

        Self {
            chain,
            min_len: base.min_len,
            cutoff_len: base.cutoff_len,
            duplicatable_letters: base.duplicatable_letters,
            state_name,
            state_suffix: base.state_suffix,
            seed_words: base.seed_words
        }
    }

    pub(crate) fn make_word<Random: Rng>(&mut self, rng: &mut Random, min_len: Option<usize>, cutoff_len: Option<usize>) -> String {

        let min_len = min_len.unwrap_or_else(|| self.min_len);
        let cutoff_len = cutoff_len.unwrap_or_else(|| self.cutoff_len);

        let mut choices = self.chain.get(&None).unwrap(); // TODO: NMS: Am I always guaranteed that this one will be filled?
        let mut cur = choose(rng,choices).to_owned();
        let mut word = String::new();
        for _ in 0..20 {
       
            if cur == "" {
                // end of word
                if word.len() < min_len {
                    cur = String::new();
                    word = String::new();
                    choices = self.chain.get(&None).unwrap(); // TODO: NMS: Is this guaranteed?
                } else {
                    break
                }
            } else {
                if (word.len() + cur.len()) > cutoff_len {
                    // word too long
                    if word.len() < min_len {
                        // would be too short, add it anyway
                        word.push_str(&cur)
                    } else if !choices.contains(&"".to_owned()) {
                        // can't end the word with the previous choices, so add the new one anyway.
                        word.push_str(&cur)
                        // although, in theory, I might still be adding on an incorrect ending, but not much else I can do to avoid really long words.
                        // except, maybe, have more data to start with.
                    }
                    break;
                } else {
                    choices = self.chain.get(&cur.chars().last()).unwrap_or_else(|| self.chain.get(&None).unwrap()) // TODO: NMS: Is None guaranteed to return a value?
                };
            }

            word.push_str(&cur);
            cur = choose(rng,choices).to_owned();
        }

        // parse word to get a final name
        // not allow some characters at the end
        let word = word.trim_end_matches(['\'',' ','-']);
        let word: Vec<char> = word.chars().collect();

        let mut name = String::new();
        for (current_index,current_char) in word.iter().enumerate() {
            if Some(current_char) == word.get(current_index + 1) && !self.duplicatable_letters.contains(current_char) {
                // duplication is not allowed except in language-based circumstances
                continue;
            }; 

            let last = name.chars().last();
            if (matches!(last,Some('-')) && current_char == &' ') {
                // remove space after hyphen
                continue;
            }; 

            // TODO: NMS: Why this particular combination? If it happens in the chains, why can't it happen here?
            if current_char == &'a' && matches!(word.get(current_index + 1),Some('e')) {
                // "ae" => "e"
                continue;
            }; 

            if Some(current_char) == word.get(current_index + 1) && 
                 Some(current_char) == word.get(current_index + 2) {
                // remove three same letters in a row
                continue;
            }; 
            name.push(*current_char)
        }
        
        // join the word if any part has only 1 letter
        if name.split(" ").any(|part| part.len() < 2) {
            name = name
                .split(" ")
                .collect();
        }

        if name.len() < 2 {
            name = choose(rng,&self.seed_words).to_owned().to_owned();
        }

        return name;
    }

    pub(crate) fn make_name<Random: Rng>(&mut self, rng: &mut Random, min_len: Option<usize>, cutoff_len: Option<usize>) -> String {
        self.make_word(rng, min_len, cutoff_len).to_title_case()
    }

    pub(crate) fn make_short_name<Random: Rng>(&mut self, rng: &mut Random) -> String {
        let min = self.min_len  - 1;
        let cutoff = (self.cutoff_len - 2).max(min);
        self.make_word(rng, Some(min), Some(cutoff)).to_title_case()
    }

    pub(crate) fn make_state_name<Random: Rng>(&mut self, rng: &mut Random, min: Option<usize>, cutoff: Option<usize>) -> String {
        let mut name = self.make_word(rng, min, cutoff);

        if name.contains(" ") {
            // don't allow multiword state names // TODO: NMS: Why not? There are or were places like "Saudi Arabia", "Papua New Guinea", "Saint Kitts", and all of the caribbean saints, "West Germany" -- In any case, I'm seeing some such names from Vietnamese.
            name = name.replace(' ', "");
        }; 

        for behavior in &self.state_name {
            name = behavior.apply(name);
        }

        name = name.to_title_case();

        let suffixing = &self.state_suffix;

        if let StateSuffixBehavior::NoSuffix = suffixing {
            return name
        }


        // define if suffix should be used // FUTURE: NMS: This should be based on language as well, but I'll leave it for now.
        let suffixed_name = if name.len() > 3 && name.ends_with(is_vowel) {

            let (trimmed_name,ending) = name.split_at(name.len() - 2);
            let ending: Vec<char> = ending.chars().collect();
            let is_penultimate_vowel = is_vowel(ending[0]);

            if is_penultimate_vowel && rng.gen_bool(0.85) {
                // 85% for vv
                // trim off last two vowels before adding the suffix
                trimmed_name.to_owned()
            } else if !is_penultimate_vowel && rng.gen_bool(0.7) {
                // ~60% for cv
                let mut name = trimmed_name.to_owned();
                // trim off the vowel before adding suffix
                name.push(ending[0]);
                name
            } else {
                // no suffix, just return this.
                return name;
            }
        } else if rng.gen_bool(0.6) {
            // 60% for cc and vc
            // so return the name if we're below 40%
            name.clone()
        } else {
            // no suffix, just return this
            return name;
        }; 


        let suffix = match suffixing.apply(rng,&suffixed_name) {
            Ok(Some(suffix)) => suffix,
            Ok(None) => "ia".to_owned(), // standard suffix
            Err(()) => return name, // don't apply a suffix, and return the original name.
        };

        return Self::validate_suffix(suffixed_name, suffix) // TODO: Should be passing suffixed_name
    }
    

    fn validate_suffix(name: String, suffix: String) -> String {
        let mut name = name;
        if name.ends_with(&suffix) {
            // no suffix if name already ends with it
            return name
        }; 
        let s1 = suffix.chars().nth(0).unwrap(); // first letter of suffix

        if name.ends_with(s1) {
            // remove name last letter if it's same as suffix first letter
            name = name.split_at(name.len() - 1).0.to_owned();
        }

        if name.len() > 2 {
            let (beginning,ending) = name.split_at(name.len() - 2);
            let ending: Vec<char> = ending.chars().collect();
    
            if is_vowel(s1) == is_vowel(ending[0]) && is_vowel(s1) == is_vowel(ending[1]) {
                 // remove name last char if 2 last chars are the same type as suffix's 1st
                name = beginning.to_owned();
                name.push(ending[0]);
            }
    
        }

        if name.ends_with(s1) {
            // remove name last letter if it's a suffix first letter (Again)
            name = name.split_at(name.len() - 1).1.to_owned();
        }; 
        return name + &suffix
    }



}

pub(crate) struct NamerSet {
    source: HashMap<String,NamerSource>,
    loaded: HashMap<String,Namer>
}

impl NamerSet {

    pub(crate) fn load(&mut self, name: &str) -> Option<&mut Namer> { // TODO: Should be an error if this one doesn't exist, once we get this hooked up to the database
        if let Entry::Vacant(entry) = self.loaded.entry(name.to_owned()) {
            if let Some(name_base) = self.source.remove(name)  { 
                entry.insert(Namer::new(name_base));
            }

        }
        self.loaded.get_mut(name)

    }

    pub(crate) fn list_languages(&self) -> Vec<String>  {
        self.loaded.keys().chain(self.source.keys()).cloned().collect()
    }

    #[allow(dead_code)] pub(crate) fn to_json(&self) -> Result<String,serde_json::Error> {

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        self.source.serialize(&mut ser)?;
        Ok(format!("{}", String::from_utf8(buf).unwrap()))
    }
    
    pub(crate) fn from_json<Reader: std::io::Read>(source: BufReader<Reader>) -> Result<Self,serde_json::Error> {
        let data = serde_json::from_reader::<_,Vec<NamerSource>>(source)?;
        let mut source = HashMap::new();
        for base in data {
            let name = base.name.clone();
            source.insert(name, base);
        }
        Ok(Self {
            source,
            loaded: HashMap::new()
        })

        
    }
    

}
