use rand::Rng;
use random_color::RandomColor;
use random_color::Luminosity;

pub(crate) struct ColorGenerator {
    generator: RandomColor
}

impl ColorGenerator {

    pub(crate) fn new(_base_color: Option<(u8,u8,u8)>) -> Self {
        let mut generator = RandomColor::new();
        // TODO: I want to be able to start with a base color and generate colors around that.
        generator.luminosity = Some(Luminosity::Light);
        
        Self {
            generator
        }

    }

    pub(crate) fn generate<Random: Rng>(&mut self, rng: &mut Random) -> (u8,u8,u8) {
        // TODO: This is kind of odd, generating a seed for a different random number generator to use?
        self.generator.seed = Some(rng.gen_range(0..u64::MAX));
        let [r,g,b] = self.generator.to_rgb_array();
        (r,g,b)
    }
}

/*
FUTURE: In case I ever need this

fn rgb_to_hsv(red: f32, green: f32, blue: f32) -> (f32,f32,f32) {

    // https://math.stackexchange.com/q/556341
    let c_max = red.max(green).max(blue);
    let c_min = red.min(green).min(blue);
    let delta = c_max - c_min;
    let hue = if c_max == red {
        ((green - blue)/delta) % 6.0
    } else if c_max == green {
        ((blue - red)/delta) + 2.0
    } else if c_max == blue {
        ((red - green)/delta) + 4.0
    } else {
        unreachable!("c_max should equal one of the three colors")
    } * 60.0;
    let saturation = if c_max == 0.0 {
        0.0
    } else {
        delta/c_max
    };
    let value = c_max;
    (hue,saturation,value)

}
*/
