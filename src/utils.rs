use rand::rngs::StdRng;
use rand::SeedableRng;

pub fn random_number_generator(seed_vec: Vec<u8>) -> StdRng {
    if seed_vec.len() > 0 {
        let mut seeds = [0u8; 32];
        for (&x, p) in seed_vec.iter().zip(seeds.iter_mut()) {
            *p = x;
        }
        StdRng::from_seed(seeds)
    } else {
        StdRng::from_entropy()
    }
}

pub trait RoundHundredths {

    fn round_hundredths(&self) -> Self;
}

impl RoundHundredths for f64 {

    fn round_hundredths(&self) -> Self {
        (self * 100.0).round() / 100.0
    }
}

#[derive(Clone)]
pub struct Size<DataType> {
    pub height: DataType,
    pub width: DataType
}

impl Size<f64> {

    pub fn from_usize(source: Size<usize>) -> Self {
        let width = source.width as f64;
        let height = source.height as f64;
        Self {
            width,
            height
        }
    }
}