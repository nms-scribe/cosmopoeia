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

# Tasks

To proceed on this, I can break it down into the following steps:

[X] Command line application that handles commands and configuration (start with "version" and "help"). 
[X] Need Usage/Documentation as well..
[X] `convert-heightmap` command -- Doing this first allows me to play with pre-existing terrains for the rest of it.
    [X] Create Points
    [X] Delaunay Regions
    [X] Voronoi Regions
    [X] validate the output
    [X] compare to QGIS voronoi functions
        [X] One option that might work: 
            [X] Make sure there are four "infinity" points (when generating the delaunay) at (-width,-height),(-width,2*height),(2*width,2*height),(2*width,-height)
            [X] Clamp the random points to within the extents.
            [X] When we get to the voronoi
                [X] check if site is outside the extent, and don't generate a voronoi for that.
                [X] Otherwise, check if any vertices of the voronoi are outside the extent, and if so clip them (create an extent geography and use an intersection, perhaps? Or, just find the intersection and do it?)
    [X] Need density instead of spacing. Right now, spacing doesn't increment across the map, especially if you specify it. There are other problems with that, but that's at least one of them. 
    [X] Trim voronoi to within the extent (will require an extent on the voronoi dev command).
        [X] Okay, I need to move the border points along the border instead of outside it, otherwise, I can't sample for voronoi on the edge.
        This broke the voronoi, I'm not certain why. I don't guarantee it's not a problem with my voronoi code, however. I'm going to revisit this tomorrow.
        What if I got rid of the borders, and just spread the points across and clamped them within the extents? -- See the thing about trimming below
        [X] If the boundary dots are all outside extent, and regulated, but the random are all inside, I believe we should be able to easily identify sites that don't belong. Then, if circumcenter is outside, we can just find an intersection between the line and the extent edge.
    [X] Figure out neighbors. The idea of tracking triangles doesn't work, at least partly because the circumcenter of a triangle may not be inside the triangle. So, I'm going to have to do a separate process for getting the neighbors.
        * I believe this is a close enough answer, and as it is probably using gdal, it works: https://www.qgistutorials.com/en/docs/find_neighbor_polygons.html
        * Essentially, you iterate through each feature, create a bounding box around it, then look for features that fit inside the bounding box, and check
          each of those if they are not disjoint.
        * There's a function called set_spatial_feature on the layer that can help.
        * As this works closely with GDAL (I'm not going to implement it using my own types), it might be a function on the tile layer.
    [X] Sample heights from heightmap
    [X] add Ocean Mask option vs ocean elevation.
[X] `gen-climate` command
    [X] Review AFMG climate generation algorithms and add them -- we'll wait on improved algorithms until later
    [X] Generate temperatures
    [X] Generate wind directions
    [X] Generate precipitation
[X] `gen-water` command
    [X] Review AFMG river generation algorithm
    [X] Generate flow
    [X] Fill lakes
    [X] Connect Rivers
    [X] Make connected rivers into bezier curves
        [X] Need to work on the bezier function -- what data type should be returned?
        [X] Need to work on polylinization of the bezier curve.
    [X] Draw lake polygons
    [X] Draw river polygons - This can all be done in QGIS
[X] `gen-biomes` command
    [X] Review AFMG biome generation algorithms
    [X] Create command (requires water, temperature, precipitation, rivers and lakes)
[X] `gen-people` command
    [X] various auxiliary files
    [X] Review AFMG people generation algorithms -- again, wait on improvements until later
    [X] Figure out how to break the task apart into sub commands and create those commands.
[X] civilization commands:
    [X] `gen-create-towns`
    [X] `gen-populate-towns`
    [X] `gen-create-nations`
    [X] `gen-expand-nations`
    [X] `gen-normalize-nations`
    [X] `gen-create-subnations`
    [X] `gen-expand-subnations`
    [X] `gen-fill-empty-subnations`
    [X] `gen-normalize-subnations`
    [X] `gen-towns`
    [X] `gen-subdivisions` -- wraps up all of the subdivision commands
    [X] `gen-nations` -- wraps up all of the nation commands
[X] curve-borders commands
    [X] `gen-biomes-dissolve`
    [X] `gen-people-cultures-dissolve`
    [X] `gen-nations-dissolve`
    [X] `gen-subnations-dissolve`
    [X] `gen-biomes-curvify`
    [X] `gen-cultures-curvify`
    [X] `gen-nations-curvify`
    [X] `gen-subnations-curvify`
    [X] add the above to `gen-biomes`
    [X] add the above to `gen-people`
    [X] add the above to `gen-nations`
    [X] add the above to `gen-subnations`
[X] I need colors for the thematic maps. I can then change QGIS to get the color from that field.
[X] `create-terrain` commands
    [X] terrain template files
    [X] Review AFMG terrain generation algorithms
[X] Replace 'with_insertor' with a callback in the indexing functions.
[X] Improve reproducibility by iterating layer features in the insertion order.
[X] Speed up the shore_distance algorithm by using the cost-expand process as with cultures, states, etc.
[X] `big-bang` command
[X] Get rid of "namer" defaults. This removes the weird dependency on the compression utility, reduces the binary size, and removes some of the command line options that complicate things. Instead, provide my namerset alongside the application.
[X] I need some default QGIS project with some nice styles and appearance which can be saved to the same directory. Any way to specify the filename when we create it (how difficult is it to "template" the project)? Or, do we come up with a standard file name as well?
    [X] I could just provide a QGIS file called World.qgz with defaults, and as long as the user places this in the same directory and calles their file 'world.gpkg' then it will work. If they want to rename things, they'll have to change the data source.
    [X] If I do do that, then I will need to edit the qgs file inside the qgz to remove the use of Inannak in the xml id values.
[-] Hide the sub-commands from help, but document them with an appropriate option flag on help. -- I wonder if this might work if I can do nested subcommands, but with defaults? Then maybe I could only display them when they do help on the other command. I've decided these are going in the wiki instead.
[X] Clean up and triage TODOs and FUTURE:s into things I must do now, simple things I could do now, and things that can wait until sometime later.
[X] Work on "Simple Pre-release Tasks" below
[X] Work on "Complex Pre-release Tasks" below
[X] Documentation
    [-] It's possible to have a separate executable in a bin/<name>.rs file which would give alternate access to code. Create a cosmopoeia-docs command there, perhaps. This will auto-generate some of the docs to a docs subfolder. Should look at github to see where the best place to put this is, though.
    [X] I need to document:
        [X] Command line: There are clap-markdown crates or something like that
        [-] Database Schema: I might could put some rudimentary "Schema description" function in the layer macro which let's me write the documentation metadata to a function.
            [X] Replace the current "Serialization Documentation" with a simple PropertyType Documentation trait that would be impld on every field type.
            [X] In fact, if I had a TypedField trait I could possibly get away with getting rid of all the get_field... macros
                [X] implemented GetTypedField for f64, test and then finish the others
                [X] SetTypedField is a separate trait because we can use references to set some values, so we can implement for Borrowed
                [X] Add get_field_value for SetTypedField?
        [X] Terrain recipes should use the enum tag under a "task" key alongside their properties, instead of the currently object key form. Same for the other ones if they're not already.
        [-] I might need to do something similar to what I'm doing for serialization here. Call it JSONDoc or something, where I have to specify field names and enum variants in a macro in order to get what I want, using a trait.
            [X] I think I can use the following: https://docs.rs/schemars/latest/schemars/
                [X] doc tags are under metadata.description
            [X] There might be tools to generate markdown, search for on internet: json schema to markdown
            [X] Terrain recipe file: Although this is partly done in the command line help, I need alternative information for the JSON file
            [X] Namer files
            [X] Culture set files
        [X] The QGIS file?
        [X] How to get started
    [-] The docs command should write each of the docs to a separate directory.
    [ ] Can I set it up so it shows up as a wiki on github?
    [-] Need to edit the content on all of the above.
    [X] Include a caveat that this is not intended to be used for scientific purposes (analyzing streams, etc.) and the algorithms are not meant to model actual physical processes.
    [X] Include a note that unlike it's predecessors, there are certain things I won't touch, like Coats of Arms, random zones and markers. There has to be a point where your imagination gets to take over, otherwise there is no real purpose for this tool. It's predecessors were designed for generating lots of random maps over and over. This is designed for generating a few maps, which you can then modify in external tools as you need.
    [-] Make sure it's clear that, although the algorithms were inspired by AFMG, the tool is not guaranteed to, and indeed not designed to, behave exactly the same in regards to output given the same output parameters.
    [X] Add notes on layer id fields to documentation:
[X] Break world_map module into several modules:
    [X] typed_map: everything for typing a map
        [X] fields
        [X] features
        [X] entities
        [X] geometry (move this over)
        [X] layers
        [X] dataset? do I have something for this?
    [X] world_map:
        [X] field_types: all of the custom field types
        [X] layers: all of the layers
        [X] mod: the WorldMap construct itself.
    [X] fix uses in the new modules.
[X] As I have approval, move the recipes and things like that from AFMG into /share
[-] Consider, before final, a change to the way lakes fill. Instead of filling up to a level based on accumulation, always fill up to the lowest tile that has a neighbor with a lower elevation on the other side. This should create more interesting lakes than the little potholes I've got now.
    [-] What if there are more than one tile thus? they probably both outflow.
    [-] If the tile is already an outflow from another lake, it doesn't count, allowing us to fill in a deeper lake that encompasses the other.
    [-] In order to get endorheic lakes: if the biome of the accumulation is arid, then the lake is reduced to about a quarter of its depth and recalculated (any "swallowed" lakes also have to be recalculated). If it's a semi-arid biome (grassland, tundra?, etc.) then it can go up to half of its depth. This isn't really scientific, but it is a little more versimilar.
[X] clippy
[X] Need some sample screenshots
[X] Need a readme, that includes the top part of the introduction.
[X] Figure out a license, possibly MIT, but I wouldn't mind something a little more...
[X] Set up private github repository for now, with docs...
[X] Turn on #![warn(clippy::cargo_common_metadata)] and fix those warnings
[ ] Deployment routine:
    What needs to be done on a new release:
        - run clippy
        - run cargo test
        - run docs subcommand
        - make sure there's something in changelog
        - create version tags and commits in git
        - run cargo build --release
        - create deployment executables
    [X] Create a pre-flight script (can I use rust-script?)
        [X] Run cargo clippy and return error if failed
        [X] Run cargo test and return error if failed
        [X] check changelog for data under [Unreleased], return error if empty
        [X] Run `cargo run docs` to generate documentation in the generated folder
    [ ] Can't get cargo release done, so I need to complete the same steps:
        [ ] Check if any changes are unstaged
        [ ] Check if any staged changes are not committed
        [ ] Check if there are any remote code that isn't fetched.
        [ ] edit Cargo.toml to increase version
        [ ] git commit with a message indicating a new release with the version info
        [ ] git tag with an appropriate version number
        [ ] git push (`git push --atomic <remote-branch> refs/tags/v1.0.0`?)
        [ ] https://github.com/crate-ci/cargo-release
        [ ] Maintain changelog: https://github.com/crate-ci/cargo-release/blob/master/docs/faq.md#maintaining-changelog
        [ ] Make sure I *absolutely do not publish to crates.io*
        [ ] Make it run the pre-flight script in pre-release-hook, so that I can't accidently run this.
    [ ] Configure deployment
        [ ] https://crates.io/crates/cargo-aur
        [ ] https://crates.io/crates/cargo-deb
    [ ] Set up a "deployment" script:
        [ ] Run cargo release (with pre-release-hook)
        [ ] publish to github if it's not done automatically
        [ ] run `cargo build --release`
        [ ] run cargo-aur and cargo-deb
        [ ] Anyway to automate upload of these things to github?
[ ] Put the Post-Release Tasks into issues on github
[ ] Make the github repository public.
[ ] Announce beta release on Blog, Mammoth, Reddit (AFMG list, imaginarymapping, maybe the rust forums?), and start updating those places when changes are made.

The following additional tasks need to be triaged:

## Simple Pre-release Tasks
They're simple in concept, that doesn't mean they won't lead to hours of refactoring work.

[X] Remove #[allow(dead_code)] and see what we should get rid of.
[X] Set the CRS for the dataset and each layer on create.
[X] Consider adding a new 'u64_string' or 'id_ref' type for typed features, and storing id foreign keys as string, so that they can be parsed into u64 for entity stuff, instead of using as u64 everywhere on lookups. Since I don't do anything in the map project, there shouldn't be anything with changing the types.
    [X] NationSchema should have 'capital_town_id' instead of 'capital' field. In fact, make sure all schemas use that nomenclature of <purpose>_<layer>_id, or at least <layer>_id if the purpose is obvious.
[X] The special values in WorldMap, in fact anywhere where we implemtn TryFrom<String>, should use serde_json instead. Use a json standard for all of that.
[X] Wherever I'm using serde_json, 'use' the functions as to_json_string and from_json_str, because I'm a little scared of using modules without use.
[X] Move the BiomeFeature consts and associated types into an impl of BiomeSchema
[X] Replace the BiomeLayer::build_lookup with TypedFeatureIterator::to_named_entities_index
[X] MarkovGenerator::calculate_chain -- should return an error if the array is empty, or if there are no strings in it. Because that will cause an error when we're trying to create strings.
[X] Check all 'unwraps' and make sure we don't need to throw an error there instead.
    [X] TilePreference::get_value -- there are a bunch of 'unwraps' that should be proper errors, since that's all based on user input.
    [X] In calculate_coastline, after the 'difference' function is done on the ocean, it can potentially return null. Make this an error instead. I think it's only there because of the GEOS requirement.
    [X] Arithmetic overflow is only a panic on debug builds. See if I can make it part of the release as well, I don't want to accidentally wrap values.
[X] Get rid of HashMap/HashSet::entry calls, replace with get and get_mut. This means I don't need to clone the key unless I actually add the value. Also, as long as 'None' is done first in the match, I need to do less type specification.
[X] Allow the user to use CultureSet::make_random_culture_set and CultureSet::make_random_culture_set_with_same_namer when generating cultures.
    [X] Do this by adding a way of specifying random values for cultures.
[X] Can I make the Cultureset serialization allow a number instead of a CultureSet or CultureSource, to simpley generate that many random cultures with random names?
[X] Since we need to load all of the namers to generate the cultures anyway, and the primary usage is intended to be the BigBang command, just load the namers automatically when namer sets are read from the file, everywhere. It doesn't take *that much time* to load them, and they will still only be loaded in steps that need them.
[X] Default namer should be an option in all cases. If it's None, then a random namer will be chosen.
[X] grouping::calculate_grouping: replace the `table.keys().next().cloned().map(|first| table.try_remove(&first))` call with a call to IndexMap::pop. It was originally written this way because I was using HashMap which doesn't have a pop. Keep in mind that this would work backward, so check if that matters first.
    [X] Also, can this then become a queue_watcher?
[X] PointGenerator::make_point -- can I utilize Point::create_geometry in the function?
[X] Default values (`default_value`) for CLI arguments should be stored in a constant, so I can change them more easily.
    [-] I wonder if I could store help values for those arguments as well? What if I had a set of macros defining the argument fields?
    [X] Probably the easiest way I can think of to do this (macros are potentially possible but basically require me to parse a whole struct) is to make use of the #[clap(flatten)] attribute and just group the attributes into sets of standard attributes. This means additional member chaining, but it will work from a UI point of view. Go through each command and look for "shared" arguments. Some of them contain multiple arguments (wind directions, extents)
    [-] Keep in mind that the terrain commands, also have to be #[serde(flatten)]
[-] Consider if it makes sense to turn the TemperatureArg into an ArgRange
[X] Consider turning the WindsArg into a map of directions by latitude range
    [X] Test this if possible
[-] naming: is_ref_vowel -- do I have all the vowels? Is there a unicode class I could use?
[X] Upgrade rust to latest -- not that there are any features I think I need (although NonZero might be nice) but it might be a good idea to keep it up since my version is almost 9 months old now. But cargo add is supposed to be much faster.
[X] Look at other lints I might want to do
[X] Work through clippy warnings
[-] In ProcessTerrainTilesWithPointIndex, rethink unimplemented pattern. I can catch things at compile time if I don't implement those things.
[X] In layer!, to_field_names_values should be replaced with a New<FeatureName> struct and an 'add' function.


## Complex Pre-release tasks
These are things that really should be done before release, but they might take a bit of work to figure out.

[X] TypedGeometry -- similar to how I did TypedFeature, a TypedGeometry is a struct that can be created using TryFrom<Geometry>, and TryFrom<GeometryRef> (or two structs?). Each one is a specific type, not an enum, so you don't have to worry about whether a specific geometry supports many polygons, many points, etc. The type tells you.
   -- Do a search for references to Gdal's Geography type to find all the places that need to be changed first (ignoring things like geometry.rs which need those things)
   [X] 'algorithms/tiles.rs'
   [X] 'algorithms/curves.rs' is next.
   [X] 'utils.rs'
   [X] 'world_map.rs' to finish things up
   [X] Fix every identifier that I've appended 2 to, should be all in World_map.
   [-] Make sure to get rid of the None geometry when I've got everything moved over. Have to figure out what to do with the TypedFeature implementation if there are no geometries. Might need another trait to implement there. Or, maybe I do keep a None? It will just raise an error if I try to create it.
   [-] bezierify_multipolygon should be able to call bezierify_polygon
   [X] make_valid_default should be part of areal_fns!, and should always return a variantarealgeometry.
[X] size_hint doesn't work the way I thought it did. The result is a range of how many remain instead. Fix it's usage and implementations.
[-] Consider converting to the nalgebra::Vector2 type for points.
    -- Won't do because 1) Vector2 is not hashable and 2) the Vector2 used by adaptive_bezier isn't the one I would get on my own.
    [-] Start by using Vector2 as the inner data for the Point structure, and see how much I can simplify things. If that works, then I can switch over to it.
    [-] Once I've got that figured out, see if I can make use of Vector2 from the more up-to-date version of nalgebra
[X] Move bezierify code into geometry (after switching to vector2, because then I don't have to retype the "Point" values.)
[X] The PolyBezier object shouldn't be shared, instead provide functions for bezierifying the lines.
[-] Colors are not reproducible with the same seed, can I fix this?
    [X] Can't do this with the current crate. Find a new one.
[X] Generate different colors for subnations
    [X] Colors should relate to their nation colors.
        [X] What if we did the coloring in the DissolveTiles algorithm? By the time we get there, we know how many there are going to be, and can generate colors as necessary. I can even supply a 'generate_color' method and let the biomes do their own thing. I would need to have a FeatureWithColor trait
        to apply the colors.
        [X] In addition to that, we still need to be able to specify the spacing based on an axis.
    [X] I may need to extract code from random color crate to make my own, I think. Or find a better crate.
[X] Use angular_units for directions and other things...
[X] Run clippy again.
[X] climate::generate_precipitation -- I think this will be improved if instead of just sending precipitation to one tile, I send it to all tiles within about 20-25 degrees of the wind direction. I'll have less of those "snake arms" that I see now. Split up the precipitation evenly, or perhaps weighted by how far the angle is from the degree. -- This would require switching to a queue thing like I did for water flow. -- but then we don't have the 'visited' set to check against. If a circle passes over water, it will infinite loop. What if I have a counter that decrements instead, stopping when we hit zero and passed along to the queue.
   [X] Try switching to elevation instead of elevation_Scaled in the functions.
[X] Play around with the temperature interpolation function in climate::generate_temperatures. I had some data figured out a long time ago with real-world interpolation. Hopefully I still have that around. Also, possibly calculate four seasonal curves and then take the average of those for the results.
    [X] Unfortunately, I did not save it. So I'll have to recreate it using another method.
[X] Test the precipitation with larger and smaller numbers of tiles.
[X] Go back and relook at biomes, there are way too many "wetlands". Maybe that's what's happened to my grassland?
[X] Cultures and nations spread much further then they should on my world-sized map. I'm not sure the limit_factor actually changes much. One thing I do need to change is add the area of the tile as a factor in determining expansion cost, to make sure that they expand less on smaller scale maps. And maybe that's all I have to do?
[X] Capital_count default should depend on the size of the map.
    [X] After calculations based on the older areas, there should be about 1 for 1,000 square degrees that the world covers. Limited by habitability, of course.
[X] Make cultures, nations, subnations fill lake tiles even if there is no population. I mean, I already allow them to spread through those tiles, but the tiles have to be marked with the culture to make sure there aren't weird holes in spots. At least get them out to -2. This just applies to lakes, I think.
[X] Okay, with the creation of a generated map, I am surprised to find a *lot* more basins than I expected. I just assumed my original Inannak just had a lot of craters. Maybe I do need to force rivers to flow out of sinks in certain situtations. I would also be okay with an 'erosion' terrain processor that cuts higher elevations down by moving things to lower slopes.
    The algorithm would work a lot like the flow algorithm, starting at the highest elevations and moving down. For each tile:
    * The tile entity tracks a temporary "soil" field.
    * Take a certain amount of elevation from the tile and add it to the soil field, representing weathering.
    * Find the lowest neighbor and calculate the grade of the slope to that neighbor.
    * Based on that slope, calculate a percentage which is higher the more of a slope there is. 
        I could just use percent grade, or just rise/run as a ratio. This would give me 100% loss on 45 degrees, however, and I feel that might be a little extensive. Halving it would give me 100% at about 63 degrees, quartering it is somewhere in the 70s. 
    * Multiply that percentage by the value of soil on that tile (keep in mind this includes soil added from higher tiles)
    * Subtract that amount of soil from the tile and add it to the lowest neighbor tile.
    * Once every tile has been iterated, go back and add that soil to the tile's elevation, (and update elevation_scaled? Unless that's done automatically by the terrain processor)
    * Input includes: amount to weather on every tile (which could be random), and possibly a number of iterations, which I don't think a random value would benefit. I might want a factor which defines how to calculate the percent based on slope, but I'm not certain right now how that's calculated anyway.
[X] Check with AFMG about appropriateness of copying, converting and reusing name sets, culture sets and terrain templates in other tools.
[X] Edges and Wrapping
    [X] Handle tiles that point to edges
    [X] Calculate edges for tiles in voronoi generation
    [X] Go back and find the 'if let Neighbor::Tile' and turn them into matches, so we can add more neighbor types, which we will in a bit
    [X] In tiles::calculate_tile_neighbors, add an "Edge" neighbor if the tile is on the edge of the map.
    [X] Test with both worlds before continuing
    [X] Revisit tiles::calculate_tile_neighbors: Except if the extents are "full world" extents, then need to calculate neighbors across the extents:
        [X] Such neighbors will be special "Wrapped" neighbors instead of Tile enum, which contains the 'id' and the edge.
        [X] East-west neighbors match if their 'edge' points would be on the edge of the other.
            [X] Neighbor directions have to be calculated over the edge of the map, not across to the other side.
            [X] I may need to do the same with distances.
    [X] Will have to deal with rivers that cross the map.
    [X] figure out if a tile is on the edge of a map -- and use this in water_flow and precipitation to prevent just dumping everything on the edge.
    [X] If we have edges calculated in calculate_tile_neighbors, then water_fill should take note of that and take appropriate action so there aren't any weird lakes along the edges.
    [X] Need to fix calculation of TileForTownPopulation::find_middle_point_between, because these neighbors won't necessarily share vertices.
        I can just put it on the "extent edge" of the tile if the neighbor is across map.
    [X] Now, work on the polar tiles.
        [X] I feel like the easiest way is to just not have any neighbors at all for the south and north directions. This does two things:
            1) I don't have to do anything anywhere
            2) It keeps features from flowing into the poles, which can make the map look weird even under an appropriate projection.
    [X] grouping::calculate_grouping -- I think knowing about edge tiles can help me solve a corner case in calculating whether an island is a lake_island or a continent if there are no oceans.
    [X] Coastlines and the thematic curvifications can extend over the edge of the map when curved. If I have knowledge of whether a tile is an edge tile, and where that edge is, then I can stop the curve at the points that are on the edge. Although, I might be able to just do an intersection with the extents.
[X] Can tile.outlet_from now become an Option<u64>?
[X] Play around with a simple serializer for the field data. The idea is a macro that outputs code like below, so you have to write the names of every field exactly, and the compilation will fail if you don't. This is not as good as a proc macro, but I will have control.
    ```
    let Foo{ field_a, field_b } = value
    ```
[X] separate utils into modules
[X] Change 'Point' to 'Coordinates', perhaps, and then get rid of UtilsPoint and GeoPoint
[X] clippy
[X] Is flow_to supposed to be a vector? Or optional?
[X] Namers: Figure out a way to get the mean length and a standard deviation while calculating the markov chain. Then, when generating words, use those values to generate the length of the output word. I feel that will be a lot closer to realistic names.
[X] map_err should pass the error on to the various things if it's found.


## Post-release tasks and feature requests.

[ ] Add a branching option to the water flow operation:
    [ ] Allow Branches --- current system
    [ ] Avoid Branches -- when the elevation is the same, pick only one in a reproducible manner
    [ ] Allow Branches on Flat Land -- allow branches if the elevation is the same and the flow_from elevation is very close to that elevation
[ ] Culture/Nation expansionism: some of these still expand way to far with higher tile numbers.
[ ] Add a spheremode option which causes points to be generated at higher spacing at higher latitudes and changes how distance and area are calculated where that's important.
    [ ] RasterBounds::pixels_to_coords and coords_to_pixels -- make sure these are calculated correctly for spheremode
    [ ] I may need to bring in my own delaunay algorithm. First, I wouldn't need to collect points into a geometry. But second, when I add sphere_mode, the changes to the distance formula might change. Third, I might be able to remove an array collection step in there, before generating voronoi. Not sure.
[ ] For schema documentation: If an enum type is an object, internally tagged, it should go into the definitions instead. Name it with the path to the enum from the parent. This is especially true for Recipe Set, where it would be nice to have a separate list of tasks.
[ ] More informational error handling: Make user of the following to add in information about where errors occur.
    * https://doc.rust-lang.org/std/macro.module_path.html
    * https://doc.rust-lang.org/std/macro.line.html
    * https://doc.rust-lang.org/std/macro.file.html
    * https://doc.rust-lang.org/std/any/fn.type_name.html with https://stackoverflow.com/questions/38088067/equivalent-of-func-or-function-in-rust
[ ] A trace log option might be nice, but it's kind of useless if the error occurs while you're not running it, and then doesn't when you turn it on.
[ ] Decide how to handle the following: It would be nice to avoid these common errors, but this will lead to a lot of errors that will stop the process anyway. I think, before I do something like this, I have to have more in-depth error handling, by adding locations and the like to error messages (there is a 'line!' macro that might be useful for that.)
    [ ] #![warn(clippy::arithmetic_side_effects)]
    [ ] #![warn(clippy::as_conversions)]
    [ ] #![warn(clippy::indexing_slicing)]
[ ] Consider a change to culture stuff. Right now we have two ways of specifying tile preference for culture and nation expansion: CultureType enum and the TilePreference enum. Play around with making the TilePreference option the standard, and at best create some pre-defined TilePreferences that are easier to serialize. **Or** replace the TilePreference with CultureTypes.
    [ ] If I do keep CultureType, figure out a better way that allows for more customization.
    [ ] If I can make the culture and biomes more configurable, then I can get rid of the 'supports_nomadic' and 'supports_hunting' fields on biomes.
[ ] Consider adding a 'status' property to the properties table, which specifies the last step completed during building. This can then be checked before running an algorithm to add some hope that the database is in the correct state. So, instead of getting a missing field error when running biomes before water, we get an immediate error that a step hasn't been completed.
    [ ] The status property would also make it easier to automatically finish a processing based on the last step completed. This makes 'regen' type commands easier -- you can just modify the biomes, set the status to biome, and go on from there.
    [ ] For that matter, there might also be an option to "bring it up" to a given status from whatever status it is, or from scratch.
[ ] Convert at least some of the algorithms into objects, with tile_maps and other preparations loaded in the constructor, and then various steps as methods. It would be nice if I could get it to similar patterns as the Terrain Processors, perhaps even implementing a load trait for the command arg structs and a run trait for the returned object.
[ ] Gdal has a nasty habit of printing out warnings and errors while processing. Is there any way to turn that off and turn them into actual errors?
[ ] rethink biome values so that they are more customizable. Right now there's a lot of hard-coded functionality, especially related to the TilePreference enum and culture/nation expansion algorithms.
[ ] In fact, a better climate generation scheme. I need seasonal values for temperature and precipitation. I need "accurate" precipitation equations to produce precipitation in mm/day or something like that.
    [ ] One thought on precipitation is to base it off of humidity being percent humidity, and if the humidity is pushed into a tile with different temperature and pressure (elevation), that percent changes, and if it goes above 100, we get precipiation. But how much?
    [ ] A better climate generation scheme would allow me to generate climates based on precipitation and temperature ranges (by season). It might be possible to use that for biomes instead.
[ ] Need a FillEmpty task on Cultures, just like provinces. Once cultures are generated, there should be no populated tiles that aren't part of a culture of some sort, even if I just have a 'wildlands' culture or something like that.
    [ ] If everything has cultures, then there shouldn't be any place which doesn't have one, in which case I will no longer need the default_namer argument on various commands.
[ ] Is there any way to create a function that will let me do the cost expansion algorithm with just a few closures for customization?
[ ] cultures::get_shore_cost and nations::get_shore_cost -- Lake and Naval have a penalty for going past -2 shore_distance. That doesn't seem right. Meanwhile, every other culture type gets a 0 for that area. 
[ ] naming::NamerLoadObsever::start_known_endpoint -- either make the count required to produce a progress bar configurable, or find a way of popping it up only if the code is taking longer than half a second or so
[ ] A lot of stuff where I'm opening a layer for editing, I'm only actually reading it. Unfortunately, the layer still has to be mutable in order to iterate features, but is there any way to open it read only? What if that would fix the problem that I patched with the 'reedit' function?
[ ] Where I use the word 'normalize' for getting rid of small bits of culture/nation/etc. I feel like that's the wrong word. See if it's the correct usage, and if not, rename to something better.
[ ] In a lot of places, when I use read_features().into_entities, I have to reassign the variable with a question mark to get rid of the error. Is there a better way of doing that?
[ ] AddRange::process_terrain_tiles_with_point_index and same in AddStrait -- Instead of processing separate queues in batches, I might be able to pass the next height_delta into the queue when I push the item on the queue. Then I can keep using the same queue, and can more easily add a queue_watcher.
[ ] Come up with a better algorithm for AddStrait (see note in function)
    [ ] If I don't rethink AddStrait completely, I feel like the exp in AddStrait creates straits that are too deep. Since I'm not utilizing elevation_scale for these, the values may be off.
[ ] A few "tectonic" patterns for terrain processing tasks: "Colliding Plates", "Mid-Ocean Ridge", "Transform Fault" I could get as far as generating plates, but I'm not sure I need that.
[ ] Consider moving the stuff for the grouping algorithm into calculate_coastline (which is also spreading through the tiles) and fill_lakes (which it would be simple to change grouping to lake, but I'm not sure about island_lake).
[ ] Certain culture types, such as Nomadic, shouldn't generate towns, or at least generate fewer towns. 
[ ] Consider allowing the Point, Delaunay and Voronoi generators to have a reference to the progressbar instance to update progress. This comes up because of the one part where the DelauneyGenerator has to pass an empty progress observer to the start method if not started ahead of time.
[ ] Look at smoothing algorithm and figure out what that fr property is supposed to be doing. If anything, replace with a real smoothing algorithm that just uses a weighted average.
[ ] grouping::calculate_grouping -- continent/island/isle threshold should depend on the size of the map, not the tile count.
[ ] naming.rs -- there are a bunch of patterns which are removed that should be dependent on the language. Figure out how to make this better.
[ ] MarkovGenerator::calculate_chain -- should use the chars directly for iterating through the name, instead of collecting into a vec of chars.
[ ] PointGenerator -- Why does START_Y have to start with 1, but START_X can start with 0?
[ ] terrain::Invert -- the algorithm is a little slow, but also it's not a true invert, which would involve actually moving the geometry of the tiles (but that's not available in the terrain processing toolkit, I would have to add that)
[ ] After curving, towns which are along the coastline will sometimes be in the ocean. May need to deal with that.
[ ] Technically, since lakes have an inset, it should be possible to have population on a lake tile, along the coast. But, if a town is set there, it has to be placed outside of the lake.
[ ] There's a double link that needs to be maintained with towns: the town has a tile_id, and the tiles have a town_id. If towns are randomly regenerated, I'm not sure if the old town_ids in the tiles are then cleared out.
[ ] `gen-transporation`
    [ ] `gen-transportation-roads`
    [ ] `gen-transportation-trails`
    [ ] `gen-transportation-ocean-routes` -- Make a note that ocean currents are not used here (unless I figure those out first)
    [ ] Update `gen-town` so that the population of towns are effected by connection to roads
[ ] `regen-*` commands
    [ ] Based on what is done in `gen-people` and some other things, but keep things that shouldn't be regenerated. -- Do I want to allow them to "lock" things? This almost has to be the same algorithms that I'm using. In which case, do I really need this? The only way this would be useful is if I could lock, because otherwise you could just redo some things.
[ ] Also a `regenesis` command that will let you start at a specific stage in the process and regenerate everything from that, but keep the previous stuff. This is different from just the sub-tasks, as it will finish all tasks after that. (This is related to the status thing I came up with before)
[ ] `image` terrain source which accepts an image and an extent in one dimension (the other is calculated by ratio), and a way of combining the different bands on the image.
[ ] Revisit the target.reedit problem in big_bang, see if I can get an MRE that causes the problem and track down the problem.
[ ] Revisit subnation curvify: the subnations should follow their nation borders when possible. This might be done more easily if we curvify the subnations first, then just dissolve the nations out of their subnations.
[ ] Review all of the algorithms to see if there are better ways
[ ] Any way to save the state of the random number generator so we can reload it later? This will be helpful in reproducing stuff while running separate steps.
[ ] `enhance!` command: This will take a map and *add* random points and tiles to it. Perhaps it will base the new points off of the intersections of the tile boundaries. Heights will be randomly generated for the new points, although within a certain range so that I can try to keep a similar slope, and try to keep the water flow going between the same two tiles. Water will probably not be recalculated, but rivers do need to be rerouted, and lakeshores reshaped. Towns and nations will stay the same, but the new tiles will be assigned by "spreading" the cultures, and nations out based on their neighbors and what they were before. "enhance" comes from the TV show trope where they "enhance" a blurry image and somehow get details out of it that the camera could never have picked up.
    [ ] Combine this with a `clip` command to extract a smaller extent from the map, and you have the submap command.
[ ] Curvy shapes could have a way of adding noise to the edges, depending on elevations and random knowledge: coastlines get bumpier curves around higher slopes, while rivers get more meanders on lower slopes.
[ ] How about a way to output SVG files from the generated file. The files would create black and white outlines, but the objects would be divided into layers for most fields, possibly with class fields if those are available now, which would allow easy styling in any SVG editor of your choice.
[ ] For wind directions: What if the wind is affected by the "slope" of the average temperatures -- the direction to the warmest temperature neighbor is averaged with the based direction (because warmer is usually lower pressure). This is closer to how prevailing winds work. Better would be to figure out how to do pressure.
[ ] Terrain task: Fill sinks -- how much should I fill? Do I only do it for single tiles that are lower? How about more?
[ ] Terrain task: cut through sinks: Calculate flow, and if it ends in a sink, it cuts the lowest neighboring tile (from a different direction) to just below the current. This should, in theory, create canyons through some highlands. There are many questions, such as: is there a limit to how much it will cut so we can still get Death Valleys and Caspian Seas? If it hits that limit, how do we backtrack and warn the algorithm not to try again? What happens if the cut just leads to another sink?
[ ] Add installation for windows: [https://crates.io/crates/cargo-wix]. I'm not sure how to get gdal dependency on windows. Is it a DLL? Where would I find it? How does the program find it at runtime?
