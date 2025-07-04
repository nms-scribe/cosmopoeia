
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::fs::File;

use serde::Serialize;
use serde::Deserialize;
use serde_json::Serializer as JSONSerializer;
use serde_json::from_reader as from_json_reader;
use rand::Rng;
use ordered_float::OrderedFloat;
use schemars::JsonSchema;

use crate::errors::CommandError;
use crate::utils::namers_pretty_print::PrettyFormatter;
use crate::utils::random::RandomIndex;
use crate::algorithms::naming::NamerSet;
use crate::world_map::tile_layer::TileForCulturePrefSorting;
use core::slice::Iter;
use core::ops::Index;
use std::io::Read;
use crate::typed_map::entities::NamedEntity;

#[derive(Clone,Serialize,Deserialize,JsonSchema)]
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
    
    pub(crate) fn get_value(&self, tile: &TileForCulturePrefSorting, max_habitability: f64) -> Result<OrderedFloat<f64>,CommandError> {

        // formulaes borrowed from AFMG
        Ok(match self {
            Self::Habitability => OrderedFloat::from(tile.habitability()),
            Self::ShoreDistance => OrderedFloat::from(tile.shore_distance() as f64),
            Self::Elevation => OrderedFloat::from(tile.elevation_scaled() as f64),
            Self::NormalizedHabitability => OrderedFloat::from((tile.habitability() / max_habitability) * 3.0),
            Self::Temperature(goal) => OrderedFloat::from((tile.temperature() - goal).abs() + 1.0),
            Self::Biomes(preferred_biomes, fee) => OrderedFloat::from(if preferred_biomes.contains(tile.biome().name()) {
                1.0
            } else {
                *fee
            }),
            Self::OceanCoast(fee) => OrderedFloat::from(if tile.water_count().is_some() && tile.neighboring_lake_size().is_none() {
                1.0
            } else {
                *fee
            }),
            Self::Negate(pref) => -pref.get_value(tile, max_habitability)?,
            Self::Multiply(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().ok_or_else(|| CommandError::TilePreferenceMultiplyMissingData)?.get_value(tile, max_habitability)?; 
                for pref in prefs {
                    result *= pref.get_value(tile, max_habitability)?
                }
                result
            },
            Self::Divide(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().ok_or_else(|| CommandError::TilePreferenceDivideMissingData)?.get_value(tile, max_habitability)?; 
                for pref in prefs {
                    result /= pref.get_value(tile, max_habitability)?
                }
                result
            },
            Self::Add(prefs) => {
                let mut prefs = prefs.iter();
                let mut result = prefs.next().ok_or_else(|| CommandError::TilePreferenceAddMissingData)?.get_value(tile, max_habitability)?; 
                for pref in prefs {
                    result += pref.get_value(tile, max_habitability)?
                }
                result
            },
            Self::Pow(pref, pow) => OrderedFloat::from(pref.get_value(tile, max_habitability)?.powf(*pow)),
        })
        
    }

}


// NOTE: The serialization of this and CultureSetItem should be almost the same (except that no fields are optional in CultureSetItem, and count is only on this one)
#[derive(Deserialize,Clone,JsonSchema)]
pub(crate) struct CultureSetItemSource {
    name: Option<String>,
    namer: Option<String>,
    probability: Option<f64>, // in AFMG this was 'odd'
    preferences: Option<TilePreference>, // in AFMG this was 'sort'
    count: Option<usize>
}

// NOTE: The serialization of this and CultureSetItemSource should be almost the same (except that some fields are optional in CultureSetItemSource and it has an optional count)
#[derive(Clone,Serialize)]
pub(crate) struct CultureSetItem {
    name: String,
    namer: String,
    probability: f64, // in AFMG this was 'odd'
    preferences: TilePreference // in AFMG this was 'sort'
}

impl CultureSetItem {

    fn from<Random: Rng>(value: &CultureSetItemSource, rng: &mut Random, namers: &mut NamerSet) -> Vec<Self> {
        let mut result = Vec::new();
        let count = match value.count {
            Some(0) | None => 1,
            Some(c) => c
        };

        for _ in 0..count {
            let namer = match &value.namer {
                Some(namer) => namer.clone(),
                None => {
                    namers.list_names().choose(rng).clone().clone()
                },
            };
    
            let name = if let Some(name) = &value.name {
                name.clone()
            } else {
                let namer = namers.get_mut(Some(&namer)).expect("Why would the key not be here if we just chose this value from amidst its keys?");
                namer.make_name(rng)
            };

            let probability = value.probability.unwrap_or(1.0);
    
            let preferences = match &value.preferences {
                Some(preferences) => preferences.clone(),
                None => TilePreference::Habitability
            };
    
            result.push(Self {
                name,
                namer,
                probability,
                preferences,
            })
        }
        result


    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn namer_name(&self) -> &str {
        &self.namer
    }

    pub(crate) const fn preferences(&self) -> &TilePreference {
        &self.preferences
    }
}



pub(crate) struct CultureSet {
    // NOTE: This is not a map as with namers, one could have multiple cultures with the same name but possibly different other parameters.
    // Such usage would "weight" a culture to increase the probability it will appear, as well as allow it to coexist
    // with other similar cultures under a different name.
    source: Vec<CultureSetItem>
}

impl CultureSet {

    const fn empty() -> Self {
        Self {
            source: Vec::new()
        }
    }

    pub(crate) fn from_files<Random: Rng>(files: &Vec<PathBuf>, rng: &mut Random, namers: &mut NamerSet) -> Result<Self,CommandError> {
        let mut result = Self::empty();

        for file in files {
            result.extend_from_file(file,rng,namers)?;
        }
        Ok(result)
    }



    pub(crate) fn to_json(&self) -> Result<String,CommandError> {

        let mut buf = Vec::new();
        let formatter = PrettyFormatter::with_indent(b"    ");
        let mut ser = JSONSerializer::with_formatter(&mut buf, formatter);
        self.source.serialize(&mut ser).map_err(|e| CommandError::CultureSourceWrite(format!("{e}")))?;
        String::from_utf8(buf).map_err(|e| CommandError::CultureSourceWrite(format!("{e}")))
    
    }

    fn add_culture(&mut self, data: CultureSetItem) {
        self.source.push(data);
    }
    
    pub(crate) fn extend_from_json<Reader: Read, Random: Rng>(&mut self, source: BufReader<Reader>, rng: &mut Random, namers: &mut NamerSet) -> Result<(),CommandError> {
        let data = from_json_reader::<_,Vec<CultureSetItemSource>>(source).map_err(|e| CommandError::CultureSourceRead(format!("{e}")))?;
        for datum in data {
            for item in CultureSetItem::from(&datum,rng,namers) {
                self.add_culture(item)
            }
        }

        Ok(())

        
    }

    pub(crate) fn extend_from_file<AsPath: AsRef<Path>, Random: Rng>(&mut self, file: AsPath, rng: &mut Random, namers: &mut NamerSet) -> Result<(),CommandError> {

        enum Format {
            JSON
        }

        let format = match file.as_ref().extension().and_then(OsStr::to_str) {
            Some("json" | _) | None => Format::JSON, // this is the default, I'm probably not supporting any other formats anyway, but just in case.
        };

        let culture_source = File::open(file).map_err(|e| CommandError::CultureSourceRead(format!("{e}")))?;
        let reader = BufReader::new(culture_source);

        match format {
            Format::JSON => self.extend_from_json(reader,rng,namers),
        }



    }

    pub(crate) const fn len(&self) -> usize {
        self.source.len()
    }

    pub(crate) fn select<Random: Rng>(&self, rng: &mut Random, culture_count: usize) -> Vec<CultureSetItem> {

        // This algorithm taken from AFMG. 

        let mut result = Vec::new();
        let mut available = self.source.clone();
        let mut i = 0;
        while (result.len() < culture_count) && (!available.is_empty()) {
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
impl Index<usize> for CultureSet {
    type Output = CultureSetItem;

    fn index(&self, index: usize) -> &Self::Output {
        &self.source[index]
    }
}

// allow iterating through the culture set.
impl<'data_life> IntoIterator for &'data_life CultureSet {
    type Item = &'data_life CultureSetItem;

    type IntoIter = Iter<'data_life, CultureSetItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.source.iter()
    }
}
