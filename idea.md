**NOTE** The below are personal notes and comments during development of this project. It's not a specification document, and what is mentioned here may not resemble the code finally produced.

Highly inspired by [Azgaar's Fantasy Map Generator](<https://azgaar.github.io/Fantasy-Map-Generator/>) (AFMG).

This project would be a series of console commands, and possible QGIS extensions, and other tools which make it easier to generate fantasy maps using GIS data.

# Name

I'm going to settle on "cosmopoeia" for now. This word comes from anglicized Greek roots "cosmo-", meaning world, and "-poeia", a making or creation. 

The first part is obvious. Although many might associate the word with space, or even a whole universe, to the Greeks, the world they lived in was their entire universe, so it's the same thing. Plus, there name which is closer in scope, "oikouménē", doesn't scan well in English when combined with the same suffix, and it's descendant "ecumene" is known better as a term used in the organization of churches.

The second part you are probably familiar with in "onomatopoeia", which literally means name creation. However, I like the parallel with the word "mythopoeia", a word Tolkein used to refer to creation of his mythologies.

A quick search for this word, and it's variant "cosmopeia", found it related only to a couple of obscure philosophical texts (as all philosophical texts are obscure), and to misspellings of words like cosmopolitan, cosmology and cassiopeia. Cosmopeia may be a word in Portuguese, but I couldn't find a translation of the pages that word turned up.

This wasn't my first word for this. I originally started programming this under the term "nfmt", for "Neil's Fantasy Map Tools". However, I found the term uninspiring and vague. What if I come up with other mapping tools that have nothing to do with creating worlds?

Other terms I considered:

I decided that the single verbs were not specific enough, and could conflict with some future programs which might also be used create things, such as art or music. I'll leave that for some software giant to use.

*facio* - From latin, "I make, construct..."
*creō* - From latin, "I create"
*creatio* - From latin, "Creation"
*poiéō* - From greek, "I make"

I briefly considered the imperative forms of some of those verbs. Considering its the name of a command you tell the computer. But I decided against these for the same reason as the other verbs.

*fac* - From latin, imperative form of "You make, construct.."
*creā* - From latin, imperative form of "You create"
*creāre* - From latin, imperative form, "you be created"
*poíei* - From greek, imperative form of "You make"

Similarly, this one word noun seems inspecific:

*mundus* - From latin, "world, universe"

The following two words brought in a religious element to the naming which I was uncomfortable with, but I also worried about vagu

*génesis* - From greek, "origin, source", "creation"
*subcreation* - Term used by Tolkein to refer to his process of world building, as opposed to actual "creation" (out of nothing) in the sacred sense.

I ended up going with Cosmopeia
*cosmopoeia* - From anglicized greek roots *cosmo-* "world" and *-poeia*, "creation"

# Reasoning

AFMG maps are a rectangle full of voronoi cells around randomly generated points. Each cell has an elevation, plus a number of other attributes, which make all land within that cell fairly uniform. In some ways this resembles hexagonal grid maps of role-playing games, and if you had uniform placement of points, it would look exactly like that. However, the random points and the voronoi give a more organic look to the output. With appropriate line smoothing and styling, the resulting maps look very much like a traditional fantasy map.

The problem I have with AFMG is that it is a monolithic tool inseparable from the user interface built around it. While the creator has added a lot of features and customization to the system, additional features and customization are dependent on their schedule and vision. Due to its development as a browser application, it suffers from performance problems with very complex maps. 

Implementing much of the functionality as commands around open geographic file formats would mean: 

1) No need to create a complex user interface. QGIS and other open mapping tools could be used to display the results, allowing me to focus on the algorithms.
2) The ability to use alternative data formats, and pull in data produced by custom tools into any point in the map generation process.
3) The ability to work with larger, more complex maps by utilizing optimizations built-in to GIS tools.

In addition, this would give me a chance to find ways to improve upon AFMG's algorithms. For example, AFMG creates flat maps. However, I believe that if the voronoi centers were generated correctly and the voronoi generation algorithm was based on projected metric distance instead of degrees, I would be able to create cells that are similar in area on a globe. This would make terrain processing algorithms more verisimilar.

# Development

I foresee this project not as a big monolithic processing script, but a series of smaller scripts that perform parts of the operation. It would also allow creating one master script which can easily generate a world similar to AFMG's "New Map" button.

These tools could be developed in a combination of python and rust. There are many geographic libraries available in rust, including code allowing access to common GIS file formats, and a binding library for GDAL. This would make development of command-line tools a breeze. Python, however, would be required to integrate some of these tools into QGIS as plugins and commands for its toolbox.

Alternatively, the tools could be developed entirely as python scripts for QGIS, however that would limit their usage to only those who wish to use QGIS. For that matter, I could develop all of the commands as python scripts, but I fear the maintenance of such tools.

# The Tools

At least at the start, the program will be a single, monolithic tool which handles individual command arguments to run different algorithms, or collections of algorithms. I believe this will allow quicker compilations when manipulating the back-end library code. It would also produce a smaller total file size, since copies of the shared code do not have to be copied into each separate program. Once it is working and working well, it could be separated into smaller programs that utilize a shared library, but that is a challenge for another time.

The name of the executable will be nfmt, which are initials that stand for "Neil's Fantasy Map Tools", or using my creative name, "N M Sheldon's Fantasy Map Tools".

I foresee the need for the following commands:

* `nfmt genesis`: the equivalent of the 'New Map' button on AFMG. It will create random terrain, and populate it with random states, and create a final set of maps and data all at once, including dissolved versions. There will be options for specifying some parameters for the world, but overall this won't be as flexible as running the commands separately. However, it may be enough.
* `nfmt convert-afmg`: This will convert GIS data from AFMG into formats used for NFMT. It may eventually be able to read a ".map" file, however that format is complex, poorly documented and probably subject to change.
* `nfmt create-terrain`: generates an NFMT Terrain file given world parameters and optional terrain templates. The templates will be reminiscent of similar templates used in AFMG, and may even take the same formats. There should be some built-in templates.
* `nfmt convert-heightmap`: generates an NFMT Terrain file given world parameters and a heightmap. Some of the world parameters might be taken from the heightmap.
* `nfmt convert-image`: generates an NFMT Terrain file given a graphic image, that converts based on hue in a similar manner to AFMG. This might be the same tool as `convert-heightmap`, but with some specific options. The purpose of this tool is mostly to allow the user to complete the same tasks as AFMG, which seems to be quite flexible on its input.
* `nfmt gen-climate`: generates climate information for an NFMT Terrain file (temperature and precipitation) given climate parameters. The simple algorithm that AFMG uses seems to assign temperatures based on latitude and elevation, and moisture based on latitudinal wind directions and distance from the ocean. A more complex climate model may be developed in the future.
* `nfmt import-climates`: given raster data classified with temperature and moisture data, and a similar extent as the NFMT Terrain file, applies their results to the terrain. This would allow generating climate data from some other tool.
* `nfmt gen-biomes`: this will take an NFMT Terrain file and an NFMT Biome file and generate biomes on the terrain file based on climate information already in it.
* `nfmt gen-water`: this will generate an NFMT Water file containing river and lake polygons for an NFMT Terrain file given climate information.
* `nfmt gen-people`: this takes an NFMT Terrain file, for which biomes have been generated, and populates it with people, creating random cultures, nations, cities, etc. This creates a bunch of accessory files by default, but some of these can be pre-created. Although this command does a lot, the process of generating these things is fairly interconnected, so it almost needs to be. There may be additional `gen-*` commands which will allow separating parts of this process into further steps, but only if I can actually divide it into steps.
* `nfmt gen-features`: this takes an terrain file with biomes and people, and generates interesting points of interest and regions for the world, equivalent to markers and zones in AFMG.
* `nfmt regen-*`: these commands regenerate specific entities (like cities, nations, etc.) for a given NFMT Terrain file, while keeping other entities the same as much as possible. These commands will also allow "locking" some entities, to prevent changes to those, or regenerating them only within specific areas of the map, such as a nation or a province.
* `nfmg dissolve`: this takes a Terrain map and dissolves it into various entity maps (biomes, states, provinces, etc.). An option would allow smoothing the lines, but preferably in a way that they match with coastlines and that provinces match with states. Essentially, this is the one that produces the final "product", from which you can then make nice maps.
* `nfmt submap`: this takes a viewport and world parameters and attempts to create an NFMT Terrain and auxiliary vector files for a smaller part of the world. If the original heightmap is provided, it uses that to help interpolate data for the smaller scale. Otherwise, it uses some randomization to create the illusion of more detailed cells.

# File Specifications

The NFMT tools which generate in parts can't work without being able to expect access to specific data in the input files. This creates a number of file "types", which aren't necessarily tied to specific formats. There are two basic formats for each file: 'vector files' and 'tables'. 

The files below have been given names representing a "standard" naming scheme, which makes it easier to tell where things are, and automatically find files. All commands should allow customizing these file names, but if not specified these will be the default. In the names below `<world>` represents the project or world name, and `<ext>` represents a compatible extension. When "discovering" files, an error will occur if multiple files are given with the same identifier but different supported extensions. A few items don't have names, these are generally configuration files which would be used for multiple worlds.

**Vector** This is a GDAL-supported vector-based GIS file or layer in a multi-layer file.

**Table** This is a file containing tabular data (columns and rows) in a format supported by NFMT. NFMT will support CSV files at the very least. It may support more files if these sorts of things are supported by GDAL.

* NFMT Terrain: (`<world>.terrain.<ext>`) vector containing polygons or multi-polygons and a number of expected fields. It's possible that some tools will add those fields if they do not exist. The four that are absolutely needed are a voronoi ID field, a NEIGHBORS field that lists the polygons that border this one, a numeric ELEVATION field, and a boolean IS_OCEAN field. Other fields represent climatic and political data for the world, which is uniform across a given cell. While the program is not able to detect this, it assumes the polygons are laid out in the voronoi format described earlier in this document.
* NFMT Water: (`<world>.freshwater.<ext>`)vector containing polygons that represent rivers and lakes.
* NFMT Towns: (`<world>.towns.<ext>`) vector containing points that represent the locations of towns (Burgs in AFMG). I understand why AFMG might have chosen burgs over cities, towns, etc. to avoid preconceived notions and regional government definitions. However, I feel the word "burg" is burdened by the same problems as those words, and in English the word "town" is the closest we have to a generic word for an urbanized area.
* NFMT Routes: (`<world>.routes.<ext>`) vector containing lines that represent routes between towns.
* NFMT Point Features: (`<world>.features-point.<ext>`) vector containing points that represent the locations of special features in a World. 
* NFMT Poly Features: (`<world>.features-poly.<ext>`) vector containing polygons that represent special regional features in a World.
* NFMT Labels: (`<world>.labels.<ext>`) vector containing points with labels and label types for displaying on the map. The points represent good places to put the labels. There might also be IDs to associate them with specific entities, so it's easier to update them when those entities are modified, without losing the point locations.
* NFMT Biomes: (`<world>.biomes.<ext>`) Table containing a list of biomes with ID, Name, Habitability, Temperature Range, and Precipitation range. The last two fields are used for finding biomes by climate. Habitability is used by `gen-people` to determine where people are living.
* NFMT Nations: (`<world>.nations.<ext>`) Table containing a list of nations (States in AFMG) with ID, Name, and some other important data. The IDs are used in the Terrain file to specify what nation controls that cell. I do not presume to know why AFMG chose the word state for this entity. While it works, it is an ambiguous term in the U.S, Australia and other nations with "states". Meanwhile, "state" implies a heavily organized system of government, whereas "nation" can have connotations of a group of people following similar cultural rules without necessarily having heavy organization. I like nation better.
* NFMT Provinces: (`<world>.provinces.<ext>`) Table containing a list of provinces with ID, Name and other data. The IDs are used in the Terrain file to specify what province a cell is in.
* NFMT Cultures: (`<world>.cultures.<ext>`) Table containing a list of cultures with ID, Name, and other data. The IDs are used in the Terrain file to specify what culture is predominant in a cell.
* NFMT Religions: (`<world>.religions.<ext>`) Table containing a list of religions with ID, Name, and other data. The IDs are used in the Terrain file to specify what religion is predominant in a cell.
* NFMT Name Bases: Table containing language names and lists of words and other parameters for generating names for places. Note that NFMT might be able to work with my other project, Elbie, to generate names. The Name Bases are used to generate names using similar algorithms to AFMG.
* NFMT Culture Sets: Table containing information for randomly generating sets of cultures. This might be more than just a table. This is something that is built into AFMG and not configurable, but I feel it's important to be able to configure this.
* NFMT Dissolved Files: these vectors represent dissolved, and possibly smoothed data, as created by the `--dissolve` command.
    * `<world>.dissolved-elevation.<ext>`
    * `<world>.dissolved-ocean.<ext>`
    * `<world>.dissolved-biomes.<ext>`
    * `<world>.dissolved-nations.<ext>`
    * `<world>.dissolved-provinces.<ext>`
    * `<world>.dissolved-cultures.<ext>`
    * `<world>.dissolved-religions.<ext>`

# Configuration

There are two ways of configuring a command: command-line operations and configuration files. Configuration files will be generally written in JSON, and passed with a single parameter to each command. When processing configuration, later configuration values override earlier ones, whether the source was a command-line or a file.

Some configuration options include:

* `seed`: A number to use to seed random number generators. NFMT should be designed so that if the seed is provided, it will always return the same results for the same input, given the same version of a program. This aids testing and allows the user more control over the results.
* `cells`: An integer indicating the number of voronoi cells to generate when creating Terrain maps. The more there are, the more detailed the map, and the more processor intensive.
* `epoch`: An enum representing different types of worlds and technology levels for use in `gen-people` and related tools. AFMG seems to always generate cultures which might fit into a period 16th-19th centuries of Earth. It is not possible to limit it to stone or bronze age cultures, for example, where naval cultures do not extend anywhere near as far and non-agricultural societies are much more numerous.
* `cultures`: An integer representing the number of cultures to generate.
* `nations`: An integer representing the number of nations to generate.
* `provinces-ratio`: An integer representing the average number of provinces to generate per nation. I'm not certain how to do this correctly, as that number may depend on the government and size of the nation.

There may be numerous other options, including some numbers set under `Options` and `Units` in AFMG. However, as the algorithms I'm using may differ, and some of those values are used for the UI, I can't be certain until I develop the algorithms what parameters I'll actually need.

# Algorithm Improvements

For the most part, AFMG is open source, and I can use some of the same algorithms as a basis for my code. However, there are a few things I know I will change. Note that most of these are not there yet, these are just part of the ideas so I can add them later.

## Equal-Area Voronoi Cells

AFMG creates flat maps. This format hides the fact that the areas of cells near the poles are significantly smaller than the cells near the equator. This automatically creates a bias towards generating nations closer to the poles, which is not verisimilar.

While I can't make the voronoi cells exactly the same area without creating a hexagonal grid map, I could try to place the points so that the areas of each cell are similar to each other on the globe. This means, placing less points in the upper latitudes. I am not yet sure how to do this, but this might be doable by taking a subset of points on a rectangular extent that would fit into a shape similar to some pseudocylindrical projection, such as Mollweide. These points would then be stretched out from the east and west by some standard formula to fill the rectangle.

Part of this process is replacing anything based on distances between coordinates with a distance algorithm that assumes the coordinates are lat/lon. This includes the production of voronoi. (which depend on equidistance between points, this is done in the calculation of triangle circumcenters), and bezier curve generation for rivers, lakes and other features.

The results of such a map will look very strange in am equirectanguilar projection, due to colors stretching east-west near the poles, and straighter coasts and boundary lines in those areas. I could overcome this by using something that's not a true voronoi. Take the original voronoi generated over a rectangular area, and calculate new shapes by dissolving them by which stretched voronoi their center points lay in, creating more organic edges to the higher latitude cells. 

Something like this could also be used to get around another problem with AFMG: uniformly smooth borders. If the original rectangular voronoi created more points in areas of higher relief, this could be used to create rougher looking cells in those areas in the final map.

Another requirement for this system is a way to knit the wrapped edges together when the map represents an entire world. The cells on the edges need to wrap around 180 W, so that elevations, rivers and political continuums meet appropriately. To create these sorts of voronoi, you would probably need to "repeat" the easternmost points transformed to just beyond the western edge (or vice versa). And then, after that is all done, somehow merge them together. The boundaries of the map still need to be rectangular, so the merged cells then have to be split by the anti-prime-meridian line. This probably requires the cells to be multipolygons instead.

## Elevationless Ocean

Land below sea level exists. NFMT does not define its ocean based on a specific elevation, but marks it as a property of the terrain cell, allowing for mid-Messinnian Mediterranean-style seas, or at least Death Valley. Whether such things get generated, however, or is only available in imported heightmaps, is not for me to say at this point.

## Climate Models

The climate model used by AFMG is simplistic. Although I could play around with it more, one noticeable problem is the lack of distinct climates from west and east coast. I think at first I will use the algorithm as straight as I can get it from AFMG. Eventually, however, I want something that at least looks at air pressure for generating winds. I'm just not going to go as for as using a real climate model tool. However, the tools as above should be enough to be able to pull in that kind of data if available.

A second problem is that there doesn't seem to be any accounting for the Earth's tilt. Although the width of the temperate zone vs polar can be modified, based on changes to the curve, I feel there might be improvements to the climate calculations if I can get temperature averages for each of the four seasons at least. At the least I can have "cold winters" and "hot summers".

## People Models

I don't know the details of AFMG's algorithms for generating political features. I don't know if there's something I can improve. I just know that it outputs results that seem odd sometimes. I have to play around with the assumptions for these, maybe what AFMG has are more sufficient than I'm seeing.

One thing I will change, are the "types" of cultures. I will also add the ability to filter out types depending on an epoch or world type. AFMG has "generic", "river", "lake", "naval", "nomadic", "hunting", and "highland". I'm surprised there is no "agricultural" or "industrial" culture type, and I'm not sure what a "highland" culture would be, short of men in kilts. In addition, "naval" cultures in the 19th century are much different from naval cultures in the 1st century, and in earlier ages those cultures shouldn't even be generated. Finally, I feel like a "wildlands" culture is unlikely -- it should be filled in with all sorts of nomadic and hunting cultures instead -- at best there might be a frontier culture of sorts which combines two or more cultures together. There should also be some fantasy cultures: Elves who actually prefer to live in forests, and Dwarves who actually want to build cities in mountains. And creatures whose real homes are underground, or underwater.

## Water

Water flow is based on the tiles, however, there are two problems: 1) rivers and lakes aren't "detailed" enough. and 2) rivers and lakes should be able to form natural political borders. 

I'm wondering if generating river flow would be better done with a heightmap instead. 
1) Take the elevation tiles and generate a raster heightmap using interpolation from the tile sites. 
2) use that result to generate the rivers and lakes on a grid instead using more traditional DEM water flow algorithms.
3) Convert those maps and rivers into polygons using appropriate digitization and smoothing algorithms.
4) Pull those back in and intersect them with the tiles somehow to get a "water-infused" tile layer, where tiles are split by rivers and lakes. I just have to watch out for cases where the new tile areas become tiny because of a minor crossing. Maybe in that case we add the new tile to neighboring tiles.
5) Use that layer for generating people things.

# Testing Commands:

The following commands were used, to generate the testing maps of my Inannak world during development. `time` is not the bash command, but a GNU program you might have to install on your machine and call by path.

```sh
cargo run big-bang testing_output/World.gpkg --overwrite-all --cultures share/culture_sets/afmg_culture_antique.json --namers share/namers/afmg_namers.json --default-namer English --seed 9543572450198918714 from-heightmap ~/Cartography/Inannak/Inannak-Elevation.tif recipe --source share/terrain_recipes/heightmap-recipe.json
```

The following was used to generate shared World.gpkg:

```sh
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run big-bang share/qgis/World.gpkg --overwrite-all --cultures share/culture_sets/afmg_culture_antique.json --namers share/namers/afmg_namers.json --default-namer English --seed 9543572450198918714 blank 180 360 -90 -180 recipe-set --source share/terrain_recipes/afmg_recipes.json --recipe continents
```

# Improvement Thoughts 2024-09-19

These thoughts are on a major overhaul that would support raster worldbuilding for the physical elements. Some of the things I want to address:

* Parts of the physical process, especially precipitation, rivers and lakes are broken in their algorithms.
* It would be nice to be able to use the physical algorithms, once refined, on raster maps without having to pull into tiles, for other kinds of world-building.
* It would also be nice to turn Cosmopoeia into a general-purpose world-building tool. This is what I originally envisioned.
* The political processes suffer from a major issue in that borders do not form at rivers.

**NOTE: These might go into geo-tools instead.**

I'm proposing switching the physical processes over to raster-based operations, and changing the way tiles are generated from this in the political process.

This is what the plan is:

* Random Terrain Generation can stay the same. The tile mechanism for building terrains is fine, even if the various recipe tasks could be improved. Below are listed some new commands.
* `export-heightmap`: Will let you extract an interpolated heightmap from the terrain generation. 
  * Parameters
    * World Geopackage file
    * Output heightmap
    * Raster resolution
    * Maybe, interpolation parameters
    * Maybe, interpolation method
  * Create a new raster, with the extents matching the world.
  * Create an interpolator
     * Natural Neighbor Interpolation seems like the best, and it's already got rust implementations: 
        * https://blog.mikebourgeous.com/2021/06/09/voronoi-diagrams-and-natural-neighbor-interpolation-explained-in-video/
        * https://gwlucastrig.github.io/TinfourDocs/NaturalNeighborTinfourAlgorithm/index.html
        * https://crates.io/crates/naturalneighbor
        * https://crates.io/crates/spade
        * If there are too many pixels and the process is slow, one option for this is if it there are too many pixels, you can just interpolate to a lower resolution, then use some other interpolation method for the rest.
     * Other options might include 'Bicubic Interpolation', 'TIN Interpolation' and maybe 'Cubic Spline' interpolation. I'd rather just pick one, but provide data that others could use to run their own algorithms on.
     * Either way, the goal is an interpolator that reads in a bunch of points, then lets you query a point to calculate its interpolation, one at a time.
  * Read the heights of the tiles, and load the interpolator with those heights and the coordinates of the tile site.
  * For each pixel in the height-map raster, query the interpolator to get its interpolated height for that pixel, and write it to the heightmap.
* `export-ocean-mask`: Extracts the ocean seeds from terrain generation and flood fills based on a given heightmap
  * Parameters:
    * World Geopackage
    * Reference heightmap
    * Output ocean mask
    * Maybe, flood parameters
  * Create a flood-filler
    * The goal is an object that takes a bunch of seed points, and migrates around an existing raster to flood-fill based on given criteria.
  * Read the tiles and find the sites of all ocean tiles, and feed the flood-filler. Set the condition to be any pixel <= 0.
  * Iterate through the flood-filler, or however it's done, and mark ocean pixels in the mask based on the results.
* `export-rasters`: Runs `export-heightmap` and `export-ocean-mask` all at once.

* **Spatial Equity**. For all of the raster processes, where possible, I will incorporate the area of the pixel given the latitude into the process. This means that, for example, a pixel at the same elevation and temperature could have less water stored in it if it's further away from the equator. I contemplated some sort of fancy pixel format which has less pixels in the line, but I think assuming pixel area is probably the best solution to maintain spherical activity.
* **Angular Equity**. Another thing I might want to consider is, whenever I'm looking for slopes, use some sort of bilinear interpolation to figure out the direction, rather then just NSEW. 
* `raster-erosion`: This command would take a heightmap and ocean mask, and run erosion algorithms on it. 
  * Although there would be built-in repetition factors, it should be possible to re-run the algorithm.
  * After each repetition of the erosion process, the ocean mask is recalculated by:
    * Masking out those pixels from the old ocean raster that are now above sea level.
    * Flood filling to sea level using the remaining pixels as a seed.
    * This process might have the habit of creating little oceans in basins that are filling in, the user can always edit the raster later, though.
  * Alternatively, it might be possible to run this in cycles by just using the `export-ocean-mask` after each erosion.
* `raster-temperature`: This command takes a heightmap and ocean mask, and using the parabolic method I use currently, assigns temperatures based on elevation and ocean. This version can include tilt in the process. The result is actually a 12-band raster, with average temperatures for each month of the year.
  * I can't remember if the current algorithm takes distance from ocean into consideration. I might do that. 
  * To allow for climate cycles, I might allow passing in a shade mask, which could be calculated based on precipitation and winds from later parts of the process. This might be a future thing, though.
* `raster-pressure`: This one may or may not be done, or it might just be a simple function of temperature. Currently, in cosmopoeia, I have winds moving based on temperature. But there's a possibility that I can actually calculate pressure areas for each month of the year, making winds easier to calculate. It's also possible that I might take existing winds and precipitation as an input, allowing for cycles.
* `raster-winds`: Using similar algorithm as I'm currently using, this one would calculate wind directions for each month of the year.
* `raster-currents`: This is a process to be determined, to find the ocean currents. It would be based on prevailing winds, but also coriolis effect.
* `raster-precipitation`: This algorithm gets an overhaul. With pixels I think I can do this more easily. The following is done for each month. 
  * Requires winds and temperature.
  * Every cell is assigned a starting humidity level. This is calculated thusly:
    * Over land cells, it's zero.
    * Over ocean cells, it's calculated based on the amount of water in the ocean, based on the pixels area, that can evaporate on a given day (this is probably an f64 number). 
      * This is then compared to the amount of water the air can actually hold. Any excess water is converted into precipitation, which is marked on the output raster, probably after being converted into millimeters over the entire area of the pixel.
    * If I can pull in a pre-existing lakes layer, I can add that humidity as well. This again allows for cycles to make more realistic worlds.
  * Humidity is moved onto a new raster. Each cell is visited:
    * Using wind direction, the neighbor to which the humidity will be moved is discovered.
    * The humidity is moved onto the next cell, adding on to any humidity already moved there. If the humidity would be higher than what the air in that pixel can hold, it is converted into precipitation as above. Higher elevations and colder temperatures hold less humidity, which means this technique should form mountain deserts as easily as it forms deserts from distance to ocean.
    * There is likely to be evaporation from the precipitation from the previous iteration to be added to the humidity as well.
  * This is repeated a certain number of times. There are several possibilities:
    * I could just repeat it based on the number of pixels on the heightmap in the longest direction, this would allow making sure the humidity covers as much as possible. However, the precipitation in mm would not be accurate for this as it would likely be rain over a thousand years or something. But then I could divide the precipitation up into single days and use that to calculate an average for the month.
    * I could just repeat it 30 times, assuming the calculations I'm doing are based on daily evaporation rates. But this may not spread the humidity far enough.
    * A hybrid option would be to do the 1000 day loop or whatever as above, but ignore all precipitation gained from that. Then just take the precipitation from the last loop.
  * This is not accurate to real precipitation processes, but it should be adequate for our purposes.
* `raster-water`: This algorithm also gets an overhaul, but it might actually be similar.
  * **Alternatively**: See the next option
  * Requires precipitation.
  * First, slopes and slope angles are calculated for all land. Pixels with no lower neighbors have a null slope. Equal elevations are not lower.
  * Second, precipitation across a whole year is calculated and laid down on each pixel as "water".
  * Mark the flow flag as on.
  * Loop while flag is on.
    * Flow step:
      * Flow flag is turned off.
      * Water in every pixel is moved down the slope to the next pixel. It is added to any that has already been added from another pixel (but not to the precipitation flow)
      * Yes, there can be branches, but they're more unlikely in this algorithm, I think.
      * If the water ends in an ocean, it is done.
      * If the water ends in a lake, the fill flag is marked, and the amount of water is recorded.
      * If the water ends in a pixel with a non-null slope, a flag is marked that the flowage is not complete and the water is put into a flowing value.
      * If the water can not flow, because all pixels are higher than it (slope is null), then a flag is marked that filling is necessary, and the amount of water is recorded.
    * Fill step: only if the 'fill' flag is marked.
      * Fill flag is turned off.
      * scan through the pixels for pixels with 'filling' values.
      * First, perform a small flood-fill to find all of the neighboring pixels that are the same elevation, or who are a lake with the same elevation.
      * If, during the process of this flood-fill, an outlet is found (a pixel with a lower elevation), then:
        * modify the slope direction for all pixels so that they point towards this outlet, so future iterations no longer fill this lake, but flow through.
        * If multiple outlets are found, the slope for the pixels should point to the closest one.
      * Distribute the amount of water included throughout these pixels evenly and mark this as a lake (if it isn't already), providing it's lake elevation. Even if an outlet was found, a lake is still forming here, any more water added will simply go out the outlet.
    * Next: if the flow flag is on, repeat the loop. In theory, the fill step should never leave any water that hasn't filled.
  * Go through the flowage, and everything over a certain threshold should be marked as a river. I'm not sure how to calculate this threshold.
  * Then, turn the raster rivers into vector flow lines, creating our first vector layer from this process. This involves connecting pixels that are marked as rivers, based on slope. It could get complext. *Maybe this is a separate command, or part of import-environment*
  * Again, this should take input lake and river files, as well as existing slope files, to allow this to be repeated several times.
* `raster-water-alt`: This algorithm simplifies this even more. It might be broken up into separate steps, allowing you to repeat the lake filling if you want to.
  * This version breaks it down into three steps: flow, fill and flow. Instead of making a cycle of that. 
    * It's very difficult to calculate lakes correctly, since they fill and dry over time, and may be based on weather in the past, not the amount of precipiation it's getting *now*. For example, would the Aral Sea even exist if it hadn't been wetter in the past? How much actual less rain did it take to turn Lake Bonneville into Great Salt Lake? It might be easier to come up with a system of setting lake levels based on precipitation and temperature, and a little randomness. 
    * In addition, once lakes form, they are unlikely to create more lakes, because the amount flowing out (if any) is just going to equal the amount flowing in, not that much more. At most it increases the flow of a river.
    * This would reduce the complexity of the system and get rid of potential infinite loops.
  * Requires precipitation.
  * First, slopes and slope angles are calculated for all land. Pixels with no lower neighbors have a null slope. Equal elevations are not lower.
  * Second, precipitation across a whole year is calculated and laid down on each pixel as "water".
  * Mark the flow flag as on.
  * Loop while flag is on.
    * Flow step:
      * Flow flag is turned off.
      * Water in every pixel is moved down the slope to the next pixel. It is added to any that has already been added from another pixel (but not to the precipitation flow)
      * Yes, there can be branches, but they're more unlikely in this algorithm, I think.
      * If the water ends in an ocean, it is done.
      * If the water ends in a lake, the fill flag is marked, and the amount of water is recorded.
      * If the water ends in a pixel with a non-null slope, a flag is marked that the flowage is not complete and the water is put into a flowing value.
      * If the water can not flow, because all pixels are higher than it (slope is null), then a flag is marked that filling is necessary, and the amount of water is recorded.
  * Fill step: only if the 'fill' flag is marked. -- **This is not a loop, it's only done once**
      * scan through the pixels for pixels with 'filling' values.
      * Once found, look around at the size of the basin... how far out do you have to go to find a pixel that starts going down. Remember that level, then follow it down and then up again. If you run into an ocean while flowing down, then the basin is the previous level remembered. If you don't, remember the next outlet.
      * *Another possibility is that I actually have a separate step called **find-basins*** that finds the basins around these fill marks. 
      * Based on this value, precipitation, temperature, guesstimate a lake level. Basically, if the precipitation is high enough and the temperature low enough, then the basin gets filled, otherwise it becomes endorheic. 
      * **An option might be to let the user decide these elevations and put it in a different step**.
      * Do a flood-fill to generate the lake. Look for outlets: areas which are at the elevation of the lake (which might already be determined).
      * Add up all the filling values that are now covered by the lake. Divide them up for each outlet and mark as 'flowing', and turn on the flow flag.
      * If there are no flows, mark the lake as endorheic.
  * Second flowage: re-run the flow loop from before, but ignore any filling flags, and pass flowage into lakes on to their outlets.
  * Go through the flowage, and everything over a certain threshold should be marked as a river. I'm not sure how to calculate this threshold.
  * Then, turn the raster rivers into vector flow lines, creating our first vector layer from this process. This involves connecting pixels that are marked as rivers, based on slope. It could get complex. *Maybe this is a separate command, or part of import-environment*
  * Again, this should take input lake and river files, as well as existing slope files, to allow this to be repeated several times.
* `raster-glaciers`: I'm not sure if this is something I want to do, but basically, it's similar to the water thing, but takes precipitation that falls in temperatures below 0 and converts it into snow, then figures out how much melts during the rest of the year. If some lasts throughout the year, then it accumulates into a glacier. Then the cycle is repeated, with pixels along the edge of the glacier automatically being included in that below 0 even if it's not their natural temperature. The glacier also shifts down one slope each cycle.
* `raster-climate`: If I've done everything right, I should have real-world style measurements, which I can use to classify as Koppler zones.
* `raster-biome`: I'm not sure if there are any classification systems for these, or if I should just based them off of Koppler zones.
* `import-environment`: This is finally where we pull the raster data back into the vector data. This might be a set of commands. It gets the climate, biome, waterflow, rivers, lakes and oceans and pulls that into the tiles. The climates and biomes are spread out through the tile according to the predominant intersection. The rivers and lakes, however, are simply traced into vector lines. This allows lakes to exist smaller than tiles.
  * There will still be some smoothing of rivers and lakes and biome/climate zone boundaries. They were pixelated before, so not only do they need to be curved, but vertices need to be removed to bring them closer to the fantasy scale. However, it might also be nice to keep them as they were, in case you did your own smoothing, so that the world looks a little more natural later.
  * Part of this process, or another command, is going to re-tile everything, and I'm not sure exactly how to do that. But the goal is to split tiles that rivers go through, and cut lakes and oceans out of the land tiles. Tiles that are cut this way that become significantly below average may get merged into neighboring tiles that aren't across the water. 
    * One issue with this is when a river starts in the middle of a tile. If I digitize the rivers correctly, however, this might not be a huge problem, because I can find their source and end points (have to be careful that the source isn't a branch, though). If I discover that the start point is inside a tile, then I can follow that to where it intersects with the border of a tile before splitting the tiles.
    * Another issue are small lakes entirely inside tiles. These should be easy to find as well with a 'contains' operation, so they can stay and not split tiles also. These sorts of lakes would be too small to make ports, though.
    * This system may change the political processes slightly, as there now can be no land tiles that are also lake tiles. And, to figure out if there is a river, I can't just go with waterflow. In fact, I might need a "River-Neighbor" field, or maybe more data in the neighbors field that specifies whether the neighbor is accross a river. (I can look at the data itself to see if it's land, lake or water)
    * *One option*: there is no reason the tiles for the political processes need to be voronoi, except that there might be some assumptions I need to work with. I could take the vertices of the rivers, lakes and oceans, plus some random scattered points (trying not to set points too near those water features, maybe a buffer), and simply create Delaunay Triangles, and use triangle tiles. I really think this might make worlds that look nicer anyway, since the voronoi leads to almost a hex look and feel.





