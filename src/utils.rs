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

