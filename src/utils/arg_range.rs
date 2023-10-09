use core::fmt::Display;
use core::str::FromStr;

use serde::Serialize;
use serde::Deserialize;
use rand::Rng;
use rand_distr::uniform::SampleUniform;

use crate::errors::CommandError;


#[derive(Clone)]
pub enum ArgRange<NumberType> {
    // While I could use a real Range<> and RangeInclusive<>, I'd have to copy it every time I want to generate a number from it anyway, and
    Inclusive(NumberType,NumberType),
    Exclusive(NumberType,NumberType),
    Single(NumberType)
}

pub trait TruncOrSelf {

    fn trunc_or_self(self) -> Self;
}

macro_rules! impl_trunc_or_self_float {
    ($float: ty) => {
        impl TruncOrSelf for $float {
            fn trunc_or_self(self) -> Self {
                self.trunc()
            }
        }
            
    };
}

macro_rules! impl_trunc_or_self_int {
    ($int: ty) => {
        impl TruncOrSelf for $int {
            fn trunc_or_self(self) -> Self {
                self
            }
        }
            
    };
}

impl_trunc_or_self_float!(f64);

impl_trunc_or_self_float!(f32);

impl_trunc_or_self_int!(usize);

impl_trunc_or_self_int!(i8);

impl_trunc_or_self_int!(i16);

impl_trunc_or_self_int!(i32);

impl_trunc_or_self_int!(i64);

impl_trunc_or_self_int!(i128);

impl_trunc_or_self_int!(u8);

impl_trunc_or_self_int!(u16);

impl_trunc_or_self_int!(u32);

impl_trunc_or_self_int!(u64);

impl_trunc_or_self_int!(u128);

impl<NumberType: SampleUniform + PartialOrd + Copy + TruncOrSelf> ArgRange<NumberType> {

    pub(crate) fn choose<Random: Rng>(&self, rng: &mut Random) -> NumberType {
        match self  {
            Self::Inclusive(min,max) => rng.gen_range(*min..=*max),
            Self::Exclusive(min,max) => rng.gen_range(*min..*max),
            Self::Single(value) => *value,
        }
    }

    pub(crate) fn includes(&self, value: &NumberType) -> bool {
        match self {
            Self::Inclusive(min, max) => (value >= min) && (value <= max),
            Self::Exclusive(min, max) => (value >= min) && (value < max),
            Self::Single(single) => single.trunc_or_self() == single.trunc_or_self(),
        }
    }
}

impl<'deserializer,NumberType: FromStr + PartialOrd + Deserialize<'deserializer>> Deserialize<'deserializer> for ArgRange<NumberType>
where <NumberType as FromStr>::Err: Display {

    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'deserializer> {

        // https://stackoverflow.com/q/56582722/300213
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum StrOrNum<NumberType> {
            Str(String),
            Num(NumberType)
        }

        let value = StrOrNum::deserialize(deserializer)?;
        match value {
            StrOrNum::Str(deserialized) => deserialized.parse().map_err(|e: CommandError| serde::de::Error::custom(e.to_string())),
            StrOrNum::Num(deserialized) => Ok(Self::Single(deserialized)),
        }
    
    }
}

impl<NumberType: FromStr + Display> Serialize for ArgRange<NumberType> {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

impl<NumberType: FromStr + PartialOrd> FromStr for ArgRange<NumberType> 
where <NumberType as FromStr>::Err: Display {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((first,mut last)) = s.split_once("..") {
            let include_last = if last.starts_with('=') {
                last = last.trim_start_matches('=');
                true
            } else {
                false
            };

            let first = first.parse().map_err(|e| CommandError::InvalidRangeArgument(s.to_owned(),format!("{e}")))?;
            let last = last.parse().map_err(|e| CommandError::InvalidRangeArgument(s.to_owned(),format!("{e}")))?;
            if first > last {
                return Err(CommandError::InvalidRangeArgument(s.to_owned(),"First number must be less than last.".to_owned()))
            }

            Ok(if include_last {
                Self::Inclusive(first,last)
            } else {
                Self::Exclusive(first,last)
            })
        } else {
            let number = s.parse().map_err(|e| CommandError::InvalidRangeArgument(s.to_owned(),format!("{e}")))?;
            Ok(Self::Single(number))
        }
    }
}

impl<NumberType: FromStr + Display> Display for ArgRange<NumberType> {

    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Inclusive(min,max) => write!(f,"{min}..={max}"),
            Self::Exclusive(min,max) => write!(f,"{min}..{max}"),
            Self::Single(single) => write!(f,"{single}"),
        }
    }
}
