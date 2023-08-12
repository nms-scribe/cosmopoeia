/*

I have reasons for all of the languages in the list. Sources are as of August of 2024.

# Top 20 Native Languages

The following are the the top 20 languages by native speakers from <https://en.wikipedia.org/wiki/List_of_languages_by_number_of_native_speakers#Top_languages_by_population>
+ Mandarin Chinese
+ Spanish
+ English
+ Hindi
+ Portuguese
+ Bengali
+ Russian
+ Japanese
+ Yue Chinese (as Cantonese)
+ Vietnamese
+ Turkish
+ Wu Chinese (as Shanghainese)
+ Marathi
+ Telugu
+ Korean
+ French
+ Tamil
+ Egyptian Spoken Arabic (as Arabic)
+ Standard German
+ Urdu

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
+ Indonesian
* (Standard German)
* (Japanese)
+ Nigerian Pidgin
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
  + Swahili
  + Yoruba
  + Igbo
* Afroasiatic
  * (Arabic)
  + Berber
* Austronesian
  * (Indonesian)
  + Tagalog
  + Javanese
* Dravidian
  * (Telugu)
  * (Tamil)
* Turkic
  * (Turkish)
  + Uzbek
* Japonic
  * (Japenese)
* Austroasiatic
  * (Vietnamese)
  + Khmer
* Kra–Dai
  + Thai
  + Lao
* Koreanic
  + Korean
* Nilotic
* Mande
* Uralic
  + Hungarian
  + Finnish

# Some Indigenous Languages from Major Global Regions

An indigenous language in this case means a language descended from a language spoken in the same region since before historical records. This includes languages of peoples considered indigenous in Americas and Australia. But in the old world, for example, Turkish would not be indigenous to Southwest Asia, nor Arabic to North Africa, as both moved into those regions after the historical record began. Romance languages are indigenous to Europe, however, even though they might not be technically indigenous to parts of Europe, such as Greece, by this definition.

* Africa
  + Nubian
  + Amharic
  + Zulu
  + Khoekhoe
* Americas
  * Northern America
    + Navajo
    + Cherokee (Actually Iroquoian)
    + Ojibwe (Actually Central Algonquian)
    + Powhatan (Actually Eastern Algonquian)
    + Salish
    + Dakotan
    + Inuktitut
  * Central America
    + Nahuatl
    + Maya
  * South America
    + Guarani
    + Quechua
    + Arawakan
* Asia
  * Central Asia
    + Mongolian
* Australia and New Zealand
  + Strayan (just a list of Australian place names of indigenous origin)
  + Maori
* Oceania
  + Samoan

# Some Ancient Languages

+ Hellenic
+ Roman
+ Sumerian
+ Ancient Egyptian
+ Old Persian
+ Akkadian
+ Phoenician
+ Old English

TODO: I need to collect lists of words, especially place names, for each of the above.
- There's sixty three of these, so make sure we get a few of them. Maybe save some of them for later.
TODO: Then, I need to collect them all in one JSON file that gets stored, possibly compressed, in the code.
- I can just create some text documents with the language names for the file name, then I can load them in as list pickers using the dev-namers command.
- Then, write them all into one big json document with the write_json argument.
- Then, convert them to markov chain method instead, and test those.
TODO: Finally, the defaults will be automatically loaded when the namer set is created (maybe if we put an option in there), and will simply get overridden if the user wants to use someone elses namers.



*/