Highly inspired by [Azgaar's Fantasy Map Generator](<https://azgaar.github.io/Fantasy-Map-Generator/>) (AFMG).

This project would be a series of console commands, and possible QGIS extensions, and other tools which make it easier to generate fantasy maps using GIS data.

# Reasoning

AFMG maps are a rectangle full of voronoi cells around randomly generated points. Each cell has an elevation, plus a number of other attributes, which make all land within that cell fairly uniform. In some ways this resembles hexagonal grid maps of role-playing games, and if you had uniform placement of points, it would look exactly like that. However, the random points and the voronoi give a more organic look to the output. With appropriate line smoothing and styling, the resulting maps look very much like a traditional fantasy map.

The problem with AFMG is that it is a monolithic tool inseparable from the user interface built around it. While the creator has added a lot of features and customization to the system, additional features and customization are dependent on their schedule and vision. Due to its development as a browser application, it suffers from performance problems with very complex maps. 

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

For the most part, AFMG is open source, and I can use some of the same algorithms as a basis for my code. However, there are a few things I know I will change:

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

# AFMG Algorithms

## Temperature

```js
function calculateTemperatures() {
  TIME && console.time("calculateTemperatures");
  const cells = grid.cells;
  cells.temp = new Int8Array(cells.i.length); // temperature array

  const tEq = +temperatureEquatorInput.value;
  const tPole = +temperaturePoleInput.value;
  const tDelta = tEq - tPole;
  const int = d3.easePolyInOut.exponent(0.5); // interpolation function

  d3.range(0, cells.i.length, grid.cellsX).forEach(function (r) {
    const y = grid.points[r][1];
    const lat = Math.abs(mapCoordinates.latN - (y / graphHeight) * mapCoordinates.latT); // [0; 90]
    const initTemp = tEq - int(lat / 90) * tDelta;
    for (let i = r; i < r + grid.cellsX; i++) {
      cells.temp[i] = minmax(initTemp - convertToFriendly(cells.h[i]), -128, 127);
    }
  });

  // temperature decreases by 6.5 degree C per 1km
  function convertToFriendly(h) {
    if (h < 20) return 0;
    const exponent = +heightExponentInput.value;
    const height = Math.pow(h - 18, exponent);
    return rn((height / 1000) * 6.5);
  }

  TIME && console.timeEnd("calculateTemperatures");
}
```

Analysis:

* temp_equator = input (default 26, degrees in celsius, no noticeable limits)
* temp_pole = input (default -29)
* temp_delta = temp_equator - temp_pole
* int = `d3.easePolyInOut.exponent(0.5)` -- comments say interpolation function. But the docs talk about easing. It returns a function "which takes a normalized time t and returns the corresponding “eased” time tʹ". Further exploration indicates it is used to change the "speed" of animation over the time that the animation is taking place. So, in effect, it creates a "curve" function. The 0.5 is the exponent. 
  * This is the function returned from that, where e = 0.5 from the argument:
    ```js
    function polyInOut(t) {
      return ((t *= 2) <= 1 ? Math.pow(t, e) : 2 - Math.pow(2 - t, e)) / 2;
    }
    ```
  * `t` is supposed to be a value from 0 to 1. If t <= 0.5 (`(t *= 2) <= 1`) then the function above is `y = ((2x)^(1/2))/2`. If t is greater, then the function is `y = (2 - (2-x)^(1/2))/2`. The two functions both create a sort of parabola. The first one starts curving up steep at 0 (the pole) and then flattens out to almost diagonal at 0.5. The second one continues the diagonal that curves more steeply up towards 1 (the equator). I'm not sure whey this curve was chosen, I would have expected a flatter curve at the equator.
* for each tile (I'm assuming that's what the "range" function is doing here)
  * lat = latitude of cell
  * init_temp = temp_equator - int(lat/90) * temp_delta;
  * tile.temperature = clamp(init_temp - convertToFriendly(tile.elevation),-128,127)
* convertToFriendly(elevation) -- this is really just an adabiatic temperature reduction.
  * if is_ocean return 0; 
  * exponent = input (value from 1.5 to 2.2, configured in the Units dialog)
  * return round((elevation/1000)*6.5,2);
  * -- NOTE: Unlike AFMG, our temperature change can be negative as well. So, if you go below sea-level, it can get hotter.

This algorithm is fairly straightforward, even if I might disagree with some of it.

## Precipitation

```js
function generatePrecipitation() {
  TIME && console.time("generatePrecipitation");
  prec.selectAll("*").remove();
  const {cells, cellsX, cellsY} = grid;
  cells.prec = new Uint8Array(cells.i.length); // precipitation array

  const cellsNumberModifier = (pointsInput.dataset.cells / 10000) ** 0.25;
  const precInputModifier = precInput.value / 100;
  const modifier = cellsNumberModifier * precInputModifier;

  const westerly = [];
  const easterly = [];
  let southerly = 0;
  let northerly = 0;

  // precipitation modifier per latitude band
  // x4 = 0-5 latitude: wet through the year (rising zone)
  // x2 = 5-20 latitude: wet summer (rising zone), dry winter (sinking zone)
  // x1 = 20-30 latitude: dry all year (sinking zone)
  // x2 = 30-50 latitude: wet winter (rising zone), dry summer (sinking zone)
  // x3 = 50-60 latitude: wet all year (rising zone)
  // x2 = 60-70 latitude: wet summer (rising zone), dry winter (sinking zone)
  // x1 = 70-85 latitude: dry all year (sinking zone)
  // x0.5 = 85-90 latitude: dry all year (sinking zone)
  const latitudeModifier = [4, 2, 2, 2, 1, 1, 2, 2, 2, 2, 3, 3, 2, 2, 1, 1, 1, 0.5];
  const MAX_PASSABLE_ELEVATION = 85;

  // define wind directions based on cells latitude and prevailing winds there
  d3.range(0, cells.i.length, cellsX).forEach(function (c, i) {
    const lat = mapCoordinates.latN - (i / cellsY) * mapCoordinates.latT;
    const latBand = ((Math.abs(lat) - 1) / 5) | 0;
    const latMod = latitudeModifier[latBand];
    const windTier = (Math.abs(lat - 89) / 30) | 0; // 30d tiers from 0 to 5 from N to S
    const {isWest, isEast, isNorth, isSouth} = getWindDirections(windTier);

    if (isWest) westerly.push([c, latMod, windTier]);
    if (isEast) easterly.push([c + cellsX - 1, latMod, windTier]);
    if (isNorth) northerly++;
    if (isSouth) southerly++;
  });

  // distribute winds by direction
  if (westerly.length) passWind(westerly, 120 * modifier, 1, cellsX);
  if (easterly.length) passWind(easterly, 120 * modifier, -1, cellsX);

  const vertT = southerly + northerly;
  if (northerly) {
    const bandN = ((Math.abs(mapCoordinates.latN) - 1) / 5) | 0;
    const latModN = mapCoordinates.latT > 60 ? d3.mean(latitudeModifier) : latitudeModifier[bandN];
    const maxPrecN = (northerly / vertT) * 60 * modifier * latModN;
    passWind(d3.range(0, cellsX, 1), maxPrecN, cellsX, cellsY);
  }

  if (southerly) {
    const bandS = ((Math.abs(mapCoordinates.latS) - 1) / 5) | 0;
    const latModS = mapCoordinates.latT > 60 ? d3.mean(latitudeModifier) : latitudeModifier[bandS];
    const maxPrecS = (southerly / vertT) * 60 * modifier * latModS;
    passWind(d3.range(cells.i.length - cellsX, cells.i.length, 1), maxPrecS, -cellsX, cellsY);
  }

  function getWindDirections(tier) {
    const angle = options.winds[tier];

    const isWest = angle > 40 && angle < 140;
    const isEast = angle > 220 && angle < 320;
    const isNorth = angle > 100 && angle < 260;
    const isSouth = angle > 280 || angle < 80;

    return {isWest, isEast, isNorth, isSouth};
  }

  function passWind(source, maxPrec, next, steps) {
    const maxPrecInit = maxPrec;

    for (let first of source) {
      if (first[0]) {
        maxPrec = Math.min(maxPrecInit * first[1], 255);
        first = first[0];
      }

      let humidity = maxPrec - cells.h[first]; // initial water amount
      if (humidity <= 0) continue; // if first cell in row is too elevated consider wind dry

      for (let s = 0, current = first; s < steps; s++, current += next) {
        if (cells.temp[current] < -5) continue; // no flux in permafrost

        if (cells.h[current] < 20) {
          // water cell
          if (cells.h[current + next] >= 20) {
            cells.prec[current + next] += Math.max(humidity / rand(10, 20), 1); // coastal precipitation
          } else {
            humidity = Math.min(humidity + 5 * modifier, maxPrec); // wind gets more humidity passing water cell
            cells.prec[current] += 5 * modifier; // water cells precipitation (need to correctly pour water through lakes)
          }
          continue;
        }

        // land cell
        const isPassable = cells.h[current + next] <= MAX_PASSABLE_ELEVATION;
        const precipitation = isPassable ? getPrecipitation(humidity, current, next) : humidity;
        cells.prec[current] += precipitation;
        const evaporation = precipitation > 1.5 ? 1 : 0; // some humidity evaporates back to the atmosphere
        humidity = isPassable ? minmax(humidity - precipitation + evaporation, 0, maxPrec) : 0;
      }
    }
  }

  function getPrecipitation(humidity, i, n) {
    const normalLoss = Math.max(humidity / (10 * modifier), 1); // precipitation in normal conditions
    const diff = Math.max(cells.h[i + n] - cells.h[i], 0); // difference in height
    const mod = (cells.h[i + n] / 70) ** 2; // 50 stands for hills, 70 for mountains
    return minmax(normalLoss + diff * mod, 1, humidity);
  }

  void (function drawWindDirection() {
    const wind = prec.append("g").attr("id", "wind");

    d3.range(0, 6).forEach(function (t) {
      if (westerly.length > 1) {
        const west = westerly.filter(w => w[2] === t);
        if (west && west.length > 3) {
          const from = west[0][0],
            to = west[west.length - 1][0];
          const y = (grid.points[from][1] + grid.points[to][1]) / 2;
          wind.append("text").attr("x", 20).attr("y", y).text("\u21C9");
        }
      }
      if (easterly.length > 1) {
        const east = easterly.filter(w => w[2] === t);
        if (east && east.length > 3) {
          const from = east[0][0],
            to = east[east.length - 1][0];
          const y = (grid.points[from][1] + grid.points[to][1]) / 2;
          wind
            .append("text")
            .attr("x", graphWidth - 52)
            .attr("y", y)
            .text("\u21C7");
        }
      }
    });

    if (northerly)
      wind
        .append("text")
        .attr("x", graphWidth / 2)
        .attr("y", 42)
        .text("\u21CA");
    if (southerly)
      wind
        .append("text")
        .attr("x", graphWidth / 2)
        .attr("y", graphHeight - 20)
        .text("\u21C8");
  })();

  TIME && console.timeEnd("generatePrecipitation");
}
```

### Analysis:

NOTE: I think a lot of this depends on the cells being arranged in a grid form, but I'm not certain.

* let cells_number_modifier = (initial number of points/10000)^0.25 TODO: What is this for?
* let prec_input_modifier = (input value "Precipitation" 0-100)/100
* let modifier = cells_number_modifier * prec_input_modifier
  * Okay, what I believe this is calculating a "percent" of the cells that have precipitation.
* let westerly = []
* let easterly = []
* let southerly = 0
* let northerly = 0
* let latitude_modifier = [4, 2, 2, 2, 1, 1, 2, 2, 2, 2, 3, 3, 2, 2, 1, 1, 1, 0.5]
  * This bases itself on "bands" of rain. See the comments on this one above. Basically, represents things like ITCZ.
* let max_passable_elevation =  85
  * TODO: Now I'm wondering if the AFMG elevation scales aren't in meters.
* for each tile:
  * lat = tile.lat
  * lat_band = ((lat.abs() - 1) / 5).floor()
  * lat_mod = latitude_modifier[lat_band]
  * wind_tier = ((lat - 89).abs() / 30).floor(); 
    * Basically, like the lat_bnad, but the division is into five degrees
  * (is_west, is_east, is_north, is_south) = get_wind_directions(wind_tier); 
  * if is_west: westerly.push((tile.fid(),lat_mod,wind_tier))
  * if is_east: easterly.push((tile.fid(),lat_mod,wind_tier)) 
  * if is_north: northerly += 1
  * if is_south: southerly += 1
* if westerly.len > 0: pass_wind(westerly, 120 * modifier, 1, tile_count_x) 
* if easterly.len > 0: pass_wind(easterly, 120 * modifier, -1, tile_count_x) 
* let vert_t = southerly + northerly;
* if northerly > 0:
  * let band_north = ((northernmost_latitude.abs() - 1)/5).floor()
  * let lat_mod_north = (latitude_range > 60) ? mean(latitude_modifier) : latitude_modifier[band_north]
    * I don't know why they average it for wider worlds
  * let max_prec_n = (northerly / vert_t) * 60 * modifier * lat_mod_north;
  * pass_wind(cells,max_prec_n,tile_count_x,tile_count_y) 
* if southerly > 0:
  * let band_south = ((southernmost_latitude.abs() - 1)/5).floor()
  * let lat_mod_south = (latitude_range > 60) ? mean(latitude_modifier) : latitude_modifier[band_south]
    * I don't know why they average it for wider worlds
  * let max_prec_s = (southerly / vert_t) * 60 * modifier * lat_mod_south;
  * pass_wind(cells,max_prec_s,-tile_count_x,tile_count_y) 

* get_wind_direction(tier):
  * let angle = options.winds[tier] // This is part of the input
  * let is_west = angle > 40 && angle < 140;
  * let is_east = angle > 220 && angle < 320;
  * let is_north = angle > 100 && angle < 260;
  * let is_south = angle > 280 || angle < 80;
  * return (is_west,is_east,is_north,is_south)

* pass_wind(source,max_prec,next,steps)
  * let max_prec_init = max_prec
  * for first of source
    * if first[0] -- I think this is what happens if we've been given an array of arrays instead of just an array.
      * max_prec = (max_prec_init * first[1],255);
      * first = first[0];
    * let humidity = max_prec - tiles[first].elevation
    * if humidity <= 0 continue;
    * current = first
    * for s in 0..steps
      * if cells[current].temp < -5 continue;  // permafrost, no humidity change?
      * if cells[current].is_ocean // water cell
        * if cells[curren+next].is_ocean == false
          * cells[current+next].precipitation += Math.max(humidity / rand(10, 20), 1); // coastal precipitation
        * else
          * humidity = Math.min(humidity + 5 * modifier, maxPrec); // wind gets more humidity passing water cell
          * cells[current].precipitation += 5 * modifier; // water cells precipitation (need to correctly pour water through lakes)
        * continue
      * is_passable = cells[current + next].height < MAX_PASSABLE_ELEVATION
      * precipitation = is_passable ? get_precipitation(humidity,current,next) : humidity
      * cells[current].precipitation = precipitation;
      * evaporation = precipitation > 1.5 ? 1 : 0;
      * humidity = is_passable ? clamp(humidity - precipitation + evaporation, 0, max_prec) : 0
      * current += next;

* get_precipitation(humidity,i,n)
  * normal_loss = Math.max(humidity / (10 * modifier), 1); // precipitation in normal conditions
  * diff_height = Math.max(cells.height[i + n] - cells.height[i], 0); // difference in height
  * mod = (cells.height[i + n] / 70) ** 2; // 50 stands for hills, 70 for mountains
  * return clamp(normal_loss + diff * mod, 1, humidity))

The huge problem with this algorithm is that AFMG seems to be able to keep the tiles in a grid. I don't have that luxury. I do think splitting between north, south, east and west winds is a good idea, and might help me, but I'd still have to sort them into rows and columns.

Okay, in re-thinking this: I don't need to worry about the grid if I do have the "directions". For westerly and easterly winds, every tile that has an appropriate wind is added to an array. So, the process is run on each of these tiles, extending their humidity based on the directions. The main issue is only that their algorithms use the location on the grid to find the "next" cell. If I can have the angles to the neighbors, then I can use that to find the appropriate neighbor for a cell. So, basically:
* Any cell with wind gets added to a vector of cells to check.
* We do the calculations as above, but when looking for the next cell, we need to look in that cell's neighbor IDs, not just going along the array. We continue following that chain of cells until all of the humidity is used up. 
* We may also want to keep a list of "visited" tiles so that I don't revisit a tile in the path along the way. This prevents circles, just as a check.

## Biomes

```js
function defineBiomes() {
  TIME && console.time("defineBiomes");
  const {cells} = pack;
  const {temp, prec} = grid.cells;
  cells.biome = new Uint8Array(cells.i.length); // biomes array

  for (const i of cells.i) {
    const temperature = temp[cells.g[i]];
    const height = cells.h[i];
    const moisture = height < 20 ? 0 : calculateMoisture(i);
    cells.biome[i] = getBiomeId(moisture, temperature, height);
  }

  function calculateMoisture(i) {
    let moist = prec[cells.g[i]];
    if (cells.r[i]) moist += Math.max(cells.fl[i] / 20, 2);

    const n = cells.c[i]
      .filter(isLand)
      .map(c => prec[cells.g[c]])
      .concat([moist]);
    return rn(4 + d3.mean(n));
  }

  TIME && console.timeEnd("defineBiomes");
}

function getBiomeId(moisture, temperature, height) {
  if (height < 20) return 0; // marine biome: all water cells
  if (temperature < -5) return 11; // permafrost biome
  if (isWetLand(moisture, temperature, height)) return 12; // wetland biome

  const moistureBand = Math.min((moisture / 5) | 0, 4); // [0-4]
  const temperatureBand = Math.min(Math.max(20 - temperature, 0), 25); // [0-25]
  return biomesData.biomesMartix[moistureBand][temperatureBand];
}

let biomesData = applyDefaultBiomesSystem();

function applyDefaultBiomesSystem() {
  const name = [
    "Marine",
    "Hot desert",
    "Cold desert",
    "Savanna",
    "Grassland",
    "Tropical seasonal forest",
    "Temperate deciduous forest",
    "Tropical rainforest",
    "Temperate rainforest",
    "Taiga",
    "Tundra",
    "Glacier",
    "Wetland"
  ];
  const color = [
    "#466eab",
    "#fbe79f",
    "#b5b887",
    "#d2d082",
    "#c8d68f",
    "#b6d95d",
    "#29bc56",
    "#7dcb35",
    "#409c43",
    "#4b6b32",
    "#96784b",
    "#d5e7eb",
    "#0b9131"
  ];
  const habitability = [0, 4, 10, 22, 30, 50, 100, 80, 90, 12, 4, 0, 12];
  const iconsDensity = [0, 3, 2, 120, 120, 120, 120, 150, 150, 100, 5, 0, 150];
  const icons = [
    {},
    {dune: 3, cactus: 6, deadTree: 1},
    {dune: 9, deadTree: 1},
    {acacia: 1, grass: 9},
    {grass: 1},
    {acacia: 8, palm: 1},
    {deciduous: 1},
    {acacia: 5, palm: 3, deciduous: 1, swamp: 1},
    {deciduous: 6, swamp: 1},
    {conifer: 1},
    {grass: 1},
    {},
    {swamp: 1}
  ];
  const cost = [10, 200, 150, 60, 50, 70, 70, 80, 90, 200, 1000, 5000, 150]; // biome movement cost
  const biomesMartix = [
    // hot ↔ cold [>19°C; <-4°C]; dry ↕ wet
    new Uint8Array([1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 10]),
    new Uint8Array([3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 9, 9, 9, 9, 10, 10, 10]),
    new Uint8Array([5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 9, 9, 9, 9, 9, 10, 10, 10]),
    new Uint8Array([5, 6, 6, 6, 6, 6, 6, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 10, 10, 10]),
    new Uint8Array([7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 10, 10])
  ];

  // parse icons weighted array into a simple array
  for (let i = 0; i < icons.length; i++) {
    const parsed = [];
    for (const icon in icons[i]) {
      for (let j = 0; j < icons[i][icon]; j++) {
        parsed.push(icon);
      }
    }
    icons[i] = parsed;
  }

  return {i: d3.range(0, name.length), name, color, biomesMartix, habitability, iconsDensity, icons, cost};
}

function isWetLand(moisture, temperature, height) {
  if (moisture > 40 && temperature > -2 && height < 25) return true; //near coast
  if (moisture > 24 && temperature > -2 && height > 24 && height < 60) return true; //off coast
  return false;
}



```

### Analysis

* **Input: Biomes Table** Input is a table with the following columns:
  * key_name: string -- This is the name of the biome for referencing in the algorithm
  * built_in: bool -- Indicates if the biome is built-in. Built-in biomes should not be deleted, nor their key_name changed, or biome generation isn't going to work anymore.
  * habitability: integer - a value to be used later.
  * movement_cost: integer - a value to be used later.
* **Input: Biomes Matrix** This is a 2d array with x specifying hot to cold, and y specifying dry to wet. This is a lookup for what climate to put given a temperature and precipitation.
  ```
  // hot ↔ cold [>19°C; <-4°C]; dry ↕ wet
    matrix: [[&str; 26]; 5] =
    [["Marine", "Marine", "Marine", "Marine", "Marine", "Marine", "Marine", "Marine", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "hot desert", "taiga"],
     ["cold desert", "cold desert", "cold desert", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "savanna", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "taiga", "taiga", "taiga"],
     ["grassland", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "taiga", "taiga", "taiga"],
     ["grassland", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical seasonal forest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "taiga", "taiga", "taiga"],
     ["temperate deciduous forest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "tropical rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "temperate rainforest", "taiga", "taiga"]]
  ```

* for each tile
  * let temperature = tile temperature
  * let precipitation = tile precipitation
  * let elevation = tile elevation_scale
  * let moisture = if height < 20 ? 0 : calculate_moisture(tile)
  * tile.biome = get_biome_id(moisture, temperature, elevation)

* calculate_moisture
  * let moist = tile.precipitation
  * if (tile.is_river) moist += Math.max(tile.water_flux / 20, 2); 
  * let n = [moist, ...precipitation of all neighboring cells]
  * return 4 + n.average;

* get_biome_id(moisture,temperature,elevation)
  * if elevation < 20 return "ocean"
  * if temperature < -5 return "tundra"
  * if is_wet_land(moisture,temperature,height) return "wetland"
  * let moisture_band = ((moisture/5).floor()).min(4)
  * let temperature_band = (20 - temperature).clamp(0,25)
  * return biome_matrix[moisture_band][temperature_band]

* is_wet_land(moisture,temperature,height) 
  * if moisture > 40 && temperature > -2 && height < 25: return true // near coast -- TODO: Except that we don't know that.
  * if moisture > 24 && temperature > -2 && height > 24 && height < 60: return true -- TODO: Further from coast?
  * return false

**NOTE:** It appears that I need to have rivers and lakes before I can do this, which is part of the moisture.

## Lakes and Rivers

```js
  Rivers.generate();
  Lakes.defineGroup();
  Rivers.specify();

  window.Rivers = (function () {
  const generate = function (allowErosion = true) {
    TIME && console.time("generateRivers");
    Math.random = aleaPRNG(seed);
    const {cells, features} = pack;

    const riversData = {}; // rivers data
    const riverParents = {};
    const addCellToRiver = function (cell, river) {
      if (!riversData[river]) riversData[river] = [cell];
      else riversData[river].push(cell);
    };

    cells.fl = new Uint16Array(cells.i.length); // water flux array
    cells.r = new Uint16Array(cells.i.length); // rivers array
    cells.conf = new Uint8Array(cells.i.length); // confluences array
    let riverNext = 1; // first river id is 1

    const h = alterHeights();
    Lakes.prepareLakeData(h);
    resolveDepressions(h);
    drainWater();
    defineRivers();

    calculateConfluenceFlux();
    Lakes.cleanupLakeData();

    if (allowErosion) {
      cells.h = Uint8Array.from(h); // apply gradient
      downcutRivers(); // downcut river beds
    }

    TIME && console.timeEnd("generateRivers");

    function drainWater() {
      //const MIN_FLUX_TO_FORM_RIVER = 10 * distanceScale;
      const MIN_FLUX_TO_FORM_RIVER = 30;
      const cellsNumberModifier = (pointsInput.dataset.cells / 10000) ** 0.25;

      const prec = grid.cells.prec;
      const area = pack.cells.area;
      const land = cells.i.filter(i => h[i] >= 20).sort((a, b) => h[b] - h[a]);
      const lakeOutCells = Lakes.setClimateData(h);

      land.forEach(function (i) {
        cells.fl[i] += prec[cells.g[i]] / cellsNumberModifier; // add flux from precipitation

        // create lake outlet if lake is not in deep depression and flux > evaporation
        const lakes = lakeOutCells[i]
          ? features.filter(feature => i === feature.outCell && feature.flux > feature.evaporation)
          : [];
        for (const lake of lakes) {
          const lakeCell = cells.c[i].find(c => h[c] < 20 && cells.f[c] === lake.i);
          cells.fl[lakeCell] += Math.max(lake.flux - lake.evaporation, 0); // not evaporated lake water drains to outlet

          // allow chain lakes to retain identity
          if (cells.r[lakeCell] !== lake.river) {
            const sameRiver = cells.c[lakeCell].some(c => cells.r[c] === lake.river);

            if (sameRiver) {
              cells.r[lakeCell] = lake.river;
              addCellToRiver(lakeCell, lake.river);
            } else {
              cells.r[lakeCell] = riverNext;
              addCellToRiver(lakeCell, riverNext);
              riverNext++;
            }
          }

          lake.outlet = cells.r[lakeCell];
          flowDown(i, cells.fl[lakeCell], lake.outlet);
        }

        // assign all tributary rivers to outlet basin
        const outlet = lakes[0]?.outlet;
        for (const lake of lakes) {
          if (!Array.isArray(lake.inlets)) continue;
          for (const inlet of lake.inlets) {
            riverParents[inlet] = outlet;
          }
        }

        // near-border cell: pour water out of the screen
        if (cells.b[i] && cells.r[i]) return addCellToRiver(-1, cells.r[i]);

        // downhill cell (make sure it's not in the source lake)
        let min = null;
        if (lakeOutCells[i]) {
          const filtered = cells.c[i].filter(c => !lakes.map(lake => lake.i).includes(cells.f[c]));
          min = filtered.sort((a, b) => h[a] - h[b])[0];
        } else if (cells.haven[i]) {
          min = cells.haven[i];
        } else {
          min = cells.c[i].sort((a, b) => h[a] - h[b])[0];
        }

        // cells is depressed
        if (h[i] <= h[min]) return;

        // debug
        //   .append("line")
        //   .attr("x1", pack.cells.p[i][0])
        //   .attr("y1", pack.cells.p[i][1])
        //   .attr("x2", pack.cells.p[min][0])
        //   .attr("y2", pack.cells.p[min][1])
        //   .attr("stroke", "#333")
        //   .attr("stroke-width", 0.2);

        if (cells.fl[i] < MIN_FLUX_TO_FORM_RIVER) {
          // flux is too small to operate as a river
          if (h[min] >= 20) cells.fl[min] += cells.fl[i];
          return;
        }

        // proclaim a new river
        if (!cells.r[i]) {
          cells.r[i] = riverNext;
          addCellToRiver(i, riverNext);
          riverNext++;
        }

        flowDown(min, cells.fl[i], cells.r[i]);
      });
    }

    function flowDown(toCell, fromFlux, river) {
      const toFlux = cells.fl[toCell] - cells.conf[toCell];
      const toRiver = cells.r[toCell];

      if (toRiver) {
        // downhill cell already has river assigned
        if (fromFlux > toFlux) {
          cells.conf[toCell] += cells.fl[toCell]; // mark confluence
          if (h[toCell] >= 20) riverParents[toRiver] = river; // min river is a tributary of current river
          cells.r[toCell] = river; // re-assign river if downhill part has less flux
        } else {
          cells.conf[toCell] += fromFlux; // mark confluence
          if (h[toCell] >= 20) riverParents[river] = toRiver; // current river is a tributary of min river
        }
      } else cells.r[toCell] = river; // assign the river to the downhill cell

      if (h[toCell] < 20) {
        // pour water to the water body
        const waterBody = features[cells.f[toCell]];
        if (waterBody.type === "lake") {
          if (!waterBody.river || fromFlux > waterBody.enteringFlux) {
            waterBody.river = river;
            waterBody.enteringFlux = fromFlux;
          }
          waterBody.flux = waterBody.flux + fromFlux;
          if (!waterBody.inlets) waterBody.inlets = [river];
          else waterBody.inlets.push(river);
        }
      } else {
        // propagate flux and add next river segment
        cells.fl[toCell] += fromFlux;
      }

      addCellToRiver(toCell, river);
    }

    function defineRivers() {
      // re-initialize rivers and confluence arrays
      cells.r = new Uint16Array(cells.i.length);
      cells.conf = new Uint16Array(cells.i.length);
      pack.rivers = [];

      const defaultWidthFactor = rn(1 / (pointsInput.dataset.cells / 10000) ** 0.25, 2);
      const mainStemWidthFactor = defaultWidthFactor * 1.2;

      for (const key in riversData) {
        const riverCells = riversData[key];
        if (riverCells.length < 3) continue; // exclude tiny rivers

        const riverId = +key;
        for (const cell of riverCells) {
          if (cell < 0 || cells.h[cell] < 20) continue;

          // mark real confluences and assign river to cells
          if (cells.r[cell]) cells.conf[cell] = 1;
          else cells.r[cell] = riverId;
        }

        const source = riverCells[0];
        const mouth = riverCells[riverCells.length - 2];
        const parent = riverParents[key] || 0;

        const widthFactor = !parent || parent === riverId ? mainStemWidthFactor : defaultWidthFactor;
        const meanderedPoints = addMeandering(riverCells);
        const discharge = cells.fl[mouth]; // m3 in second
        const length = getApproximateLength(meanderedPoints);
        const width = getWidth(getOffset(discharge, meanderedPoints.length, widthFactor, 0));

        pack.rivers.push({
          i: riverId,
          source,
          mouth,
          discharge,
          length,
          width,
          widthFactor,
          sourceWidth: 0,
          parent,
          cells: riverCells
        });
      }
    }

    function downcutRivers() {
      const MAX_DOWNCUT = 5;

      for (const i of pack.cells.i) {
        if (cells.h[i] < 35) continue; // don't donwcut lowlands
        if (!cells.fl[i]) continue;

        const higherCells = cells.c[i].filter(c => cells.h[c] > cells.h[i]);
        const higherFlux = higherCells.reduce((acc, c) => acc + cells.fl[c], 0) / higherCells.length;
        if (!higherFlux) continue;

        const downcut = Math.floor(cells.fl[i] / higherFlux);
        if (downcut) cells.h[i] -= Math.min(downcut, MAX_DOWNCUT);
      }
    }

    function calculateConfluenceFlux() {
      for (const i of cells.i) {
        if (!cells.conf[i]) continue;

        const sortedInflux = cells.c[i]
          .filter(c => cells.r[c] && h[c] > h[i])
          .map(c => cells.fl[c])
          .sort((a, b) => b - a);
        cells.conf[i] = sortedInflux.reduce((acc, flux, index) => (index ? acc + flux : acc), 0);
      }
    }
  };

  // add distance to water value to land cells to make map less depressed
  const alterHeights = () => {
    const {h, c, t} = pack.cells;
    return Array.from(h).map((h, i) => {
      if (h < 20 || t[i] < 1) return h;
      return h + t[i] / 100 + d3.mean(c[i].map(c => t[c])) / 10000;
    });
  };

  // depression filling algorithm (for a correct water flux modeling)
  const resolveDepressions = function (h) {
    const {cells, features} = pack;
    const maxIterations = +document.getElementById("resolveDepressionsStepsOutput").value;
    const checkLakeMaxIteration = maxIterations * 0.85;
    const elevateLakeMaxIteration = maxIterations * 0.75;

    const height = i => features[cells.f[i]].height || h[i]; // height of lake or specific cell

    const lakes = features.filter(f => f.type === "lake");
    const land = cells.i.filter(i => h[i] >= 20 && !cells.b[i]); // exclude near-border cells
    land.sort((a, b) => h[a] - h[b]); // lowest cells go first

    const progress = [];
    let depressions = Infinity;
    let prevDepressions = null;
    for (let iteration = 0; depressions && iteration < maxIterations; iteration++) {
      if (progress.length > 5 && d3.sum(progress) > 0) {
        // bad progress, abort and set heights back
        h = alterHeights();
        depressions = progress[0];
        break;
      }

      depressions = 0;

      if (iteration < checkLakeMaxIteration) {
        for (const l of lakes) {
          if (l.closed) continue;
          const minHeight = d3.min(l.shoreline.map(s => h[s]));
          if (minHeight >= 100 || l.height > minHeight) continue;

          if (iteration > elevateLakeMaxIteration) {
            l.shoreline.forEach(i => (h[i] = cells.h[i]));
            l.height = d3.min(l.shoreline.map(s => h[s])) - 1;
            l.closed = true;
            continue;
          }

          depressions++;
          l.height = minHeight + 0.2;
        }
      }

      for (const i of land) {
        const minHeight = d3.min(cells.c[i].map(c => height(c)));
        if (minHeight >= 100 || h[i] > minHeight) continue;

        depressions++;
        h[i] = minHeight + 0.1;
      }

      prevDepressions !== null && progress.push(depressions - prevDepressions);
      prevDepressions = depressions;
    }

    depressions && WARN && console.warn(`Unresolved depressions: ${depressions}. Edit heightmap to fix`);
  };

  // add points at 1/3 and 2/3 of a line between adjacents river cells
  const addMeandering = function (riverCells, riverPoints = null, meandering = 0.5) {
    const {fl, conf, h} = pack.cells;
    const meandered = [];
    const lastStep = riverCells.length - 1;
    const points = getRiverPoints(riverCells, riverPoints);
    let step = h[riverCells[0]] < 20 ? 1 : 10;

    let fluxPrev = 0;
    const getFlux = (step, flux) => (step === lastStep ? fluxPrev : flux);

    for (let i = 0; i <= lastStep; i++, step++) {
      const cell = riverCells[i];
      const isLastCell = i === lastStep;

      const [x1, y1] = points[i];
      const flux1 = getFlux(i, fl[cell]);
      fluxPrev = flux1;

      meandered.push([x1, y1, flux1]);
      if (isLastCell) break;

      const nextCell = riverCells[i + 1];
      const [x2, y2] = points[i + 1];

      if (nextCell === -1) {
        meandered.push([x2, y2, fluxPrev]);
        break;
      }

      const dist2 = (x2 - x1) ** 2 + (y2 - y1) ** 2; // square distance between cells
      if (dist2 <= 25 && riverCells.length >= 6) continue;

      const flux2 = getFlux(i + 1, fl[nextCell]);
      const keepInitialFlux = conf[nextCell] || flux1 === flux2;

      const meander = meandering + 1 / step + Math.max(meandering - step / 100, 0);
      const angle = Math.atan2(y2 - y1, x2 - x1);
      const sinMeander = Math.sin(angle) * meander;
      const cosMeander = Math.cos(angle) * meander;

      if (step < 10 && (dist2 > 64 || (dist2 > 36 && riverCells.length < 5))) {
        // if dist2 is big or river is small add extra points at 1/3 and 2/3 of segment
        const p1x = (x1 * 2 + x2) / 3 + -sinMeander;
        const p1y = (y1 * 2 + y2) / 3 + cosMeander;
        const p2x = (x1 + x2 * 2) / 3 + sinMeander / 2;
        const p2y = (y1 + y2 * 2) / 3 - cosMeander / 2;
        const [p1fl, p2fl] = keepInitialFlux ? [flux1, flux1] : [(flux1 * 2 + flux2) / 3, (flux1 + flux2 * 2) / 3];
        meandered.push([p1x, p1y, p1fl], [p2x, p2y, p2fl]);
      } else if (dist2 > 25 || riverCells.length < 6) {
        // if dist is medium or river is small add 1 extra middlepoint
        const p1x = (x1 + x2) / 2 + -sinMeander;
        const p1y = (y1 + y2) / 2 + cosMeander;
        const p1fl = keepInitialFlux ? flux1 : (flux1 + flux2) / 2;
        meandered.push([p1x, p1y, p1fl]);
      }
    }

    return meandered;
  };

  const getRiverPoints = (riverCells, riverPoints) => {
    if (riverPoints) return riverPoints;

    const {p} = pack.cells;
    return riverCells.map((cell, i) => {
      if (cell === -1) return getBorderPoint(riverCells[i - 1]);
      return p[cell];
    });
  };

  const getBorderPoint = i => {
    const [x, y] = pack.cells.p[i];
    const min = Math.min(y, graphHeight - y, x, graphWidth - x);
    if (min === y) return [x, 0];
    else if (min === graphHeight - y) return [x, graphHeight];
    else if (min === x) return [0, y];
    return [graphWidth, y];
  };

  const FLUX_FACTOR = 500;
  const MAX_FLUX_WIDTH = 2;
  const LENGTH_FACTOR = 200;
  const STEP_WIDTH = 1 / LENGTH_FACTOR;
  const LENGTH_PROGRESSION = [1, 1, 2, 3, 5, 8, 13, 21, 34].map(n => n / LENGTH_FACTOR);
  const MAX_PROGRESSION = last(LENGTH_PROGRESSION);

  const getOffset = (flux, pointNumber, widthFactor, startingWidth = 0) => {
    const fluxWidth = Math.min(flux ** 0.9 / FLUX_FACTOR, MAX_FLUX_WIDTH);
    const lengthWidth = pointNumber * STEP_WIDTH + (LENGTH_PROGRESSION[pointNumber] || MAX_PROGRESSION);
    return widthFactor * (lengthWidth + fluxWidth) + startingWidth;
  };

  // build polygon from a list of points and calculated offset (width)
  const getRiverPath = function (points, widthFactor, startingWidth = 0) {
    const riverPointsLeft = [];
    const riverPointsRight = [];

    for (let p = 0; p < points.length; p++) {
      const [x0, y0] = points[p - 1] || points[p];
      const [x1, y1, flux] = points[p];
      const [x2, y2] = points[p + 1] || points[p];

      const offset = getOffset(flux, p, widthFactor, startingWidth);
      const angle = Math.atan2(y0 - y2, x0 - x2);
      const sinOffset = Math.sin(angle) * offset;
      const cosOffset = Math.cos(angle) * offset;

      riverPointsLeft.push([x1 - sinOffset, y1 + cosOffset]);
      riverPointsRight.push([x1 + sinOffset, y1 - cosOffset]);
    }

    const right = lineGen(riverPointsRight.reverse());
    let left = lineGen(riverPointsLeft);
    left = left.substring(left.indexOf("C"));

    return round(right + left, 1);
  };

  const specify = function () {
    const rivers = pack.rivers;
    if (!rivers.length) return;

    for (const river of rivers) {
      river.basin = getBasin(river.i);
      river.name = getName(river.mouth);
      river.type = getType(river);
    }
  };

  const getName = function (cell) {
    return Names.getCulture(pack.cells.culture[cell]);
  };

  // weighted arrays of river type names
  const riverTypes = {
    main: {
      big: {River: 1},
      small: {Creek: 9, River: 3, Brook: 3, Stream: 1}
    },
    fork: {
      big: {Fork: 1},
      small: {Branch: 1}
    }
  };

  let smallLength = null;
  const getType = function ({i, length, parent}) {
    if (smallLength === null) {
      const threshold = Math.ceil(pack.rivers.length * 0.15);
      smallLength = pack.rivers.map(r => r.length || 0).sort((a, b) => a - b)[threshold];
    }

    const isSmall = length < smallLength;
    const isFork = each(3)(i) && parent && parent !== i;
    return rw(riverTypes[isFork ? "fork" : "main"][isSmall ? "small" : "big"]);
  };

  const getApproximateLength = points => {
    const length = points.reduce((s, v, i, p) => s + (i ? Math.hypot(v[0] - p[i - 1][0], v[1] - p[i - 1][1]) : 0), 0);
    return rn(length, 2);
  };

  // Real mouth width examples: Amazon 6000m, Volga 6000m, Dniepr 3000m, Mississippi 1300m, Themes 900m,
  // Danube 800m, Daugava 600m, Neva 500m, Nile 450m, Don 400m, Wisla 300m, Pripyat 150m, Bug 140m, Muchavets 40m
  const getWidth = offset => rn((offset / 1.5) ** 1.8, 2); // mouth width in km

  // remove river and all its tributaries
  const remove = function (id) {
    const cells = pack.cells;
    const riversToRemove = pack.rivers.filter(r => r.i === id || r.parent === id || r.basin === id).map(r => r.i);
    riversToRemove.forEach(r => rivers.select("#river" + r).remove());
    cells.r.forEach((r, i) => {
      if (!r || !riversToRemove.includes(r)) return;
      cells.r[i] = 0;
      cells.fl[i] = grid.cells.prec[cells.g[i]];
      cells.conf[i] = 0;
    });
    pack.rivers = pack.rivers.filter(r => !riversToRemove.includes(r.i));
  };

  const getBasin = function (r) {
    const parent = pack.rivers.find(river => river.i === r)?.parent;
    if (!parent || r === parent) return r;
    return getBasin(parent);
  };

  const getNextId = function (rivers) {
    return rivers.length ? Math.max(...rivers.map(r => r.i)) + 1 : 1;
  };

  return {
    generate,
    alterHeights,
    resolveDepressions,
    addMeandering,
    getRiverPath,
    specify,
    getName,
    getType,
    getBasin,
    getWidth,
    getOffset,
    getApproximateLength,
    getRiverPoints,
    remove,
    getNextId
  };
})();

window.Lakes = (function () {
  const setClimateData = function (h) {
    const cells = pack.cells;
    const lakeOutCells = new Uint16Array(cells.i.length);

    pack.features.forEach(f => {
      if (f.type !== "lake") return;

      // default flux: sum of precipitation around lake
      f.flux = f.shoreline.reduce((acc, c) => acc + grid.cells.prec[cells.g[c]], 0);

      // temperature and evaporation to detect closed lakes
      f.temp = f.cells < 6 ? grid.cells.temp[cells.g[f.firstCell]] : rn(d3.mean(f.shoreline.map(c => grid.cells.temp[cells.g[c]])), 1);
      const height = (f.height - 18) ** heightExponentInput.value; // height in meters
      const evaporation = ((700 * (f.temp + 0.006 * height)) / 50 + 75) / (80 - f.temp); // based on Penman formula, [1-11]
      f.evaporation = rn(evaporation * f.cells);

      // no outlet for lakes in depressed areas
      if (f.closed) return;

      // lake outlet cell
      f.outCell = f.shoreline[d3.scan(f.shoreline, (a, b) => h[a] - h[b])];
      lakeOutCells[f.outCell] = f.i;
    });

    return lakeOutCells;
  };

  // get array of land cells aroound lake
  const getShoreline = function (lake) {
    const uniqueCells = new Set();
    lake.vertices.forEach(v => pack.vertices.c[v].forEach(c => pack.cells.h[c] >= 20 && uniqueCells.add(c)));
    lake.shoreline = [...uniqueCells];
  };

  const prepareLakeData = h => {
    const cells = pack.cells;
    const ELEVATION_LIMIT = +document.getElementById("lakeElevationLimitOutput").value;

    pack.features.forEach(f => {
      if (f.type !== "lake") return;
      delete f.flux;
      delete f.inlets;
      delete f.outlet;
      delete f.height;
      delete f.closed;
      !f.shoreline && Lakes.getShoreline(f);

      // lake surface height is as lowest land cells around
      const min = f.shoreline.sort((a, b) => h[a] - h[b])[0];
      f.height = h[min] - 0.1;

      // check if lake can be open (not in deep depression)
      if (ELEVATION_LIMIT === 80) {
        f.closed = false;
        return;
      }

      let deep = true;
      const threshold = f.height + ELEVATION_LIMIT;
      const queue = [min];
      const checked = [];
      checked[min] = true;

      // check if elevated lake can potentially pour to another water body
      while (deep && queue.length) {
        const q = queue.pop();

        for (const n of cells.c[q]) {
          if (checked[n]) continue;
          if (h[n] >= threshold) continue;

          if (h[n] < 20) {
            const nFeature = pack.features[cells.f[n]];
            if (nFeature.type === "ocean" || f.height > nFeature.height) {
              deep = false;
              break;
            }
          }

          checked[n] = true;
          queue.push(n);
        }
      }

      f.closed = deep;
    });
  };

  const cleanupLakeData = function () {
    for (const feature of pack.features) {
      if (feature.type !== "lake") continue;
      delete feature.river;
      delete feature.enteringFlux;
      delete feature.outCell;
      delete feature.closed;
      feature.height = rn(feature.height, 3);

      const inlets = feature.inlets?.filter(r => pack.rivers.find(river => river.i === r));
      if (!inlets || !inlets.length) delete feature.inlets;
      else feature.inlets = inlets;

      const outlet = feature.outlet && pack.rivers.find(river => river.i === feature.outlet);
      if (!outlet) delete feature.outlet;
    }
  };

  const defineGroup = function () {
    for (const feature of pack.features) {
      if (feature.type !== "lake") continue;
      const lakeEl = lakes.select(`[data-f="${feature.i}"]`).node();
      if (!lakeEl) continue;

      feature.group = getGroup(feature);
      document.getElementById(feature.group).appendChild(lakeEl);
    }
  };

  const generateName = function () {
    Math.random = aleaPRNG(seed);
    for (const feature of pack.features) {
      if (feature.type !== "lake") continue;
      feature.name = getName(feature);
    }
  };

  const getName = function (feature) {
    const landCell = pack.cells.c[feature.firstCell].find(c => pack.cells.h[c] >= 20);
    const culture = pack.cells.culture[landCell];
    return Names.getCulture(culture);
  };

  function getGroup(feature) {
    if (feature.temp < -3) return "frozen";
    if (feature.height > 60 && feature.cells < 10 && feature.firstCell % 10 === 0) return "lava";

    if (!feature.inlets && !feature.outlet) {
      if (feature.evaporation > feature.flux * 4) return "dry";
      if (feature.cells < 3 && feature.firstCell % 10 === 0) return "sinkhole";
    }

    if (!feature.outlet && feature.evaporation > feature.flux) return "salt";

    return "freshwater";
  }

  return {setClimateData, cleanupLakeData, prepareLakeData, defineGroup, generateName, getName, getShoreline};
})();
```

**NOTE:** The lake data is actually started by the river generator.

### My Algorithms

Trying to understand the AFMG code for generating rivers and lakes is too difficult. I already knew the code was hard to read, but it also appears to make some decisions I disagree with. Number one is placing lakes in semi-random locations rather than basing them off of river flowage. So, I'm rethinking the approach and while I was inspired by some of what was done, my algorithm is different. I'm going to trace flowage much the way I did with the wind. Although I think this will be more difficult.


* Create a queue of tiles and accumulation.
* Start off with every land tile using an flowage equal to their precipitation
* Process queue:
  * pop the tile and flowage off the stack.
  * find their lowest neighbors (include lake_depth in the calculation)
  * if the lowest neighbor(s) is higher than this elevation + lake_depth, then subtract the difference from the flowage and add to the lake depth. Then act as if they were equal with the rest if there's still any flowage left.
  * If the lowest neighbor(s) is equal to this elevation + lake_depth, then flood-fill:
     * let lake_threshold = flowage.
     * while lake_threshold > 0:
      * subtract 1 from lake_threshold.
      * add 1 to the lake depth (or some other small amount)
      * walk each neighbor:
        * if the neighbor is higher than the new lake elevation, then don't do anything.
        * if the neighbor is between the tile elevation and the new lake elevation, set it's lake_depth so that the elevations will match. And continue to walk it's neighbors.
        * if the neighbor is below the tile elevation, add it to a list of potential outlets. 
     * Once done, if there's a list of outlets, choose the lowest one(s). Queue those tiles with the original flowage divided equally.
  * If the lowest neighbor(s) are lower than this elevation, then divide the flowage equally. Also, if the current tile has a lake_depth, add the difference between the elevations to the flowage as well before dividing. Then queue those neighbors with that flowage.
  * Finally, mark this tile in the database by increasing the flowage and which tiles were flowed to if we didn't make a lake.



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
[ ] `gen-biomes` command
    [X] Review AFMG biome generation algorithms
    [ ] Create command (requires water, temperature, precipitation, rivers and lakes)
[ ] `gen-people` command
    [ ] various auxiliary files
    [ ] Review AFMG people generation algorithms -- again, wait on improvements until later
    [ ] Figure out how to break the task apart into sub commands and create those commands.
[ ] `curve-borders` command
    [ ] Creates new layers for several thematic layers that have less blocky borders. This is a matter of taking the shape line segments, and converting them to beziers. It makes for better visual appeal.
    [ ] is_ocean
    [ ] biomes
    [ ] nations and provinces
    [ ] cultures
    [ ] religions
[ ] `create-terrain` commands
    [ ] terrain template files
    [ ] Review AFMG terrain generation algorithms
[ ] Documentation
    [ ] Include a caveat that this is not intended to be used for scientific purposes (analyzing streams, etc.) and the algorithms are not meant to model actual physical processes.
[ ] Figure out how to compile and deploy this tool to various operating systems. At least arch linux and windows.
[ ] Announce beta release on Blog, Mammoth, Reddit (AFMG list, imaginarymapping, a few other places), and start updating those places when changes are made.
    -- I feel like having all the above is enough to announce, as long as "creating terrain", a large task, will be the next thing on the list.
[ ] Possibly, split some of the commands apart, for custom manipulation (some of these commands are `dev-` commands now, but can be switched over):
    [ ] `convert-heightmap-voronoi`: creates points and voronoi from a heightmap, but doesn't add neighbors, sample heights, or set ocean
    [ ] `convert-heightmap-neighbors`: calculates neighbors for voronoi tiles -- this will be an alias for a future command `create-terrain-neighbors`
    [ ] `convert-heightmap-sample`
    [ ] `convert-heightmap-ocean`
    [ ] `gen-climate-temperatures`
    [ ] `gen-climate-wind`
    [ ] `gen-climate-precipitation`
[ ] Improved, Similar-area voronoization algorithm vaguely described above
[ ] Improved climate generation commands
[ ] Improved people generation commands
[ ] `gen-features` command
    [ ] Various auxiliar files
    [ ] Review AFMG markers and zones algorithm
[ ] `regen-*` commands
    [ ] Based on what is done in `gen-people` and some other things, but keep things that shouldn't be regenerated.
[ ] `dissolve` commands
[ ] `genesis` command
[ ] Start working on QGIS scripts and tools and a plugin for installing them.
[ ] `convert-afmg` command -- for now, just convert CSV and GeoJSON exports. Don't worry and probably don't plan to support the ".map" file.
[ ] `submap` command
[ ] `convert-image` command if I can't just use convert-heightmap
[ ] `import-climates` command

