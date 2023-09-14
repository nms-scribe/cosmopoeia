
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::fs::File;

use serde::Serialize;
use serde::Deserialize;
use rand::Rng;
use ordered_float::OrderedFloat;

use crate::errors::CommandError;
use crate::utils::namers_pretty_print::PrettyFormatter;
use crate::utils::RandomIndex;
use crate::algorithms::naming::NamerSet;
use crate::progress::ProgressObserver;
use crate::world_map::TileForCulturePrefSorting;

#[derive(Clone,Serialize,Deserialize)]
pub(crate) enum TilePreference {
    // s[i] -> Habitability
    Habitability, 
    // t[i] -> ShoreDistance
    ShoreDistance,
    // h[i] -> Elevation,
    Elevation,
    // NormalizedHabitability -> NormalizedHabitability
    NormalizedHabitability,
    // td(i, (\d+)) -> Temperature($1)
    Temperature(f64), // preferred temperature
    // bd(i, *\[([^\]]+)]) -> Biomes([$1],4)
    // bd(i, *\[([^\]]+)], *(\d+))) -> Biomes([$1],$2)
    // FUTURE: Unfortunately, this requires the culture sets to be associated with a specific biome set. I may want to revisit this someday.
    Biomes(Vec<String>, f64), // list of biomes, fee for wrong biome 
    // sf(i) => OceanCoast(4)
    // sf(i, *(\d+)) => OceanCoast($1)
    OceanCoast(f64), // fee for not being on ocean
    Negate(Box<TilePreference>),
    Multiply(Vec<TilePreference>),
    Divide(Vec<TilePreference>),
    Add(Vec<TilePreference>),
    Pow(Box<TilePreference>,f64)
}


impl TilePreference {
    
    pub(crate) fn get_value(&self, tile: &TileForCulturePrefSorting, max_habitability: f64) -> OrderedFloat<f64> {

        // formulaes borrowed from AFMG
        match self {
            TilePreference::Habitability => OrderedFloat::from(tile.habitability),
            TilePreference::ShoreDistance => OrderedFloat::from(tile.shore_distance as f64),
            TilePreference::Elevation => OrderedFloat::from(tile.elevation_scaled as f64),
            TilePreference::NormalizedHabitability => OrderedFloat::from((tile.habitability / max_habitability) * 3.0),
            TilePreference::Temperature(goal) => OrderedFloat::from((tile.temperature - goal).abs() + 1.0),
            TilePreference::Biomes(preferred_biomes, fee) => OrderedFloat::from(if preferred_biomes.contains(&tile.biome.name) {
                1.0
            } else {
                *fee
            }),
            TilePreference::OceanCoast(fee) => OrderedFloat::from(if tile.water_count.is_some() && tile.neighboring_lake_size.is_none() {
                1.0
            } else {
                *fee
            }),
            TilePreference::Negate(pref) => -pref.get_value(tile, max_habitability),
            TilePreference::Multiply(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().unwrap().get_value(tile, max_habitability); 
                for pref in prefs {
                    result *= pref.get_value(tile, max_habitability)
                }
                result
            },
            TilePreference::Divide(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().unwrap().get_value(tile, max_habitability); 
                for pref in prefs {
                    result /= pref.get_value(tile, max_habitability)
                }
                result
            },
            TilePreference::Add(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().unwrap().get_value(tile, max_habitability); 
                for pref in prefs {
                    result += pref.get_value(tile, max_habitability)
                }
                result
            },
            TilePreference::Pow(pref, pow) => OrderedFloat::from(pref.get_value(tile, max_habitability).powf(*pow)),
        }
        
    }

}


#[derive(Serialize,Deserialize,Clone)]
pub(crate) struct CultureSource {
    name: String,
    namer: String,
    probability: f64, // in AFMG this was 'odd'
    preferences: TilePreference // in AFMG this was 'sort'
}

impl CultureSource {

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn namer_name(&self) -> &str {
        &self.namer
    }

    pub(crate) fn preferences(&self) -> &TilePreference {
        &self.preferences
    }
}

pub(crate) struct CultureSet {
    // NOTE: This is not a map as with namers, one could have multiple cultures with the same name but possibly different other parameters.
    // Such usage would "weight" a culture to increase the probability it will appear, as well as allow it to coexist
    // with other similar cultures under a different name.
    source: Vec<CultureSource>
}

impl CultureSet {

    fn empty() -> Self {
        Self {
            source: Vec::new()
        }
    }

    pub(crate) fn from_files(files: Vec<PathBuf>) -> Result<Self,CommandError> {
        let mut result = Self::empty();

        for file in files {
            result.extend_from_file(file)?;
        }
        Ok(result)
    }



    pub(crate) fn to_json(&self) -> Result<String,CommandError> {

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
        self.source.serialize(&mut ser).map_err(|e| CommandError::CultureSourceWrite(format!("{}",e)))?;
        Ok(String::from_utf8(buf).map_err(|e| CommandError::CultureSourceWrite(format!("{}",e)))?)
    
    }

    fn add_culture(&mut self, data: CultureSource) {
        self.source.push(data);
    }
    
    pub(crate) fn extend_from_json<Reader: std::io::Read>(&mut self, source: BufReader<Reader>) -> Result<(),CommandError> {
        let data = serde_json::from_reader::<_,Vec<CultureSource>>(source).map_err(|e| CommandError::CultureSourceRead(format!("{}",e)))?;
        for data in data {
            self.add_culture(data)
        }

        Ok(())

        
    }

    pub(crate) fn extend_from_file<AsPath: AsRef<Path>>(&mut self, file: AsPath) -> Result<(),CommandError> {

        enum Format {
            JSON
        }

        let format = match file.as_ref().extension().and_then(OsStr::to_str) {
            Some("json") => Format::JSON,
            Some(_) | None => Format::JSON, // this is the default, I'm probably not supporting any other formats anyway, but just in case.
        };

        let culture_source = File::open(file).map_err(|e| CommandError::CultureSourceRead(format!("{}",e)))?;
        let reader = BufReader::new(culture_source);

        match format {
            Format::JSON => self.extend_from_json(reader),
        }



    }

    pub fn len(&self) -> usize {
        self.source.len()
    }

    #[allow(dead_code)] pub(crate) fn make_random_culture_set<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, namers: NamerSet, progress: &mut Progress, count: usize) -> Result<Self,CommandError> {

        let namer_keys = namers.list_names();
        let default = namer_keys.choose(rng).to_owned();
        let mut loaded_namers = namers.into_loaded(&namer_keys, default, progress)?;
        let mut result = Self::empty();
        for _ in 0..count {
            let namer_key = namer_keys.choose(rng);
            let namer = loaded_namers.get_mut(Some(namer_key)).unwrap(); // It should be here.

            result.add_culture(CultureSource {
                name: namer.make_name(rng),
                namer: namer_key.clone(),
                probability: 1.0,
                preferences: TilePreference::Habitability,
            });

        }
        Ok(result)
        
    }

    #[allow(dead_code)] pub(crate) fn make_random_culture_set_with_same_namer<Random: Rng, Progress: ProgressObserver>(rng: &mut Random, namers: &mut NamerSet, namer_key: &str, progress: &mut Progress, count: usize) -> Result<Self,CommandError> {

        let mut namer = namers.load_one(namer_key, progress)?;
        
        let mut result = Self::empty();
        for _ in 0..count {
            result.add_culture(CultureSource {
                name: namer.make_name(rng),
                namer: namer_key.to_owned(),
                probability: 1.0,
                preferences: TilePreference::Habitability,
            });

        }
        Ok(result)
        
    }

    pub(crate) fn select<Random: Rng>(&self, rng: &mut Random, culture_count: usize) -> Vec<CultureSource> {

        // This algorithm taken from AFMG. 

        let mut result = Vec::new();
        let mut available = self.source.clone();
        let mut i = 0;
        while (result.len() < culture_count) && (available.len() > 0) {
            let choice = loop {
                i += 1;
                let choice = available.choose_index(rng);
                if (i >= 200) || rng.gen_bool(available[choice].probability) {
                    break choice;
                }    
            };
            result.push(available.remove(choice));
        }

        result

    }

}

// allow indexing the culture set by usize.
impl std::ops::Index<usize> for CultureSet {
    type Output = CultureSource;

    fn index(&self, index: usize) -> &Self::Output {
        &self.source[index]
    }
}

// allow iterating through the culture set.
impl<'data_life> IntoIterator for &'data_life CultureSet {
    type Item = &'data_life CultureSource;

    type IntoIter = std::slice::Iter<'data_life, CultureSource>;

    fn into_iter(self) -> Self::IntoIter {
        self.source.iter()
    }
}
