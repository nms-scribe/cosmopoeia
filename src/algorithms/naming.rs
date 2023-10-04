use std::collections::HashMap;
use std::collections::HashSet;
use std::io::BufReader;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::fs::File;

use rand::Rng;
use rand_distr::Normal;
use rand_distr::Distribution;
use serde::Serialize;
use serde::Deserialize;
use serde_json::Serializer as JSONSerializer;
use serde_json::from_reader as from_json_reader;

// NOTE: *** I'M NOT GOING TO LOAD THE NAMER OR CULTURE STUFF INTO THE DATABASE. Instead, I hope to provide some default data in the `share` directory.


//use crate::utils::ToTitleCase;
use crate::utils::namers_pretty_print::PrettyFormatter;
use crate::utils::split_string_from_end;
use crate::utils::random::RandomIndex;
use crate::errors::CommandError;
use crate::progress::ProgressObserver;
use crate::commands::NamerArg;

struct NamerLoadObserver<'data,Progress: ProgressObserver> {
    name: &'data str,
    progress: &'data mut Progress,
    visible: bool
}

impl<'data,Progress: ProgressObserver>  NamerLoadObserver<'data,Progress> {

    fn new(name: &'data str, progress: &'data mut Progress) -> Self {
        Self {
            name,
            progress,
            visible: false
        }
    }

    fn start_known_endpoint<Callback: FnOnce() -> usize>(&mut self, callback: Callback) {
        let count = callback();
        self.visible = count > 10000; 
        if self.visible {
            self.progress.start_known_endpoint(|| (format!("Preparing names for {}",self.name),count))
        }
    }

    fn update<Callback: FnOnce() -> usize>(&self, callback: Callback) {
        if self.visible {
            self.progress.update(callback)
        }
    }

    fn finish(&mut self) {
        if self.visible {
            self.progress.finish(|| format!("Names prepared for {}",self.name))
        }
    }
}

// This was almost directly ported from AFMG.

#[allow(clippy::trivially_copy_pass_by_ref)] // this one is used in a call to 'map' that passes references
const fn is_ref_vowel(c: &char) -> bool {
    // FUTURE: Are these *all* the vowels? I guess we're probably just dealing with latin characters, and more specifically characters that at least some English speaker might think of as vowels, trying to support other character sets and languages would be better done by offering a vowel list option in the naming sources.
    matches!(c,'a'|'e'|'i'|'o'|'u'|'y'|'ɑ'|'\''|'ə'|'ø'|'ɛ'|'œ'|'æ'|'ɶ'|'ɒ'|'ɨ'|'ɪ'|'ɔ'|'ɐ'|'ʊ'|'ɤ'|'ɯ'|'а'|'о'|'и'|'е'|'ё'|'э'|'ы'|'у'|'ю'|'я'|'à'|'è'|'ì'|'ò'|'ù'|'ỳ'|'ẁ'|'ȁ'|'ȅ'|'ȉ'|'ȍ'|'ȕ'|'á'|'é'|'í'|'ó'|'ú'|'ý'|'ẃ'|'ő'|'ű'|'â'|'ê'|'î'|'ô'|'û'|'ŷ'|'ŵ'|'ä'|'ë'|'ï'|'ö'|'ü'|'ÿ'|'ẅ'|'ã'|'ẽ'|'ĩ'|'õ'|'ũ'|'ỹ'|'ą'|'ę'|'į'|'ǫ'|'ų'|'ā'|'ē'|'ī'|'ō'|'ū'|'ȳ'|'ă'|'ĕ'|'ĭ'|'ŏ'|'ŭ'|'ǎ'|'ě'|'ǐ'|'ǒ'|'ǔ'|'ȧ'|'ė'|'ȯ'|'ẏ'|'ẇ'|'ạ'|'ẹ'|'ị'|'ọ'|'ụ'|'ỵ'|'ẉ'|'ḛ'|'ḭ'|'ṵ'|'ṳ')
}

const fn is_vowel(c: char) -> bool {
    is_ref_vowel(&c)
}

#[derive(Clone,Serialize,Deserialize)]
enum StateNameBehavior {
    TrimSuffixes(Vec<String>), // if any of the specified strings appear at the end, get rid of them.
    TrimSuffixesIfLonger(Vec<String>,usize), // if any of the specified strings appear at the end, get rid of them if the word is longer than a specific size
    ForceVowel(String), // if the word does not end with a vowel, add the specified character
    ForcePrefix(String),
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
            Self::TrimSuffixes(suffixes) => {
                Self::trim_suffixes(name, suffixes)
            },
            Self::TrimSuffixesIfLonger(suffixes, len) => {
                if name.len() > *len {
                    Self::trim_suffixes(name, suffixes)
                } else {
                    name
                }
            },
            Self::ForceVowel(suffix) => {
                if name.ends_with(is_vowel) {
                    name
                } else {
                    name + suffix
                }
            },
            Self::ForcePrefix(prefix) => {
                let mut name = name;
                name.insert_str(0, prefix);
                name
            },
            Self::ForcePrefixByLetterClass(cons_prefix,vowel_prefix) => {
                let mut name = name;
                if name.starts_with(is_vowel) {
                    name.insert_str(0, vowel_prefix)
                } else {
                    name.insert_str(0, cons_prefix)
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
            Self::NoSuffix => Err(()),
            Self::Default => Ok(None), // let the caller apply the default.
            Self::Suffix(suffix) => Ok(Some(suffix.clone())),
            Self::ProbableSuffix(prob, suffix) => if rng.gen_bool(*prob) {
                Ok(Some(suffix.clone()))
            } else {
                Ok(None)
            },
            Self::ProbableSuffixIfShorter(len, prob, suffix) => if (&name.len() < len) && rng.gen_bool(*prob) {
                Ok(Some(suffix.clone()))
            } else {
                Ok(None)
            },
            Self::Choice(list) => {
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
struct MarkovSource {
    duplicatable_letters: Vec<char>,
    seed_words: Vec<String>,
}


#[derive(Serialize,Deserialize)]
enum NamerMethodSource {
    Markov(MarkovSource),
    ListPicker(Vec<String>)
}


#[derive(Serialize,Deserialize)] 
struct NamerSource {
    name: String,
    method: NamerMethodSource,
    state_name: Vec<StateNameBehavior>,
    state_suffix: StateSuffixBehavior,
}

struct MarkovGenerator {
    chain: HashMap<Option<char>, Vec<String>>,
    length_distribution: Normal<f64>,
    minimum_length: usize,
    duplicatable_letters: Vec<char>,
    seed_words: Vec<String>,

}

struct Chain {
    chain: HashMap<Option<char>, Vec<String>>,
    length_distribution: Normal<f64>,
    minimum_length: usize,
}

impl MarkovGenerator {

    // calculate Markov chain for a namesbase
    fn calculate_chain<Progress: ProgressObserver>(name: &str, array: &Vec<String>, progress: &mut NamerLoadObserver<Progress>) -> Result<Chain,CommandError> {
        if array.is_empty() {
            Err(CommandError::EmptyNamerInput(name.to_owned()))
        } else {
            let mut chain = HashMap::new();

            let name_count = array.len() as f64;
            let mean_length = array.iter().map(String::len).sum::<usize>() as f64/name_count;
            let mut squared_distance = 0.0;
            let mut minimum_length = usize::MAX;

            progress.start_known_endpoint(|| array.len());
    
            for (j,n) in array.iter().enumerate() {
                // for standard deviation calculation
                squared_distance += (n.len() as f64 - mean_length).powi(2);
                minimum_length = minimum_length.min(n.len());

                let word: Vec<char> = n.trim().chars().collect();
                let basic = word.iter().all(|c| matches!(c, '\u{0000}'..='\u{007f}')); // basic chars and English rules can be applied

    
                // split word into pseudo-syllables
                let mut syllable = String::new();
                let mut i = 0; 
                while i < word.len() {
                    let prev_char = if i == 0 { 
                        None 
                    } else { 
                        word.get(i-1).copied() 
                    }; // pre-onset letter
                    let mut vowel_found = false; 
    
                    for c in i..word.len() {
                        let current_char = word[c];
                        let next_char = word.get(c + 1); // next char
                        syllable.push(current_char);
                        if (syllable == " ") || (syllable == "-") { 
                            // syllable starts with space or hyphen
                            break 
                        }; 
                        let next_char = match next_char {
                            Some(' ' | '-') | None => break, // definitely the end of a syllable, no need to check.
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
                            if vowel_found && word.get(c + 2).map_or_else(|| false, is_ref_vowel) {
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
                    match chain.get_mut(&prev_char) {
                        None => {
                            _ = chain.insert(prev_char,vec![syllable]);
                        },
                        Some(entry) => entry.push(syllable),
                    }
                    
                    syllable = String::new();
                }
                progress.update(|| j);
            }

            let standard_deviation = (squared_distance/name_count).sqrt();
    
            progress.finish();

            let length_distribution = Normal::new(mean_length, standard_deviation).map_err(|e| CommandError::NamerDistributionError(name.to_owned(),format!("{e}")))?;

            if chain.is_empty() {
                Err(CommandError::EmptyNamerInput(name.to_owned()))
            } else {
                Ok(Chain {
                    chain,
                    length_distribution,
                    minimum_length
                })
            }
    
        }

    }


    fn new<Progress: ProgressObserver>(name: &str, base: MarkovSource, progress: &mut NamerLoadObserver<Progress>) -> Result<Self,CommandError> {
        let Chain{chain,length_distribution,minimum_length} = Self::calculate_chain(name,&base.seed_words,progress)?;
        

        Ok(Self {
            chain,
            length_distribution,
            minimum_length,
            duplicatable_letters: base.duplicatable_letters,
            seed_words: base.seed_words
        })
    }

    pub(crate) fn make_word<Random: Rng>(&mut self, rng: &mut Random) -> String {

        let min_len = self.minimum_length;
        let cutoff_len = self.length_distribution.sample(rng).ceil() as usize;

        let mut choices = self.chain.get(&None).expect("How would we get an empty chain?"); // As long as the input wasn't empty, this shouldn't panic
        let mut cur = choices.choose(rng).clone();
        let mut word = String::new();
        for _ in 0..20 {
       
            if cur.is_empty() {
                // end of word
                if word.len() < min_len {
                    cur = String::new();
                    word = String::new();
                    choices = self.chain.get(&None).expect("How would we get an empty chain?"); // As long as the input wasn't empty, this shouldn't panic.
                } else {
                    break
                }
            } else if (word.len() + cur.len()) > cutoff_len {
                // word too long
                if (word.len() < min_len) || !choices.contains(&String::new()) {
                    // either 1) it would be too short
                    // or 2) can't end the word with the previous choices
                    // so add it anyway.
                    word.push_str(&cur)
                    // although, in theory for case 2, I might still be adding on an incorrect ending, but not much else I can do to avoid really long words.
                    // except, maybe, have more data to start with.
                }
                break;
            } else {
                choices = self.chain.get(&cur.chars().last()).unwrap_or_else(|| self.chain.get(&None).expect("How would we get an empty chain?")) // As long as the original input wasn't empty, this shouldn't panic
            }

            word.push_str(&cur);
            cur = choices.choose(rng).clone();
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
            // NOTE: AFMG was capitalizing letters after space and hyphen, however if the seed words are curated correctly,
            // it should be following the capitalization rules already, right? If we're going to do something like this, though,
            // it would have to be customizable by language, and we'd have to be able to specify "short words" not capitalizable.
            // I really feel like this is way beyond the scope.

            // FUTURE: NMS: Why this particular combination? If it happens in the chains, why can't it happen here? Again, should be language specific.
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
        if name.split(' ').any(|part| part.len() < 2) {
            name = name
                .split(' ')
                .collect();
        }

        if name.len() < 2 {
            name = self.seed_words.choose(rng).clone();
        }

        name
    }


}

struct ListPicker {
    available: Vec<String>,
    picked: Vec<String>
}

impl ListPicker {

    fn new(name: &str, list: Vec<String>) -> Result<Self,CommandError> {
        if list.is_empty() {
            Err(CommandError::EmptyNamerInput(name.to_owned()))
        } else {
            Ok(Self {
                available: list,
                picked: Vec::new()
            })    
        }
    }

    fn pick_word<Random: Rng>(&mut self, rng: &mut Random) -> String {
        if self.available.is_empty() {
            self.available = core::mem::replace(&mut self.picked, Vec::new())
        }

        let picked = self.available.remove(rng.gen_range(0..self.available.len()));
        self.picked.push(picked.clone());
        picked
    }
}

enum NamerMethod {
    Markov(MarkovGenerator),
    ListPicker(ListPicker)
}

impl NamerMethod {

    pub(crate) fn make_word<Random: Rng>(&mut self, rng: &mut Random) -> String {
        match self {
            Self::Markov(markov) => markov.make_word(rng),
            Self::ListPicker(picker) => picker.pick_word(rng)
        }
    }

    fn new<Progress: ProgressObserver>(name: &str, method: NamerMethodSource, progress: &mut NamerLoadObserver<Progress>) -> Result<Self,CommandError> {
        Ok(match method {
            NamerMethodSource::Markov(markov) => Self::Markov(MarkovGenerator::new(name,markov,progress)?),
            NamerMethodSource::ListPicker(list) => Self::ListPicker(ListPicker::new(name,list)?)
        })
    }

}

pub(crate) struct Namer {
    method: NamerMethod,
    state_name: Vec<StateNameBehavior>,
    state_suffix: StateSuffixBehavior
}

impl Namer {


    fn default_state_name_behavior() -> Vec<StateNameBehavior> {
        vec![
            // remove -berg for any // FUTURE:  NMS: This should be language dependent 
            StateNameBehavior::TrimSuffixesIfLonger(vec!["berg".to_owned()], 6),
            // remove -ton for any // FUTURE:  NMS: This should be language dependent
            StateNameBehavior::TrimSuffixesIfLonger(vec!["ton".to_owned()], 5)
        ]
     
    }

    fn new<Progress: ProgressObserver>(base: NamerSource, progress: &mut NamerLoadObserver<Progress>) -> Result<Self,CommandError> {
        let mut state_name = Self::default_state_name_behavior();
        state_name.extend(base.state_name.iter().cloned());
        let method = NamerMethod::new(&base.name,base.method,progress)?;

        Ok(Self {
            method,
            state_name,
            state_suffix: base.state_suffix
        })
    }

    pub(crate) fn make_word<Random: Rng>(&mut self, rng: &mut Random) -> String {

        self.method.make_word(rng)
    }

    pub(crate) fn make_name<Random: Rng>(&mut self, rng: &mut Random) -> String {
        self.make_word(rng)//.to_title_case()
    }

    pub(crate) fn make_state_name<Random: Rng>(&mut self, rng: &mut Random) -> String {
        let mut name = self.make_word(rng);

        /*
        // NOTE: NMS: This was from the AFMG code. However, why not? There are or were places like "Saudi Arabia", "Papua New Guinea", "Saint Kitts", and all of the caribbean saints, "West Germany" -- In any case, I'm seeing a lot of such names from some languages.
        if name.contains(" ") {
            // don't allow multiword state names 
            name = name.replace(' ', "");
        }; 
        */

        for behavior in &self.state_name {
            name = behavior.apply(name);
        }

        let suffixing = &self.state_suffix;

        if matches!(suffixing, StateSuffixBehavior::NoSuffix) {
            return name
        }


        // define if suffix should be used // FUTURE: NMS: This should be based on language as well, but I'll leave it for now.
        let suffixed_name = if name.len() > 3 && name.ends_with(is_vowel) {

            let (trimmed_name,ending) = split_string_from_end(&name, 2);
            let ending: Vec<char> = ending.chars().collect();
            let is_penultimate_vowel = is_vowel(ending[0]);

            if is_penultimate_vowel && rng.gen_bool(0.85) {
                // 85% for vv
                // trim off last two vowels before adding the suffix
                trimmed_name.to_owned()
            } else if !is_penultimate_vowel && rng.gen_bool(0.7) {
                // ~60% for cv
                let mut trimmed_name = trimmed_name.to_owned();
                // trim off the vowel before adding suffix
                trimmed_name.push(ending[0]);
                trimmed_name
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

        Self::validate_suffix(suffixed_name, &suffix) 
    }
    

    fn validate_suffix(name: String, suffix: &str) -> String {
        let mut name = name;
        if name.ends_with(&suffix) {
            // no suffix if name already ends with it
            return name
        }; 
        let s1 = suffix.chars().nth(0).expect("Whoever called this function shouldn't have passed a blank suffix."); // first letter of suffix

        if name.ends_with(s1) {
            // remove name last letter if it's same as suffix first letter
            name = split_string_from_end(&name, 1).0.to_owned();
        }

        let (beginning,ending) = split_string_from_end(&name, 2);
        if ending.len() > 1 {
            let ending: Vec<char> = ending.chars().collect();
    
            if is_vowel(s1) == is_vowel(ending[0]) && is_vowel(s1) == is_vowel(ending[1]) {
                 // remove name last char if 2 last chars are the same type as suffix's 1st
                name = beginning.to_owned();
                name.push(ending[0]);
            }
    
        }

        if name.ends_with(s1) {
            // remove name last letter if it's a suffix first letter (Again)
            name = split_string_from_end(&name, 1).0.to_owned();
        }; 
        name + suffix
    }



}

pub(crate) struct NamerSet {
    default_namer: String,
    map: HashMap<String,Namer>
}

impl NamerSet {

    pub(crate) fn get_mut(&mut self, name: Option<&str>) -> Result<&mut Namer,CommandError> {
        #[allow(clippy::unnecessary_lazy_evaluations)] // It's calling a function, so it's not unecessary
        let name = name.unwrap_or_else(|| self.default_namer.as_str());
        self.map.get_mut(name).ok_or_else(|| CommandError::UnknownNamer(name.to_owned()))
    }

    pub(crate) fn list_names(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    pub(crate) fn check_exists(&self, namer: &str) -> Result<(),CommandError> {
        if self.map.contains_key(namer) {
            Ok(())
        } else {
            Err(CommandError::UnknownNamer(namer.to_owned()))
        }
    }

    pub(crate) fn load_from<Random: Rng, Progress: ProgressObserver>(args: NamerArg, rng: &mut Random, progress: &mut Progress) -> Result<Self, CommandError> {
        let source = NamerSetSource::from_files(args.namers)?;

        let mut map = HashMap::new();

        for (name,name_base) in source.source {
            let namer = Namer::new(name_base,&mut NamerLoadObserver::new(&name,progress))?;
            _ = map.insert(name, namer);
        }
        
        let default_namer = if let Some(default_namer) = args.default_namer {
            if !map.contains_key(&default_namer) {
                return Err(CommandError::UnknownNamer(default_namer))
            }
            default_namer    
        } else {
            let keys: Vec<&String> = map.keys().collect();
            let result = keys.choose(rng).to_owned().clone();
            progress.message(|| format!("Using default namer '{result}'"));
            result
        };

        
        Ok(Self {
            default_namer,
            map
        })
    }

}

pub(crate) struct NamerSetSource {
    source: HashMap<String,NamerSource>
}

impl NamerSetSource {

    fn empty() -> Self {
        Self {
            source: HashMap::new()
        }
    }

    pub(crate) fn from_files(files: Vec<PathBuf>) -> Result<Self,CommandError> {
        let mut result = Self::empty();

        for file in files {
            result.extend_from_file(file,false)?;
        }
        Ok(result)
    }


    pub(crate) fn to_json(&self) -> Result<String,CommandError> {

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = JSONSerializer::with_formatter(&mut buf, formatter);
        // I don't want to serialize a map, I want to serialize it as an array.
        let data = self.source.values().collect::<Vec<_>>();
        data.serialize(&mut ser).map_err(|e| CommandError::NamerSourceWrite(format!("{e}")))?;
        String::from_utf8(buf).map_err(|e| CommandError::NamerSourceWrite(format!("{e}")))
    

    }

    fn add_namer(&mut self, data: NamerSource) {
        let name = data.name.clone();
        // if the name already exists, then we're replacing the existing one.
        _ = self.source.insert(name, data);
    }
    
    pub(crate) fn extend_from_json<Reader: std::io::Read>(&mut self, source: BufReader<Reader>) -> Result<(),CommandError> {
        let data = from_json_reader::<_,Vec<NamerSource>>(source).map_err(|e| CommandError::NamerSourceRead(format!("{e}")))?;
        for datum in data {
            self.add_namer(datum)
        }

        Ok(())

        
    }

    pub(crate) fn extend_from_text<Reader: std::io::Read>(&mut self, name: String, text_is_markov: bool, source: BufReader<Reader>) -> Result<(),CommandError> {
        let mut list = Vec::new();
        let mut duplicate_chars = HashSet::new();
        for line in source.lines() {
            let word = line.map_err(|e| CommandError::NamerSourceRead(format!("{e}")))?; 
            for (a,b) in word.chars().zip(word.chars().skip(1)) {
                if a == b {
                    _ = duplicate_chars.insert(a);
                }
            }

            
            list.push(word)
        }

        if text_is_markov {

            self.add_namer(NamerSource {
                name,
                method: NamerMethodSource::Markov(MarkovSource {
                    duplicatable_letters: duplicate_chars.into_iter().collect(),
                    seed_words: list,
                }),
                state_name: Vec::new(),
                state_suffix: StateSuffixBehavior::NoSuffix,
            });
    
        } else {
            self.add_namer(NamerSource {
                name,
                method: NamerMethodSource::ListPicker(list),
                state_name: Vec::new(),
                state_suffix: StateSuffixBehavior::Default,
            });
    
        }

        Ok(())

        
    }

    pub(crate) fn extend_from_file<AsPath: AsRef<Path>>(&mut self, file: AsPath, text_is_markov: bool) -> Result<(),CommandError> {

        enum Format {
            JSON,
            TextList(String)
        }

        let format = match file.as_ref().extension().and_then(OsStr::to_str) {
            Some("txt") => Format::TextList(file.as_ref().file_stem().and_then(OsStr::to_str).unwrap_or("").to_owned()),
            Some("json" | _) | None => Format::JSON, // this is the default, although perhaps the 'txt' should be the default?
        };

        let namer_source = File::open(file).map_err(|e| CommandError::NamerSourceRead(format!("{e}")))?;
        let reader = BufReader::new(namer_source);

        match format {
            Format::JSON => self.extend_from_json(reader),
            Format::TextList(name) => self.extend_from_text(name, text_is_markov, reader),
        }



    }

}
