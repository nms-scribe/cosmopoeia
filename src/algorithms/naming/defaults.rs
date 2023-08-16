use libflate::deflate::Decoder;

/*

FUTURE: I hope to have namers built in to NFMT for the following languages eventually. The reasons for each language on this list are defined below. I've listed the sources and some notes for the ones I have completed, if you have better sources let me know. I stuck to a few major languages, a few popular ancient languages, and a few indigeonous languages to keep things balanced. The rest are postponed until I can find more time and better resources.

If you have any suggestions for languages, remember that there are thousands of living languages on Earth, and it would be impossible to support all of them. There must be a reason besides just your own personal reasons. Also, keep in your thoughts that the markov chain name generators are inadequate to generate words correctly for a language. Better would be a tool specific for the language that takes phonemes, phonotactic rules and common words like prepositions, directions and location suffixes and puts them together, then figures out how to spell them. This could then be used to generate nice lists which would be used in a list-picker namer. See my Elbie project for something that starts to get close to that.

[ ] Akkadian
[ ] Amazigh
[ ] Amharic
[X] Ancient Egyptian
    https://en.wikipedia.org/wiki/List_of_ancient_Egyptian_towns_and_cities
    https://en.wikipedia.org/wiki/Nome_(Egypt) 
    -- two-word names separated by space were broken into two and duplicates removed
    -- certain letters were changed into more "Latin" letters, since the two articles used different transcription systems.
[ ] Ancient Greek
[X] Arabic
    https://en.wikipedia.org/wiki/List_of_Arabic_place_names
[ ] Arawak
[ ] Bambara
[ ] Bengali
[ ] Cantonese
[ ] Cherokee
[X] English
    https://gazetteer.org.uk/purchase
    -- removed all places outside of England
    -- only types "Civil Parish", "Settlement", "Historic County" and "Village"
    -- removed some unusual characters such as apostrophes, as much as they add character, they will get put in weird positions and it won't look English.
[X] Finnish
    https://en.m.wikipedia.org/wiki/Regions_of_Finland
    https://en.m.wikipedia.org/wiki/Historical_provinces_of_Finland
    https://en.m.wikipedia.org/wiki/Names_of_places_in_Finland_in_Finnish_and_in_Swedish
    https://en.m.wikipedia.org/wiki/List_of_Finnish_municipalities
    https://en.m.wikipedia.org/wiki/List_of_cities_and_towns_in_Finland
[ ] French
[ ] German
[ ] Guarani
[ ] Haitian Creole
[ ] Hindi
[X] Hungarian
    https://en.m.wikipedia.org/wiki/List_of_cities_and_towns_of_Hungary
    https://en.m.wikipedia.org/wiki/Counties_of_Hungary
    https://en.m.wikipedia.org/wiki/List_of_regions_of_Hungary
[X] Igbo
    According to https://en.wikipedia.org/wiki/Igboland, "Igboland is roughly made up of Abia, Anambra, Ebonyi, Enugu, Imo, Northern Delta and Rivers states."
    https://en.wikipedia.org/wiki/List_of_villages_in_Abia_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Anambra_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Ebonyi_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Enugu_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Imo_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Rivers_State
    -- attempted to remove every element of English from the words, I can't guarantee I was consistent.
    -- I also don't know that all of these names are from Igbo or a related language.
[ ] Inuktitut
[ ] Indonesian
[X] Japenese
    https://en.wikipedia.org/wiki/List_of_towns_in_Japan
    https://en.wikipedia.org/wiki/List_of_villages_in_Japan
    https://en.wikipedia.org/wiki/List_of_cities_in_Japan
[ ] Javanese
[X] Khmer
    https://en.wikipedia.org/wiki/List_of_districts,_municipalities_and_sections_in_Cambodia
    https://en.wikipedia.org/wiki/Provinces_of_Cambodia
[ ] Khoekhoe
[X] Korean
    https://en.wikipedia.org/wiki/List_of_towns_in_South_Korea
    https://en.wikipedia.org/wiki/List_of_cities_in_South_Korea
    https://en.wikipedia.org/wiki/List_of_counties_of_South_Korea
    https://en.wikipedia.org/wiki/List_of_districts_in_South_Korea
    https://en.wikipedia.org/wiki/List_of_townships_in_South_Korea
[ ] Lao
[X] Latin
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Italy_and_Malta
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Continental_Europe,_Ireland_and_Scandinavia
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Britain
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_the_Balkans
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Asia
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Africa
    https://en.wikipedia.org/wiki/List_of_Latin_place_names_in_Iberia
    -- I tried to stick to countries which were part of the original Roman Empire (not Byzantine Empire)
[ ] Maasai
[X] Mandarin 
    https://en.wikipedia.org/wiki/List_of_cities_in_China
    -- Note that I have no way of knowing which of these cities are in Mandarin or not, I can only assume that the government's official names for cities are in Mandarin, and that these are official names.
[ ] Maninka
[ ] Maori
[ ] Marathi
[ ] Nahuatl
[ ] Navajo
[ ] Nigerian Pidgin
[ ] Nobiin
[ ] Ojibwe
[ ] Old English
[ ] Old Persian
[ ] Phoenician
[ ] Portuguese
[ ] Powhatan
[ ] Quechua
[ ] Russian
[ ] Salish
[ ] Samoan
[ ] Shanghainese
[ ] Sioux
[ ] Southern Luo
[ ] Spanish
[ ] Sumerian
[ ] Swahili
[ ] Tagalog
[X] Tamil --
    https://en.wikipedia.org/wiki/List_of_towns_in_Tamil_Nadu_by_population
    https://en.wikipedia.org/wiki/List_of_cities_in_Tamil_Nadu_by_population
[ ] Telugu
[ ] Thai
[ ] Tibetan
[X] Turkish
    https://en.wikipedia.org/wiki/List_of_largest_cities_and_towns_in_Turkey
[ ] Urdu
[ ] Uzbek
[ ] Vietnamese
[X] Yoruba
    According to https://en.wikipedia.org/wiki/Yoruba_people#Language, Yoruba culture is predominant in Oyo, Osun, Ekiti, Ogun, Ondo and Lagos stats in Nigeria
    https://en.wikipedia.org/wiki/List_of_villages_in_Oyo_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Osun_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Ekiti_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Ogun_State
    https://en.wikipedia.org/wiki/List_of_villages_in_Ondo_State
    -- similar problems as Igbo, as the data is formatted the same
[ ] Yucatec
[ ] Zulu
[ ] An Australian Indigenous Language

I have reasons for all of the languages in the list. Sources are as of August of 2024.

# Top 20 Native Languages

The following are the the top 20 languages by native speakers from <https://en.wikipedia.org/wiki/List_of_languages_by_number_of_native_speakers#Top_languages_by_population>
* Mandarin Chinese
* Spanish
* English
* Hindi
* Portuguese
* Bengali
* Russian
* Japanese
* Yue Chinese (as Cantonese)
* Vietnamese
* Turkish
* Wu Chinese (as Shanghainese)
* Marathi
* Telugu
* Korean
* French
* Tamil
* Egyptian Spoken Arabic (as Arabic)
* Standard German
* Urdu

# Top 20 Languages

The following are the top 20 languages by total speakers from <https://en.wikipedia.org/wiki/List_of_languages_by_total_number_of_speakers#Ethnologue_(2022,_25th_edition)>
* (English)
* (Mandarin Chinese)
* (Hindi)
* (Spanish)
* (French)
* (Modern Standard Arabic)
* (Bengali)
* (Russian)
* (Portuguese)
* (Urdu)
* Indonesian
* (Standard German)
* (Japanese)
* Nigerian Pidgin
* (Marathi)
* (Telugu)
* (Turkish)
* (Tamil)
* (Yue Chinese)
* (Vietnamese)

# Top 2 In the Top 14 Language Families

The following are primary language families with more than 20 million speakers from <https://en.wikipedia.org/wiki/List_of_language_families#Language_families_(non-sign)>. I've removed those families which did not have a "glottolog" code in the sidebar, as either poorly attested or controversial. For each of these families, I've picked the top two of those spoken by more than 5 million speakers. If a language family did not have enough such languages, then only one or none were picked. Some of these took research, speaker counts were taken from corresponding Wikipedia articles. 

I have no guarantee whether I was using L1 or L2 counts. Because of that, I chose three languages for Atlantic-Congo, as Swahili as noted with 20 million L1 which would make it #3 if the other numbers were L1, but 80 million as L2, which would make it #1. For Austronesian, Tagalog and Javanese tied with 82 million speakers, and since Tagalog's count was L2, I could not disqualify Javanese.

* Indo-European
  * (Spanish)
  * (English)
* Sino-Tibetan
  * (Mandarin)
  * (Yue Chinese)
* Atlantic–Congo
  * Swahili
  * Yoruba
  * Igbo
* Afroasiatic
  * (Arabic)
  * Amazigh
* Austronesian
  * (Indonesian)
  * Tagalog
  * Javanese
* Dravidian
  * (Telugu)
  * (Tamil)
* Turkic
  * (Turkish)
  * Uzbek
* Japonic
  * (Japenese)
* Austroasiatic
  * (Vietnamese)
  * Khmer
* Kra–Dai
  * Thai
  * Lao
* Koreanic
  * Korean
* Nilotic
  * Southern Luo
  * Maasai
* Mande
  * Bambara
  * Maninka
* Uralic
  * Hungarian
  * Finnish

# Some Indigenous Languages from Across the Globe

This one is harder to define, and harder to come up with concrete reasons to pick. Many are not spoken by more than a thousand or so people, making choosing by popularity a bad idea. They're effect on history and geography seems like the better reason to pick, but I'd have to filter through so much history to choose those. The best I can do is choose from the history I've already learned, which is quite fuzzy outside North America.

The list below is tentative, and may change.

* Africa
  * Nobiin
  * Amharic
  * Zulu
  * Khoekhoe
  * Maasai
* Americas
  * Northern America -- Most of these are chosen because of their predominance in toponyms of the U.S. and Canada.
    * Navajo
    * Cherokee
    * Ojibwe
    * Powhatan
    * Salish
    * Sioux
    * Inuktitut
  * Central America and the Caribbean
    * Nahuatl
    * Yucatec
    * Haitian Creole
  * South America
    * Guarani
    * Quechua
    * Arawak
* Asia
  * Central Asia
    * Tibetan
* Australia and New Zealand
  * An Australian Indigenous Language -- I don't know enough about Australia to decide which is the best choice
  * Maori
* Oceania
  * Samoan

# Some Ancient Languages

* Ancient Greek
* Latin
* Sumerian
* Ancient Egyptian
* Old Persian
* Akkadian
* Phoenician
* Old English

FUTURE: There is an issue with the English one, and I suspect it might happen in the others but I don't notice it as I'm not a native speaker. The word "Wicken-onter", as an example, is a valid generation because '-' can be followed by the lowercase letter "o" in such names as "Staunton-on-Wye". However, there's no way for the Markov chain to know that this only happens if the word is a preposition, like "on", "upon", "by", etc. This is just one of many issues with Markov chain name generators.

*/

/*
The defaults.deflated file was generated with the following command run from the root of the project directory:
`cargo run -- dev-namers src/algorithms/naming/*.txt --text-is-markov --write-deflated > src/algorithms/naming/defaults.deflated`

The defaults.json file is intended for review of what was created. FUTURE: If I ever need to "edit" the defaults.json I'll have to
come up with another way to deflate it quickly.

(Here's a double comment end to match the unintentional comment-start in that code) */*/

// NOTE: This adds 0.3 MB to the executable (from 1.1MB to 1.4MB). If I didn't deflate it it would add 0.8 MB (1.1 -> 1.9). 
// FUTURE: The question is whether including the libflate library just for that is worth it, or if there's a simpler 
// compression mechanism that would workd.
pub(crate) const DEFAULT_NAMER_DATA_DEFLATED: &[u8] = include_bytes!("defaults.deflated");

pub(crate) fn get_inflated_namer_data() -> Decoder<&'static [u8]> {
    Decoder::new(DEFAULT_NAMER_DATA_DEFLATED)
}