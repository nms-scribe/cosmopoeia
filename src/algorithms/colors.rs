use rand::Rng;
use prisma::Rgb;
use prisma::Hsv;
use prisma::FromColor;
use angular_units::Deg;
use angular_units::Angle;



// Based on randomColor by David Merfield under the CC0 license
// https://github.com/davidmerfield/randomColor/
// Although there was a rust implementation, it was insufficient as
// it did not provide as many customization features.


pub(crate) enum NamedColor {
    Red,
    Orange,
    Yellow, 
    Green,
    Blue,
    Purple,
    Pink
}

pub(crate) struct ColorInformation {
    hue_range: (Deg<f32>,Deg<f32>), // negative values are allowed to make it simpler to keep red within one range.
    lower_bounds: &'static [(f32,f32)],
    saturation_range: (f32,f32),
    //brightness_range: (u8,u8),
}

impl ColorInformation {

    const MONOCHROME: ColorInformation = ColorInformation::new((Deg(0.0),Deg(0.0)), &[(0.0,0.0)]);

    const DEFAULT: ColorInformation = ColorInformation::new((Deg(0.0),Deg(360.0)), &[(0.0,0.0),(1.0,0.0)]);

    pub(crate) const fn new(hue_range: (Deg<f32>,Deg<f32>), lower_bounds: &'static[(f32,f32)]) -> Self {
        let s_min = lower_bounds[0].0;
        let s_max = lower_bounds[lower_bounds.len() - 1].0;
        //let b_min = lower_bounds[lower_bounds.len() - 1].1;
        //let b_max = lower_bounds[0].1;

        Self {
            hue_range,
            lower_bounds: &lower_bounds,
            saturation_range: (s_min, s_max),
            //brightness_range: (b_min, b_max)
        }

    }

    fn find_color_info_for_hue(hue: &Deg<f32>) -> ColorInformation {
        // Maps red colors to make picking hue easier
        let hue = hue.normalize();
        let hue = if hue >= Deg(334.0) {
            hue - Deg(360.0)
        } else {
            hue
        };

        for entry in NAMED_COLOR_DICTIONARY {
            if hue >= entry.1.hue_range.0 && 
                hue <= entry.1.hue_range.1 {
                return entry.1
            }
        }
        unreachable!("Color info not defined for {}",hue)

    }


}

impl NamedColor {


    pub(crate) const fn get_color_information(&self) -> ColorInformation {
        match self {
            Self::Red => ColorInformation::new(
                (Deg(-26.0), Deg(18.0)),
                &[
                    (0.20, 1.00),
                    (0.30, 0.92),
                    (0.40, 0.89),
                    (0.50, 0.85),
                    (0.60, 0.78),
                    (0.70, 0.70),
                    (0.80, 0.60),
                    (0.90, 0.55),
                    (1.00, 0.50),
                ]
            ),            
            Self::Orange => ColorInformation::new(
                (Deg(18.0), Deg(46.0)),
                &[
                    (0.20, 1.00),
                    (0.30, 0.93),
                    (0.40, 0.88),
                    (0.50, 0.86),
                    (0.60, 0.85),
                    (0.70, 0.70),
                    (1.00, 0.70),
                ]
            ),
            Self::Yellow => ColorInformation::new(
                (Deg(46.0), Deg(62.0)),
                &[
                    (0.25, 1.00),
                    (0.40, 0.94),
                    (0.50, 0.89),
                    (0.60, 0.86),
                    (0.70, 0.84),
                    (0.80, 0.82),
                    (0.90, 0.80),
                    (1.00, 0.75),
                ]
            ),
            Self::Green => ColorInformation::new(
                (Deg(62.0), Deg(178.0)),
                &[
                    (0.30, 1.00),
                    (0.40, 0.90),
                    (0.50, 0.85),
                    (0.60, 0.81),
                    (0.70, 0.74),
                    (0.80, 0.64),
                    (0.90, 0.50),
                    (1.00, 0.40),
                ]
            ),
            Self::Blue => ColorInformation::new(
                (Deg(178.0), Deg(257.0)),
                &[
                    (0.20, 1.00),
                    (0.30, 0.86),
                    (0.40, 0.80),
                    (0.50, 0.74),
                    (0.60, 0.60),
                    (0.70, 0.52),
                    (0.80, 0.44),
                    (0.90, 0.39),
                    (1.00, 0.35),
                ]
            ),
            Self::Purple => ColorInformation::new(
                (Deg(257.0), Deg(282.0)),
                &[
                    (0.20, 1.00),
                    (0.30, 0.87),
                    (0.40, 0.79),
                    (0.50, 0.70),
                    (0.60, 0.65),
                    (0.70, 0.59),
                    (0.80, 0.52),
                    (0.90, 0.45),
                    (1.00, 0.42),
                ]
            ),
            Self::Pink => ColorInformation::new(
                (Deg(282.0), Deg(334.0)),
                &[
                    (0.20, 1.00),
                    (0.30, 0.90),
                    (0.40, 0.86),
                    (0.60, 0.84),
                    (0.80, 0.80),
                    (0.90, 0.75),
                    (1.00, 0.73),
                ]
            )
        }
    }
}


pub(crate) const NAMED_COLOR_DICTIONARY: [(NamedColor,ColorInformation); 7] = [
    (NamedColor::Red,NamedColor::Red.get_color_information()), 
    (NamedColor::Orange,NamedColor::Orange.get_color_information()), 
    (NamedColor::Yellow,NamedColor::Yellow.get_color_information()),
    (NamedColor::Green,NamedColor::Green.get_color_information()),
    (NamedColor::Blue,NamedColor::Blue.get_color_information()),
    (NamedColor::Purple,NamedColor::Purple.get_color_information()),
    (NamedColor::Pink,NamedColor::Pink.get_color_information())
];


#[allow(variant_size_differences)] // Not much I can do to bring the size differences closer
pub(crate) enum ColorSet {
    Hue(Deg<f32>),
    HueRange(Deg<f32>,Deg<f32>),
    #[allow(dead_code)] Named(NamedColor),
    #[allow(dead_code)] Monochrome
}

impl ColorSet {

    fn get_hue_range(color_input: &Option<ColorSet>) -> (Deg<f32>,Deg<f32>) {
        match color_input {
            Some(ColorSet::Hue(hue)) => {
                (*hue,*hue)
            },
            Some(ColorSet::HueRange(min, max)) => {
                (*min,*max)
            },
            Some(ColorSet::Monochrome) => ColorInformation::MONOCHROME.hue_range,
            Some(ColorSet::Named(named)) => named.get_color_information().hue_range,
            None => ColorInformation::DEFAULT.hue_range,
        }
    }

    fn get_color_info_for_hue(color_set: &Option<ColorSet>, hue: &Deg<f32>) -> ColorInformation {
        match color_set {
            Some(ColorSet::Hue(hue)) => {
                ColorInformation::find_color_info_for_hue(hue)
            },
            Some(ColorSet::HueRange(_,_)) => {
                ColorInformation::find_color_info_for_hue(hue)
            },
            Some(ColorSet::Monochrome) => ColorInformation::MONOCHROME,
            Some(ColorSet::Named(_)) => {
                ColorInformation::find_color_info_for_hue(hue)
            },
            None => {
                ColorInformation::find_color_info_for_hue(hue)
            },
        }
    }



}

pub(crate) enum Luminosity {
    #[allow(dead_code)] Bright,
    #[allow(dead_code)] Dark,
    Light,
    #[allow(dead_code)] Value(f32),
    #[allow(dead_code)] Saturation(f32),
    #[allow(dead_code)] SaturationValue(f32,f32)
}

pub(crate) enum ColorSpreadAxis {
    Hue,
    #[allow(dead_code)] Saturation,
    #[allow(dead_code)] Value
}

pub(crate) struct RandomColorGenerator {
    color_set: Option<ColorSet>,
    luminosity: Option<Luminosity>,
    color_spread_axis: ColorSpreadAxis,
}

impl RandomColorGenerator {

    pub(crate) fn new(color_set: Option<ColorSet>, luminosity: Option<Luminosity>) -> Self {
        Self {
            color_set,
            luminosity,
            color_spread_axis: ColorSpreadAxis::Hue
        }

    }

    // specify a color generator that generates along the saturation axis based on the hue of a given color
    #[allow(dead_code)] pub(crate) fn from_rgb(rgb: &Rgb<u8>, luminosity: Option<Luminosity>) -> Self {
        let hsv = Hsv::from_color(&rgb.color_cast());
        let mut result = Self::new(Some(ColorSet::Hue(hsv.hue())),luminosity);
        result.set_spread_axis(ColorSpreadAxis::Saturation);
        result
    }

    // This is a weird but useful one. Let's say you have a previously generated set of colors. Now, you want to generate additional colors
    // based on one of them, but varying in hue within a range around that color. For example, you have a region you now want
    // to divide into smaller regions with similar colors so they are still separated from the subregions of other regions.
    // So, use `split_hue_range_for_color_set` with the same main region count to get the same hue ranges used for generating the original,
    // and then pass the main region color. The code finds the range within that set of ranges which the original color came from, and
    // uses that to generate colors.
    pub(crate) fn from_rgb_in_split_hue_range(rgb: &Rgb<u8>, ranges: &Vec<(Deg<f32>,Deg<f32>)>, luminosity: Option<Luminosity>) -> Self {
        let hsv = Hsv::from_color(&rgb.color_cast());
        let hue = hsv.hue();
        let mut chosen_range = None;
        // find the range it's supposed to be in
        for range in ranges {
            if (range.0 <= hue) && (range.1 > hue) {
                chosen_range = Some(range);
                break;
            }
        }
        if let Some((min,max)) = chosen_range {
            Self::new(Some(ColorSet::HueRange(*min,*max)),luminosity)
        } else {
            Self::new(Some(ColorSet::Hue(hue)),luminosity)
        }
        
    }

    pub(crate) fn set_spread_axis(&mut self, axis: ColorSpreadAxis) {
        self.color_spread_axis = axis
    }

    // See `from_rgb_in_split_hue_range`
    pub(crate) fn split_hue_range_for_color_set(color_set: Option<ColorSet>, count: usize) -> Vec<(Deg<f32>,Deg<f32>)> {
        let hue_range = ColorSet::get_hue_range(&color_set);
        Self::split_hue_range(hue_range, count)
    }

    fn split_hue_range(range: (Deg<f32>,Deg<f32>), count: usize) -> Vec<(Deg<f32>,Deg<f32>)> {
        Self::split_range(range, count, |d| d.scalar(), Deg::new)
    }

    fn split_float_range(range: (f32,f32), count: usize) -> Vec<(f32,f32)> {
        Self::split_range(range, count, From::from, From::from)
    }

    fn split_range<ResultType, MapFrom: Fn(ResultType) -> f32, MapTo: Fn(f32) -> ResultType>(range: (ResultType,ResultType), count: usize, map_from: MapFrom, map_to: MapTo) -> Vec<(ResultType,ResultType)> {
        let min = map_from(range.0);
        let max = map_from(range.1);
        let step = (max - min) / count as f32;
        (0..count).map(|n| {
            let n = n as f32;
            let min = min + n * step;
            let max = min + step;
            (map_to(min),map_to(max))
        }).collect()
    }

    pub(crate) fn generate_colors<Random: Rng>(&self, count: usize, rng: &mut Random) -> Vec<Rgb<u8>> {

        let mut colors = Vec::new();

        // The original code used a different function that turned a specific hue into a range of 0..360,
        // but I don't want that to happen.
        let hue_range = ColorSet::get_hue_range(&self.color_set);

        let split_hue_range = if let ColorSpreadAxis::Hue = self.color_spread_axis {
            Self::split_hue_range(hue_range, count)
        } else {
            vec![hue_range]
        };

        for hue_range in split_hue_range {
            
            let hue = self.pick_hue(&hue_range, rng);

            let color_info = ColorSet::get_color_info_for_hue(&self.color_set,&hue);

            let saturation_range = self.get_saturation_range(&color_info);

            let split_saturation_range = if let ColorSpreadAxis::Saturation = self.color_spread_axis {
                Self::split_float_range(saturation_range, count)
            } else {
                vec![saturation_range]
            };

            for saturation_range in split_saturation_range {

                let saturation = self.pick_saturation(&saturation_range, rng);

                let value_range = self.get_value_range(saturation, &color_info);

                let split_value_range = if let ColorSpreadAxis::Value = self.color_spread_axis {
                    Self::split_float_range(value_range, count)
                } else {
                    vec![value_range]
                };

                for value_range in split_value_range {

                    let value = self.pick_value(&value_range, rng);
    
                    let hsv = Hsv::new(hue,saturation,value);
        
                    let color = Rgb::from_color(&hsv).color_cast();
        
        
                    colors.push(color)
    
                }
    
            }

        }

        // sort randomly so they aren't output in rainbow order
        colors.sort_by_key(|_| rng.gen::<usize>());

        return colors;
                
    }

    fn pick_hue<Random: Rng>(&self, hue_range: &(Deg<f32>,Deg<f32>), rng: &mut Random) -> Deg<f32> /* 0..=360 */ {

        let hue = Deg(rng.gen_range(hue_range.0.0..=hue_range.1.0));

        // Instead of storing red as two seperate ranges,
        // we group them, using negative numbers
        hue.normalize()
    }

    fn pick_saturation<Random: Rng>(&self, range: &(f32,f32), rng: &mut Random) -> f32 /* 0..=1 */ {
        rng.gen_range(range.0..=range.1)
    
    }

    fn get_saturation_range(&self, color_info: &ColorInformation) -> (f32, f32) {
        let (s_min,s_max) = if let Some(luminosity) = &self.luminosity {
            let saturation_range = color_info.saturation_range;
    
            let (s_min,s_max) = match luminosity {
                Luminosity::Bright => (0.55,saturation_range.1),
                Luminosity::Dark => (saturation_range.0 - 0.10, saturation_range.1),
                Luminosity::Light => (saturation_range.0, 0.55),
                Luminosity::Value(_) => saturation_range,
                Luminosity::Saturation(saturation) | Luminosity::SaturationValue(saturation,_) => (*saturation,*saturation),
        
            };

            (s_min,s_max)

        } else {
            (0.0,1.00)
        };
        (s_min, s_max)
    }

    fn pick_value<Random: Rng>(&self, range: &(f32, f32), rng: &mut Random) -> f32 /* 0..=1 */ {
    
        rng.gen_range(range.0..=range.1)
    }

    fn get_value_range(&self, saturation: f32, color_info: &ColorInformation) -> (f32, f32) {
        let (b_min,b_max) = match self.luminosity {
            Some(Luminosity::Dark) => {
                let min = Self::get_minimum_value(saturation,color_info);
                (min,min + 0.20)
            },
            Some(Luminosity::Light) => {
                let min = Self::get_minimum_value(saturation,color_info);
                ((1.00 + min) / 2.0,1.00)
            },
            Some(Luminosity::Bright) => {
                (Self::get_minimum_value(saturation,color_info),1.00)
            },
            Some(Luminosity::Value(value) | Luminosity::SaturationValue(_,value)) => {
                (value,value)
            },
            Some(Luminosity::Saturation(saturation)) => {
                (Self::get_minimum_value(saturation,color_info),1.00)
            }
            None => (Self::get_minimum_value(saturation,color_info),1.00)
        };
        (b_min, b_max)
    }

    fn get_minimum_value(saturation: f32, color_info: &ColorInformation) -> f32 /* 0..=1 */ {
        let lower_bounds = color_info.lower_bounds;
    
        for i in 0..lower_bounds.len() - 1 {
            let s1 = lower_bounds[i].0;
            let v1 = lower_bounds[i].1;
    
            let s2 = lower_bounds[i + 1].0;
            let v2 = lower_bounds[i + 1].1;
    
            if saturation >= s1 && saturation <= s2 {
                let m = (v2 - v1) / (s2 - s1);
                let b = v1 - m * s1;
        
                return m * saturation + b;
            }
        }
    
        0.0
    }

    /*
    fn hsv_to_rgb(hsv: (u16,u8,u8)) -> (u8,u8,u8) {

        // 223 43 91

        // this doesn't work for the values of 360
        // here's the hacky fix
        let hue = if hsv.0 == 360 {
            359
        } else {
            hsv.0
        };
    
        // Rebase the h,s,v values
        let hue = hue as f64 / 360.0; // 0.6194
        let saturation = hsv.1 as f64 / 100.0; // 0.43
        let value = hsv.2 as f64 / 100.0; // 0.91
    
        let h_i = (hue * 6.0).floor(); // 3
        let f = hue * 6.0 - h_i; // 0.7167
        let p = value * (1.0 - saturation); // 0.5187
        let q = value * (1.0 - f * saturation); // 0.6296
        let t = value * (1.0 - (1.0 - f) * saturation); // 0.7991
        let r;
        let g;
        let b;
    
        match h_i as u8 {
            0 => {
                r = value;
                g = t;
                b = p;
            },
            1 => {
                r = q;
                g = value;
                b = p;
            },
            2  => {
                r = p;
                g = value;
                b = t;
            },
            3  => {
                r = p;
                g = q;
                b = value;
            },
            4  => {
                r = t;
                g = p;
                b = value;
            },
            5  => {
                r = value;
                g = p;
                b = q;
            },
            _ => unreachable!("{{0..360}}/360 * 6 shouldn't be anything but an integer from 0..=6")
        }
    
        let result = (
            (r * 255.0).floor() as u8,
            (g * 255.0).floor() as u8,
            (b * 255.0).floor() as u8
        );
        result
    }
*/

/*
    fn get_hue_from_rgb(rgb: &(u8,u8,u8)) -> (u16,u8,u8) {
        let (red,green,blue) = rgb;
        let (red,green,blue) = (*red as i16,*green as i16, *blue as i16);
        // https://math.stackexchange.com/q/556341
        let c_max = red.max(green).max(blue);
        let c_min = red.min(green).min(blue);
        let delta = c_max - c_min;

        let hue = if delta == 0 {
            0
        } else {
            let mut hue = if c_max == red {
                ((green - blue)/delta) % 6
            } else if c_max == green {
                ((blue - red)/delta) + 2
            } else if c_max == blue {
                ((red - green)/delta) + 4
            } else {
                unreachable!("c_max '{c_max}' should equal one of the three colors '{red}','{green}','{blue}'")
            } * 60;
            while hue < 0 {
                hue = 360 + hue
            }
            hue as u16
    
        };

        let saturation = if c_max == 0 {
            0
        } else {
            (delta/c_max)*100
        } as u8;

        let value = ((c_max/255)*100) as u8;

        (hue,saturation,value)

    }
*/


}


