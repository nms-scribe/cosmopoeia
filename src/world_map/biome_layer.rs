use std::collections::HashMap;

use gdal::vector::LayerAccess;
use prisma::Rgb;

use crate::entity;
use crate::errors::CommandError;
use crate::geometry::MultiPolygon;
use crate::layer;
use crate::progress::ProgressObserver;
use crate::world_map::fields::BiomeCriteria;
use crate::typed_map::fields::IdRef;
use crate::typed_map::entities::Entity;
use crate::typed_map::entities::NamedEntity;
use crate::typed_map::features::NamedFeature;
use crate::typed_map::features::TypedFeature;
use crate::typed_map::features::TypedFeatureIterator;

pub(crate) struct BiomeDefault {
    pub(crate) name: &'static str,
    pub(crate) habitability: i32,
    pub(crate) criteria: BiomeCriteria,
    pub(crate) movement_cost: i32,
    pub(crate) supports_nomadic: bool,
    pub(crate) supports_hunting: bool,
    pub(crate) color: (u8,u8,u8),
}

pub(crate) struct BiomeMatrix {
    pub(crate) matrix: [[String; 26]; 5],
    pub(crate) ocean: String,
    pub(crate) glacier: String,
    pub(crate) wetland: String
}

layer!(Biome["biomes"]: MultiPolygon {
    #[set(allow(dead_code))] name: String,
    #[set(allow(dead_code))] habitability: i32,
    #[set(allow(dead_code))] criteria: BiomeCriteria,
    #[set(allow(dead_code))] movement_cost: i32,
    #[set(allow(dead_code))] supports_nomadic: bool,
    #[set(allow(dead_code))] supports_hunting: bool,
    #[set(allow(dead_code))] color: Rgb<u8>,
});

impl Entity<BiomeSchema> for NewBiome {

}

impl TryFrom<BiomeFeature<'_>> for NewBiome {

    type Error = CommandError;

    fn try_from(value: BiomeFeature) -> Result<Self,Self::Error> {
        Ok(Self {
            name: value.name()?,
            habitability: value.habitability()?,
            criteria: value.criteria()?,
            movement_cost: value.movement_cost()?,
            supports_nomadic: value.supports_nomadic()?,
            supports_hunting: value.supports_hunting()?,
            color: value.color()?,
        })
    }
}

impl<'feature> NamedFeature<'feature,BiomeSchema> for BiomeFeature<'feature> {
    fn get_name(&self) -> Result<String,CommandError> {
        self.name()
    }
}

impl BiomeSchema {

    pub(crate) const OCEAN: &'static str = "Ocean";
    pub(crate) const HOT_DESERT: &'static str = "Hot desert";
    pub(crate) const COLD_DESERT: &'static str = "Cold desert";
    pub(crate) const SAVANNA: &'static str = "Savanna";
    pub(crate) const GRASSLAND: &'static str = "Grassland";
    pub(crate) const TROPICAL_SEASONAL_FOREST: &'static str = "Tropical seasonal forest";
    pub(crate) const TEMPERATE_DECIDUOUS_FOREST: &'static str = "Temperate deciduous forest";
    pub(crate) const TROPICAL_RAINFOREST: &'static str = "Tropical rainforest";
    pub(crate) const TEMPERATE_RAINFOREST: &'static str = "Temperate rainforest";
    pub(crate) const TAIGA: &'static str = "Taiga";
    pub(crate) const TUNDRA: &'static str = "Tundra";
    pub(crate) const GLACIER: &'static str = "Glacier";
    pub(crate) const WETLAND: &'static str = "Wetland";

    pub(crate) const DEFAULT_BIOMES: [BiomeDefault; 13] = [ // name, index, habitability, supports_nomadic, supports_hunting
        BiomeDefault { name: Self::OCEAN, habitability: 0, criteria: BiomeCriteria::Ocean, movement_cost: 10, supports_nomadic: false, supports_hunting: false, color: (0x1F, 0x78, 0xB4)},
        BiomeDefault { name: Self::HOT_DESERT, habitability: 4, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 200, supports_nomadic: true, supports_hunting: false, color: (0xFB, 0xE7, 0x9F)},
        BiomeDefault { name: Self::COLD_DESERT, habitability: 10, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 150, supports_nomadic: true, supports_hunting: false, color: (0xB5, 0xB8, 0x87)},
        BiomeDefault { name: Self::SAVANNA, habitability: 22, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 60, supports_nomadic: false, supports_hunting: true, color: (0xD2, 0xD0, 0x82)},
        BiomeDefault { name: Self::GRASSLAND, habitability: 30, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 50, supports_nomadic: true, supports_hunting: false, color: (0xC8, 0xD6, 0x8F)},
        BiomeDefault { name: Self::TROPICAL_SEASONAL_FOREST, habitability: 50, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 70, supports_nomadic: false, supports_hunting: false, color: (0xB6, 0xD9, 0x5D)},
        BiomeDefault { name: Self::TEMPERATE_DECIDUOUS_FOREST, habitability: 100, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 70, supports_nomadic: false, supports_hunting: true, color: (0x29, 0xBC, 0x56)},
        BiomeDefault { name: Self::TROPICAL_RAINFOREST, habitability: 80, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 80, supports_nomadic: false, supports_hunting: false, color: (0x7D, 0xCB, 0x35)},
        BiomeDefault { name: Self::TEMPERATE_RAINFOREST, habitability: 90, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 90, supports_nomadic: false, supports_hunting: true, color: (0x40, 0x9C, 0x43)},
        BiomeDefault { name: Self::TAIGA, habitability: 12, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 200, supports_nomadic: false, supports_hunting: true, color: (0x4B, 0x6B, 0x32)},
        BiomeDefault { name: Self::TUNDRA, habitability: 4, criteria: BiomeCriteria::Matrix(vec![]), movement_cost: 1000, supports_nomadic: false, supports_hunting: true, color: (0x96, 0x78, 0x4B)},
        BiomeDefault { name: Self::GLACIER, habitability: 0, criteria: BiomeCriteria::Glacier, movement_cost: 5000, supports_nomadic: false, supports_hunting: false, color: (0xD5, 0xE7, 0xEB)},
        BiomeDefault { name: Self::WETLAND, habitability: 12, criteria: BiomeCriteria::Wetland, movement_cost: 150, supports_nomadic: false, supports_hunting: true, color: (0x0B, 0x91, 0x31)},
    ];

    //these constants make the default matrix easier to read.
    pub(crate) const HDT: &'static str = Self::HOT_DESERT;
    pub(crate) const CDT: &'static str = Self::COLD_DESERT;
    pub(crate) const SAV: &'static str = Self::SAVANNA;
    pub(crate) const GRA: &'static str = Self::GRASSLAND;
    pub(crate) const TRF: &'static str = Self::TROPICAL_SEASONAL_FOREST;
    pub(crate) const TEF: &'static str = Self::TEMPERATE_DECIDUOUS_FOREST;
    pub(crate) const TRR: &'static str = Self::TROPICAL_RAINFOREST;
    pub(crate) const TER: &'static str = Self::TEMPERATE_RAINFOREST;
    pub(crate) const TAI: &'static str = Self::TAIGA;
    pub(crate) const TUN: &'static str = Self::TUNDRA;

    pub(crate) const DEFAULT_MATRIX: [[&'static str; 26]; 5] = [
        // hot ↔ cold [>19°C; <-4°C]; dry ↕ wet
        [Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::HDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::CDT, Self::TUN],
        [Self::SAV, Self::SAV, Self::SAV, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::GRA, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TEF, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN, Self::TUN],
        [Self::TRR, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TER, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TAI, Self::TUN, Self::TUN]
    ];

    pub(crate) fn get_default_biomes() -> Vec<NewBiome> {
        let mut matrix_criteria = HashMap::new();
        // map the matrix numbers to biome names
        for (moisture,row) in Self::DEFAULT_MATRIX.iter().enumerate() {
            for (temperature,id) in row.iter().enumerate() {
                match matrix_criteria.get_mut(id) {
                    None => {
                        _ = matrix_criteria.insert(id,vec![(moisture,temperature)]);
                    },
                    Some(entry) => entry.push((moisture,temperature)),
                }
            }

        }

        // now insert the matrix numbers into the output biomes criteria fields and return the biome entities.
        Self::DEFAULT_BIOMES.iter().map(|default| {
            let criteria = if let BiomeCriteria::Matrix(_) = default.criteria {
                BiomeCriteria::Matrix(matrix_criteria.get(&default.name).expect("Someone messed up the default biome constants.").clone())
            } else {
                default.criteria.clone()
            };
            NewBiome {
                name: (*default.name).to_owned(),
                habitability: default.habitability,
                criteria,
                movement_cost: default.movement_cost,
                supports_nomadic: default.supports_nomadic,
                supports_hunting: default.supports_hunting,
                color: {
                    let (r,g,b) = default.color;
                    Rgb::new(r,g,b)
                }
            }

        }).collect()

    }

    pub(crate) fn build_matrix_from_biomes(biomes: &[NewBiome]) -> Result<BiomeMatrix,CommandError> {
        let mut matrix: [[String; 26]; 5] = Default::default();
        let mut wetland = None;
        let mut glacier = None;
        let mut ocean = None;
        for biome in biomes {
            match &biome.criteria {
                BiomeCriteria::Matrix(list) => {
                    for (moist,temp) in list {
                        let (moist,temp) = (*moist,*temp);
                        if matrix[moist][temp].is_empty() {
                            matrix[moist][temp].clone_from(&biome.name)

                        } else {
                            return Err(CommandError::DuplicateBiomeMatrixSlot(moist,temp))
                        }
                    }
                },
                BiomeCriteria::Wetland => if wetland.is_some() {
                    return Err(CommandError::DuplicateWetlandBiome)
                } else {
                    wetland = Some(biome.name.clone())
                },
                BiomeCriteria::Glacier => if glacier.is_some() {
                    return Err(CommandError::DuplicateGlacierBiome)
                } else {
                    glacier = Some(biome.name.clone())
                },
                BiomeCriteria::Ocean => if ocean.is_some() {
                    return Err(CommandError::DuplicateOceanBiome)
                } else {
                    ocean = Some(biome.name.clone())
                }
            }

        }
        // check for missing data
        let wetland = wetland.ok_or_else(|| CommandError::MissingWetlandBiome)?;
        let glacier = glacier.ok_or_else(|| CommandError::MissingGlacierBiome)?;
        let ocean = ocean.ok_or_else(|| CommandError::MissingOceanBiome)?;
        for (moisture,moisture_dimension) in matrix.iter().enumerate() {
            for (temperature,temperature_dimension) in moisture_dimension.iter().enumerate() {
                if temperature_dimension.is_empty() {
                    return Err(CommandError::MissingBiomeMatrixSlot(moisture,temperature))
                }
            }
        }
        Ok(BiomeMatrix { 
            matrix, 
            ocean, 
            glacier, 
            wetland 
        })
    }

}

entity!(BiomeForPopulation: Biome {
    name: String,
    habitability: i32
});

impl NamedEntity<BiomeSchema> for BiomeForPopulation {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureGen: Biome {
    name: String,
    supports_nomadic: bool,
    supports_hunting: bool
});

impl NamedEntity<BiomeSchema> for BiomeForCultureGen {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForCultureExpand: Biome {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForCultureExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForNationExpand: Biome {
    name: String,
    movement_cost: i32
});

impl NamedEntity<BiomeSchema> for BiomeForNationExpand {
    fn name(&self) -> &str {
        &self.name
    }
}

entity!(BiomeForDissolve: Biome {
    fid: IdRef,
    name: String
});

impl NamedEntity<BiomeSchema> for BiomeForDissolve {
    fn name(&self) -> &str {
        &self.name
    }
}

impl BiomeLayer<'_,'_> {

    pub(crate) fn add_biome(&mut self, biome: &NewBiome) -> Result<IdRef,CommandError> {
        self.add_struct(biome, None)

    }

    pub(crate) fn get_matrix<Progress: ProgressObserver>(&mut self, progress: &mut Progress) -> Result<BiomeMatrix,CommandError> {
        let result = self.read_features().into_entities_vec(progress)?;

        BiomeSchema::build_matrix_from_biomes(&result)

    }

}
