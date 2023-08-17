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

# AFMG Algorithms

## People

```js

rankCells
Cultures.generate(); 
Cultures.expand(); 
BurgsAndStates.generate();
Religions.generate(); 
BurgsAndStates.defineStateForms(); 
BurgsAndStates.generateProvinces(); 
BurgsAndStates.defineBurgFeatures(); 

drawStates();
drawBorders(); 
BurgsAndStates.drawStateLabels(); -- TODO:

//---

function rankCells() {
  TIME && console.time("rankCells");
  const {cells, features} = pack;
  cells.s = new Int16Array(cells.i.length); // cell suitability array
  cells.pop = new Float32Array(cells.i.length); // cell population array

  const flMean = d3.median(cells.fl.filter(f => f)) || 0,
    flMax = d3.max(cells.fl) + d3.max(cells.conf); // to normalize flux
  const areaMean = d3.mean(cells.area); // to adjust population by cell area

  for (const i of cells.i) {
    if (cells.h[i] < 20) continue; // no population in water
    let s = +biomesData.habitability[cells.biome[i]]; // base suitability derived from biome habitability
    if (!s) continue; // uninhabitable biomes has 0 suitability
    if (flMean) s += normalize(cells.fl[i] + cells.conf[i], flMean, flMax) * 250; // big rivers and confluences are valued
    s -= (cells.h[i] - 50) / 5; // low elevation is valued, high is not;

    if (cells.t[i] === 1) {
      if (cells.r[i]) s += 15; // estuary is valued
      const feature = features[cells.f[cells.haven[i]]];
      if (feature.type === "lake") {
        if (feature.group === "freshwater") s += 30;
        else if (feature.group == "salt") s += 10;
        else if (feature.group == "frozen") s += 1;
        else if (feature.group == "dry") s -= 5;
        else if (feature.group == "sinkhole") s -= 5;
        else if (feature.group == "lava") s -= 30;
      } else {
        s += 5; // ocean coast is valued
        if (cells.harbor[i] === 1) s += 20; // safe sea harbor is valued
      }
    }

    cells.s[i] = s / 5; // general population rate
    // cell rural population is suitability adjusted by cell area
    cells.pop[i] = cells.s[i] > 0 ? (cells.s[i] * cells.area[i]) / areaMean : 0;
  }

  TIME && console.timeEnd("rankCells");
}


window.Cultures = (function () {
  let cells;

  const generate = function () {
    TIME && console.time("generateCultures");
    cells = pack.cells;

    const cultureIds = new Uint16Array(cells.i.length); // cell cultures
    let count = Math.min(+culturesInput.value, +culturesSet.selectedOptions[0].dataset.max);

    const populated = cells.i.filter(i => cells.s[i]); // populated cells
    if (populated.length < count * 25) {
      count = Math.floor(populated.length / 50);
      if (!count) {
        WARN && console.warn(`There are no populated cells. Cannot generate cultures`);
        pack.cultures = [{name: "Wildlands", i: 0, base: 1, shield: "round"}];
        cells.culture = cultureIds;

        alertMessage.innerHTML = /* html */ `The climate is harsh and people cannot live in this world.<br />
          No cultures, states and burgs will be created.<br />
          Please consider changing climate settings in the World Configurator`;

        $("#alert").dialog({
          resizable: false,
          title: "Extreme climate warning",
          buttons: {
            Ok: function () {
              $(this).dialog("close");
            }
          }
        });
        return;
      } else {
        WARN && console.warn(`Not enough populated cells (${populated.length}). Will generate only ${count} cultures`);
        alertMessage.innerHTML = /* html */ ` There are only ${populated.length} populated cells and it's insufficient livable area.<br />
          Only ${count} out of ${culturesInput.value} requested cultures will be generated.<br />
          Please consider changing climate settings in the World Configurator`;
        $("#alert").dialog({
          resizable: false,
          title: "Extreme climate warning",
          buttons: {
            Ok: function () {
              $(this).dialog("close");
            }
          }
        });
      }
    }

    const cultures = (pack.cultures = selectCultures(count));
    const centers = d3.quadtree();
    const colors = getColors(count);
    const emblemShape = document.getElementById("emblemShape").value;

    const codes = [];

    cultures.forEach(function (c, i) {
      const newId = i + 1;

      if (c.lock) {
        codes.push(c.code);
        centers.add(c.center);

        for (const i of cells.i) {
          if (cells.culture[i] === c.i) cultureIds[i] = newId;
        }

        c.i = newId;
        return;
      }

      const cell = (c.center = placeCenter(c.sort ? c.sort : i => cells.s[i]));
      centers.add(cells.p[cell]);
      c.i = newId;
      delete c.odd;
      delete c.sort;
      c.color = colors[i];
      c.type = defineCultureType(cell);
      c.expansionism = defineCultureExpansionism(c.type);
      c.origins = [0];
      c.code = abbreviate(c.name, codes);
      codes.push(c.code);
      cultureIds[cell] = newId;
      if (emblemShape === "random") c.shield = getRandomShield();
    });

    cells.culture = cultureIds;

    function placeCenter(v) {
      let spacing = (graphWidth + graphHeight) / 2 / count;
      const MAX_ATTEMPTS = 100;

      const sorted = [...populated].sort((a, b) => v(b) - v(a));
      const max = Math.floor(sorted.length / 2);

      let cellId = 0;
      for (let i = 0; i < MAX_ATTEMPTS; i++) {
        cellId = sorted[biased(0, max, 5)];
        spacing *= 0.9;
        if (!cultureIds[cellId] && !centers.find(cells.p[cellId][0], cells.p[cellId][1], spacing)) break;
      }

      return cellId;
    }

    // the first culture with id 0 is for wildlands
    cultures.unshift({name: "Wildlands", i: 0, base: 1, origins: [null], shield: "round"});

    // make sure all bases exist in nameBases
    if (!nameBases.length) {
      ERROR && console.error("Name base is empty, default nameBases will be applied");
      nameBases = Names.getNameBases();
    }

    cultures.forEach(c => (c.base = c.base % nameBases.length));

    function selectCultures(culturesNumber) {
      let def = getDefault(culturesNumber);
      const cultures = [];

      pack.cultures?.forEach(function (culture) {
        if (culture.lock) cultures.push(culture);
      });

      if (!cultures.length) {
        if (culturesNumber === def.length) return def;
        if (def.every(d => d.odd === 1)) return def.splice(0, culturesNumber);
      }

      for (let culture, rnd, i = 0; cultures.length < culturesNumber && def.length > 0; ) {
        do {
          rnd = rand(def.length - 1);
          culture = def[rnd];
          i++;
        } while (i < 200 && !P(culture.odd));
        cultures.push(culture);
        def.splice(rnd, 1);
      }
      return cultures;
    }

    // set culture type based on culture center position
    function defineCultureType(i) {
      if (cells.h[i] < 70 && [1, 2, 4].includes(cells.biome[i])) return "Nomadic"; // high penalty in forest biomes and near coastline
      if (cells.h[i] > 50) return "Highland"; // no penalty for hills and moutains, high for other elevations
      const f = pack.features[cells.f[cells.haven[i]]]; // opposite feature
      if (f.type === "lake" && f.cells > 5) return "Lake"; // low water cross penalty and high for growth not along coastline
      if (
        (cells.harbor[i] && f.type !== "lake" && P(0.1)) ||
        (cells.harbor[i] === 1 && P(0.6)) ||
        (pack.features[cells.f[i]].group === "isle" && P(0.4))
      )
        return "Naval"; // low water cross penalty and high for non-along-coastline growth
      if (cells.r[i] && cells.fl[i] > 100) return "River"; // no River cross penalty, penalty for non-River growth
      if (cells.t[i] > 2 && [3, 7, 8, 9, 10, 12].includes(cells.biome[i])) return "Hunting"; // high penalty in non-native biomes
      return "Generic";
    }

    function defineCultureExpansionism(type) {
      let base = 1; // Generic
      if (type === "Lake") base = 0.8;
      else if (type === "Naval") base = 1.5;
      else if (type === "River") base = 0.9;
      else if (type === "Nomadic") base = 1.5;
      else if (type === "Hunting") base = 0.7;
      else if (type === "Highland") base = 1.2;
      return rn(((Math.random() * powerInput.value) / 2 + 1) * base, 1);
    }

    TIME && console.timeEnd("generateCultures");
  };

  const add = function (center) {
    const defaultCultures = getDefault();
    let culture, base, name;

    if (pack.cultures.length < defaultCultures.length) {
      // add one of the default cultures
      culture = pack.cultures.length;
      base = defaultCultures[culture].base;
      name = defaultCultures[culture].name;
    } else {
      // add random culture besed on one of the current ones
      culture = rand(pack.cultures.length - 1);
      name = Names.getCulture(culture, 5, 8, "");
      base = pack.cultures[culture].base;
    }
    const code = abbreviate(
      name,
      pack.cultures.map(c => c.code)
    );
    const i = pack.cultures.length;
    const color = d3.color(d3.scaleSequential(d3.interpolateRainbow)(Math.random())).hex();

    // define emblem shape
    let shield = culture.shield;
    const emblemShape = document.getElementById("emblemShape").value;
    if (emblemShape === "random") shield = getRandomShield();

    pack.cultures.push({
      name,
      color,
      base,
      center,
      i,
      expansionism: 1,
      type: "Generic",
      cells: 0,
      area: 0,
      rural: 0,
      urban: 0,
      origins: [0],
      code,
      shield
    });
  };

  const getDefault = function (count) {
    // generic sorting functions
    const cells = pack.cells,
      s = cells.s,
      sMax = d3.max(s),
      t = cells.t,
      h = cells.h,
      temp = grid.cells.temp;
    const n = cell => Math.ceil((s[cell] / sMax) * 3); // normalized cell score
    const td = (cell, goal) => {
      const d = Math.abs(temp[cells.g[cell]] - goal);
      return d ? d + 1 : 1;
    }; // temperature difference fee
    const bd = (cell, biomes, fee = 4) => (biomes.includes(cells.biome[cell]) ? 1 : fee); // biome difference fee
    const sf = (cell, fee = 4) =>
      cells.haven[cell] && pack.features[cells.f[cells.haven[cell]]].type !== "lake" ? 1 : fee; // not on sea coast fee

    if (culturesSet.value === "european") {
      return [
        {name: "Shwazen", base: 0, odd: 1, sort: i => n(i) / td(i, 10) / bd(i, [6, 8]), shield: "swiss"},
        {name: "Angshire", base: 1, odd: 1, sort: i => n(i) / td(i, 10) / sf(i), shield: "wedged"},
        {name: "Luari", base: 2, odd: 1, sort: i => n(i) / td(i, 12) / bd(i, [6, 8]), shield: "french"},
        {name: "Tallian", base: 3, odd: 1, sort: i => n(i) / td(i, 15), shield: "horsehead"},
        {name: "Astellian", base: 4, odd: 1, sort: i => n(i) / td(i, 16), shield: "spanish"},
        {name: "Slovan", base: 5, odd: 1, sort: i => (n(i) / td(i, 6)) * t[i], shield: "polish"},
        {name: "Norse", base: 6, odd: 1, sort: i => n(i) / td(i, 5), shield: "heater"},
        {name: "Elladan", base: 7, odd: 1, sort: i => (n(i) / td(i, 18)) * h[i], shield: "boeotian"},
        {name: "Romian", base: 8, odd: 0.2, sort: i => n(i) / td(i, 15) / t[i], shield: "roman"},
        {name: "Soumi", base: 9, odd: 1, sort: i => (n(i) / td(i, 5) / bd(i, [9])) * t[i], shield: "pavise"},
        {name: "Portuzian", base: 13, odd: 1, sort: i => n(i) / td(i, 17) / sf(i), shield: "renaissance"},
        {name: "Vengrian", base: 15, odd: 1, sort: i => (n(i) / td(i, 11) / bd(i, [4])) * t[i], shield: "horsehead2"},
        {name: "Turchian", base: 16, odd: 0.05, sort: i => n(i) / td(i, 14), shield: "round"},
        {name: "Euskati", base: 20, odd: 0.05, sort: i => (n(i) / td(i, 15)) * h[i], shield: "oldFrench"},
        {name: "Keltan", base: 22, odd: 0.05, sort: i => (n(i) / td(i, 11) / bd(i, [6, 8])) * t[i], shield: "oval"}
      ];
    }

    if (culturesSet.value === "oriental") {
      return [
        {name: "Koryo", base: 10, odd: 1, sort: i => n(i) / td(i, 12) / t[i], shield: "round"},
        {name: "Hantzu", base: 11, odd: 1, sort: i => n(i) / td(i, 13), shield: "banner"},
        {name: "Yamoto", base: 12, odd: 1, sort: i => n(i) / td(i, 15) / t[i], shield: "round"},
        {name: "Turchian", base: 16, odd: 1, sort: i => n(i) / td(i, 12), shield: "round"},
        {
          name: "Berberan",
          base: 17,
          odd: 0.2,
          sort: i => (n(i) / td(i, 19) / bd(i, [1, 2, 3], 7)) * t[i],
          shield: "oval"
        },
        {name: "Eurabic", base: 18, odd: 1, sort: i => (n(i) / td(i, 26) / bd(i, [1, 2], 7)) * t[i], shield: "oval"},
        {name: "Efratic", base: 23, odd: 0.1, sort: i => (n(i) / td(i, 22)) * t[i], shield: "round"},
        {name: "Tehrani", base: 24, odd: 1, sort: i => (n(i) / td(i, 18)) * h[i], shield: "round"},
        {name: "Maui", base: 25, odd: 0.2, sort: i => n(i) / td(i, 24) / sf(i) / t[i], shield: "vesicaPiscis"},
        {name: "Carnatic", base: 26, odd: 0.5, sort: i => n(i) / td(i, 26), shield: "round"},
        {name: "Vietic", base: 29, odd: 0.8, sort: i => n(i) / td(i, 25) / bd(i, [7], 7) / t[i], shield: "banner"},
        {name: "Guantzu", base: 30, odd: 0.5, sort: i => n(i) / td(i, 17), shield: "banner"},
        {name: "Ulus", base: 31, odd: 1, sort: i => (n(i) / td(i, 5) / bd(i, [2, 4, 10], 7)) * t[i], shield: "banner"}
      ];
    }

    if (culturesSet.value === "english") {
      const getName = () => Names.getBase(1, 5, 9, "", 0);
      return [
        {name: getName(), base: 1, odd: 1, shield: "heater"},
        {name: getName(), base: 1, odd: 1, shield: "wedged"},
        {name: getName(), base: 1, odd: 1, shield: "swiss"},
        {name: getName(), base: 1, odd: 1, shield: "oldFrench"},
        {name: getName(), base: 1, odd: 1, shield: "swiss"},
        {name: getName(), base: 1, odd: 1, shield: "spanish"},
        {name: getName(), base: 1, odd: 1, shield: "hessen"},
        {name: getName(), base: 1, odd: 1, shield: "fantasy5"},
        {name: getName(), base: 1, odd: 1, shield: "fantasy4"},
        {name: getName(), base: 1, odd: 1, shield: "fantasy1"}
      ];
    }

    if (culturesSet.value === "antique") {
      return [
        {name: "Roman", base: 8, odd: 1, sort: i => n(i) / td(i, 14) / t[i], shield: "roman"}, // Roman
        {name: "Roman", base: 8, odd: 1, sort: i => n(i) / td(i, 15) / sf(i), shield: "roman"}, // Roman
        {name: "Roman", base: 8, odd: 1, sort: i => n(i) / td(i, 16) / sf(i), shield: "roman"}, // Roman
        {name: "Roman", base: 8, odd: 1, sort: i => n(i) / td(i, 17) / t[i], shield: "roman"}, // Roman
        {name: "Hellenic", base: 7, odd: 1, sort: i => (n(i) / td(i, 18) / sf(i)) * h[i], shield: "boeotian"}, // Greek
        {name: "Hellenic", base: 7, odd: 1, sort: i => (n(i) / td(i, 19) / sf(i)) * h[i], shield: "boeotian"}, // Greek
        {name: "Macedonian", base: 7, odd: 0.5, sort: i => (n(i) / td(i, 12)) * h[i], shield: "round"}, // Greek
        {name: "Celtic", base: 22, odd: 1, sort: i => n(i) / td(i, 11) ** 0.5 / bd(i, [6, 8]), shield: "round"},
        {name: "Germanic", base: 0, odd: 1, sort: i => n(i) / td(i, 10) ** 0.5 / bd(i, [6, 8]), shield: "round"},
        {name: "Persian", base: 24, odd: 0.8, sort: i => (n(i) / td(i, 18)) * h[i], shield: "oval"}, // Iranian
        {name: "Scythian", base: 24, odd: 0.5, sort: i => n(i) / td(i, 11) ** 0.5 / bd(i, [4]), shield: "round"}, // Iranian
        {name: "Cantabrian", base: 20, odd: 0.5, sort: i => (n(i) / td(i, 16)) * h[i], shield: "oval"}, // Basque
        {name: "Estian", base: 9, odd: 0.2, sort: i => (n(i) / td(i, 5)) * t[i], shield: "pavise"}, // Finnic
        {name: "Carthaginian", base: 42, odd: 0.3, sort: i => n(i) / td(i, 20) / sf(i), shield: "oval"}, // Levantine
        {name: "Hebrew", base: 42, odd: 0.2, sort: i => (n(i) / td(i, 19)) * sf(i), shield: "oval"}, // Levantine
        {name: "Mesopotamian", base: 23, odd: 0.2, sort: i => n(i) / td(i, 22) / bd(i, [1, 2, 3]), shield: "oval"} // Mesopotamian
      ];
    }

    if (culturesSet.value === "highFantasy") {
      return [
        // fantasy races
        {
          name: "Quenian (Elfish)",
          base: 33,
          odd: 1,
          sort: i => (n(i) / bd(i, [6, 7, 8, 9], 10)) * t[i],
          shield: "gondor"
        }, // Elves
        {
          name: "Eldar (Elfish)",
          base: 33,
          odd: 1,
          sort: i => (n(i) / bd(i, [6, 7, 8, 9], 10)) * t[i],
          shield: "noldor"
        }, // Elves
        {
          name: "Trow (Dark Elfish)",
          base: 34,
          odd: 0.9,
          sort: i => (n(i) / bd(i, [7, 8, 9, 12], 10)) * t[i],
          shield: "hessen"
        }, // Dark Elves
        {
          name: "Lothian (Dark Elfish)",
          base: 34,
          odd: 0.3,
          sort: i => (n(i) / bd(i, [7, 8, 9, 12], 10)) * t[i],
          shield: "wedged"
        }, // Dark Elves
        {name: "Dunirr (Dwarven)", base: 35, odd: 1, sort: i => n(i) + h[i], shield: "ironHills"}, // Dwarfs
        {name: "Khazadur (Dwarven)", base: 35, odd: 1, sort: i => n(i) + h[i], shield: "erebor"}, // Dwarfs
        {name: "Kobold (Goblin)", base: 36, odd: 1, sort: i => t[i] - s[i], shield: "moriaOrc"}, // Goblin
        {name: "Uruk (Orkish)", base: 37, odd: 1, sort: i => h[i] * t[i], shield: "urukHai"}, // Orc
        {
          name: "Ugluk (Orkish)",
          base: 37,
          odd: 0.5,
          sort: i => (h[i] * t[i]) / bd(i, [1, 2, 10, 11]),
          shield: "moriaOrc"
        }, // Orc
        {name: "Yotunn (Giants)", base: 38, odd: 0.7, sort: i => td(i, -10), shield: "pavise"}, // Giant
        {name: "Rake (Drakonic)", base: 39, odd: 0.7, sort: i => -s[i], shield: "fantasy2"}, // Draconic
        {name: "Arago (Arachnid)", base: 40, odd: 0.7, sort: i => t[i] - s[i], shield: "horsehead2"}, // Arachnid
        {name: "Aj'Snaga (Serpents)", base: 41, odd: 0.7, sort: i => n(i) / bd(i, [12], 10), shield: "fantasy1"}, // Serpents
        // fantasy human
        {name: "Anor (Human)", base: 32, odd: 1, sort: i => n(i) / td(i, 10), shield: "fantasy5"},
        {name: "Dail (Human)", base: 32, odd: 1, sort: i => n(i) / td(i, 13), shield: "roman"},
        {name: "Rohand (Human)", base: 16, odd: 1, sort: i => n(i) / td(i, 16), shield: "round"},
        {
          name: "Dulandir (Human)",
          base: 31,
          odd: 1,
          sort: i => (n(i) / td(i, 5) / bd(i, [2, 4, 10], 7)) * t[i],
          shield: "easterling"
        }
      ];
    }

    if (culturesSet.value === "darkFantasy") {
      return [
        // common real-world English
        {name: "Angshire", base: 1, odd: 1, sort: i => n(i) / td(i, 10) / sf(i), shield: "heater"},
        {name: "Enlandic", base: 1, odd: 1, sort: i => n(i) / td(i, 12), shield: "heater"},
        {name: "Westen", base: 1, odd: 1, sort: i => n(i) / td(i, 10), shield: "heater"},
        {name: "Nortumbic", base: 1, odd: 1, sort: i => n(i) / td(i, 7), shield: "heater"},
        {name: "Mercian", base: 1, odd: 1, sort: i => n(i) / td(i, 9), shield: "heater"},
        {name: "Kentian", base: 1, odd: 1, sort: i => n(i) / td(i, 12), shield: "heater"},
        // rare real-world western
        {name: "Norse", base: 6, odd: 0.7, sort: i => n(i) / td(i, 5) / sf(i), shield: "oldFrench"},
        {name: "Schwarzen", base: 0, odd: 0.3, sort: i => n(i) / td(i, 10) / bd(i, [6, 8]), shield: "gonfalon"},
        {name: "Luarian", base: 2, odd: 0.3, sort: i => n(i) / td(i, 12) / bd(i, [6, 8]), shield: "oldFrench"},
        {name: "Hetallian", base: 3, odd: 0.3, sort: i => n(i) / td(i, 15), shield: "oval"},
        {name: "Astellian", base: 4, odd: 0.3, sort: i => n(i) / td(i, 16), shield: "spanish"},
        // rare real-world exotic
        {
          name: "Kiswaili",
          base: 28,
          odd: 0.05,
          sort: i => n(i) / td(i, 29) / bd(i, [1, 3, 5, 7]),
          shield: "vesicaPiscis"
        },
        {name: "Yoruba", base: 21, odd: 0.05, sort: i => n(i) / td(i, 15) / bd(i, [5, 7]), shield: "vesicaPiscis"},
        {name: "Koryo", base: 10, odd: 0.05, sort: i => n(i) / td(i, 12) / t[i], shield: "round"},
        {name: "Hantzu", base: 11, odd: 0.05, sort: i => n(i) / td(i, 13), shield: "banner"},
        {name: "Yamoto", base: 12, odd: 0.05, sort: i => n(i) / td(i, 15) / t[i], shield: "round"},
        {name: "Guantzu", base: 30, odd: 0.05, sort: i => n(i) / td(i, 17), shield: "banner"},
        {
          name: "Ulus",
          base: 31,
          odd: 0.05,
          sort: i => (n(i) / td(i, 5) / bd(i, [2, 4, 10], 7)) * t[i],
          shield: "banner"
        },
        {name: "Turan", base: 16, odd: 0.05, sort: i => n(i) / td(i, 12), shield: "round"},
        {
          name: "Berberan",
          base: 17,
          odd: 0.05,
          sort: i => (n(i) / td(i, 19) / bd(i, [1, 2, 3], 7)) * t[i],
          shield: "round"
        },
        {
          name: "Eurabic",
          base: 18,
          odd: 0.05,
          sort: i => (n(i) / td(i, 26) / bd(i, [1, 2], 7)) * t[i],
          shield: "round"
        },
        {name: "Slovan", base: 5, odd: 0.05, sort: i => (n(i) / td(i, 6)) * t[i], shield: "round"},
        {
          name: "Keltan",
          base: 22,
          odd: 0.1,
          sort: i => n(i) / td(i, 11) ** 0.5 / bd(i, [6, 8]),
          shield: "vesicaPiscis"
        },
        {name: "Elladan", base: 7, odd: 0.2, sort: i => (n(i) / td(i, 18) / sf(i)) * h[i], shield: "boeotian"},
        {name: "Romian", base: 8, odd: 0.2, sort: i => n(i) / td(i, 14) / t[i], shield: "roman"},
        // fantasy races
        {name: "Eldar", base: 33, odd: 0.5, sort: i => (n(i) / bd(i, [6, 7, 8, 9], 10)) * t[i], shield: "fantasy5"}, // Elves
        {name: "Trow", base: 34, odd: 0.8, sort: i => (n(i) / bd(i, [7, 8, 9, 12], 10)) * t[i], shield: "hessen"}, // Dark Elves
        {name: "Durinn", base: 35, odd: 0.8, sort: i => n(i) + h[i], shield: "erebor"}, // Dwarven
        {name: "Kobblin", base: 36, odd: 0.8, sort: i => t[i] - s[i], shield: "moriaOrc"}, // Goblin
        {name: "Uruk", base: 37, odd: 0.8, sort: i => (h[i] * t[i]) / bd(i, [1, 2, 10, 11]), shield: "urukHai"}, // Orc
        {name: "Yotunn", base: 38, odd: 0.8, sort: i => td(i, -10), shield: "pavise"}, // Giant
        {name: "Drake", base: 39, odd: 0.9, sort: i => -s[i], shield: "fantasy2"}, // Draconic
        {name: "Rakhnid", base: 40, odd: 0.9, sort: i => t[i] - s[i], shield: "horsehead2"}, // Arachnid
        {name: "Aj'Snaga", base: 41, odd: 0.9, sort: i => n(i) / bd(i, [12], 10), shield: "fantasy1"} // Serpents
      ];
    }

    if (culturesSet.value === "random") {
      return d3.range(count).map(function () {
        const rnd = rand(nameBases.length - 1);
        const name = Names.getBaseShort(rnd);
        return {name, base: rnd, odd: 1, shield: getRandomShield()};
      });
    }

    // all-world
    return [
      {name: "Shwazen", base: 0, odd: 0.7, sort: i => n(i) / td(i, 10) / bd(i, [6, 8]), shield: "hessen"},
      {name: "Angshire", base: 1, odd: 1, sort: i => n(i) / td(i, 10) / sf(i), shield: "heater"},
      {name: "Luari", base: 2, odd: 0.6, sort: i => n(i) / td(i, 12) / bd(i, [6, 8]), shield: "oldFrench"},
      {name: "Tallian", base: 3, odd: 0.6, sort: i => n(i) / td(i, 15), shield: "horsehead2"},
      {name: "Astellian", base: 4, odd: 0.6, sort: i => n(i) / td(i, 16), shield: "spanish"},
      {name: "Slovan", base: 5, odd: 0.7, sort: i => (n(i) / td(i, 6)) * t[i], shield: "round"},
      {name: "Norse", base: 6, odd: 0.7, sort: i => n(i) / td(i, 5), shield: "heater"},
      {name: "Elladan", base: 7, odd: 0.7, sort: i => (n(i) / td(i, 18)) * h[i], shield: "boeotian"},
      {name: "Romian", base: 8, odd: 0.7, sort: i => n(i) / td(i, 15), shield: "roman"},
      {name: "Soumi", base: 9, odd: 0.3, sort: i => (n(i) / td(i, 5) / bd(i, [9])) * t[i], shield: "pavise"},
      {name: "Koryo", base: 10, odd: 0.1, sort: i => n(i) / td(i, 12) / t[i], shield: "round"},
      {name: "Hantzu", base: 11, odd: 0.1, sort: i => n(i) / td(i, 13), shield: "banner"},
      {name: "Yamoto", base: 12, odd: 0.1, sort: i => n(i) / td(i, 15) / t[i], shield: "round"},
      {name: "Portuzian", base: 13, odd: 0.4, sort: i => n(i) / td(i, 17) / sf(i), shield: "spanish"},
      {name: "Nawatli", base: 14, odd: 0.1, sort: i => h[i] / td(i, 18) / bd(i, [7]), shield: "square"},
      {name: "Vengrian", base: 15, odd: 0.2, sort: i => (n(i) / td(i, 11) / bd(i, [4])) * t[i], shield: "wedged"},
      {name: "Turchian", base: 16, odd: 0.2, sort: i => n(i) / td(i, 13), shield: "round"},
      {
        name: "Berberan",
        base: 17,
        odd: 0.1,
        sort: i => (n(i) / td(i, 19) / bd(i, [1, 2, 3], 7)) * t[i],
        shield: "round"
      },
      {name: "Eurabic", base: 18, odd: 0.2, sort: i => (n(i) / td(i, 26) / bd(i, [1, 2], 7)) * t[i], shield: "round"},
      {name: "Inuk", base: 19, odd: 0.05, sort: i => td(i, -1) / bd(i, [10, 11]) / sf(i), shield: "square"},
      {name: "Euskati", base: 20, odd: 0.05, sort: i => (n(i) / td(i, 15)) * h[i], shield: "spanish"},
      {name: "Yoruba", base: 21, odd: 0.05, sort: i => n(i) / td(i, 15) / bd(i, [5, 7]), shield: "vesicaPiscis"},
      {
        name: "Keltan",
        base: 22,
        odd: 0.05,
        sort: i => (n(i) / td(i, 11) / bd(i, [6, 8])) * t[i],
        shield: "vesicaPiscis"
      },
      {name: "Efratic", base: 23, odd: 0.05, sort: i => (n(i) / td(i, 22)) * t[i], shield: "diamond"},
      {name: "Tehrani", base: 24, odd: 0.1, sort: i => (n(i) / td(i, 18)) * h[i], shield: "round"},
      {name: "Maui", base: 25, odd: 0.05, sort: i => n(i) / td(i, 24) / sf(i) / t[i], shield: "round"},
      {name: "Carnatic", base: 26, odd: 0.05, sort: i => n(i) / td(i, 26), shield: "round"},
      {name: "Inqan", base: 27, odd: 0.05, sort: i => h[i] / td(i, 13), shield: "square"},
      {name: "Kiswaili", base: 28, odd: 0.1, sort: i => n(i) / td(i, 29) / bd(i, [1, 3, 5, 7]), shield: "vesicaPiscis"},
      {name: "Vietic", base: 29, odd: 0.1, sort: i => n(i) / td(i, 25) / bd(i, [7], 7) / t[i], shield: "banner"},
      {name: "Guantzu", base: 30, odd: 0.1, sort: i => n(i) / td(i, 17), shield: "banner"},
      {name: "Ulus", base: 31, odd: 0.1, sort: i => (n(i) / td(i, 5) / bd(i, [2, 4, 10], 7)) * t[i], shield: "banner"},
      {name: "Hebrew", base: 42, odd: 0.2, sort: i => (n(i) / td(i, 18)) * sf(i), shield: "oval"} // Levantine
    ];
  };

  // expand cultures across the map (Dijkstra-like algorithm)
  const expand = function () {
    TIME && console.time("expandCultures");
    const {cells, cultures} = pack;

    const queue = new PriorityQueue({comparator: (a, b) => a.priority - b.priority});
    const cost = [];

    const neutralRate = byId("neutralRate")?.valueAsNumber || 1;
    const maxExpansionCost = cells.i.length * 0.6 * neutralRate; // limit cost for culture growth

    // remove culture from all cells except of locked
    const hasLocked = cultures.some(c => !c.removed && c.lock);
    if (hasLocked) {
      for (const cellId of cells.i) {
        const culture = cultures[cells.culture[cellId]];
        if (culture.lock) continue;
        cells.culture[cellId] = 0;
      }
    } else {
      cells.culture = new Uint16Array(cells.i.length);
    }

    for (const culture of cultures) {
      if (!culture.i || culture.removed || culture.lock) continue;
      queue.queue({cellId: culture.center, cultureId: culture.i, priority: 0});
    }

    while (queue.length) {
      const {cellId, priority, cultureId} = queue.dequeue();
      const {type, expansionism} = cultures[cultureId];

      cells.c[cellId].forEach(neibCellId => {
        if (hasLocked) {
          const neibCultureId = cells.culture[neibCellId];
          if (neibCultureId && cultures[neibCultureId].lock) return; // do not overwrite cell of locked culture
        }

        const biome = cells.biome[neibCellId];
        const biomeCost = getBiomeCost(cultureId, biome, type);
        const biomeChangeCost = biome === cells.biome[neibCellId] ? 0 : 20; // penalty on biome change
        const heightCost = getHeightCost(neibCellId, cells.h[neibCellId], type);
        const riverCost = getRiverCost(cells.r[neibCellId], neibCellId, type);
        const typeCost = getTypeCost(cells.t[neibCellId], type);

        const cellCost = (biomeCost + biomeChangeCost + heightCost + riverCost + typeCost) / expansionism;
        const totalCost = priority + cellCost;

        if (totalCost > maxExpansionCost) return;

        if (!cost[neibCellId] || totalCost < cost[neibCellId]) {
          if (cells.pop[neibCellId] > 0) cells.culture[neibCellId] = cultureId; // assign culture to populated cell
          cost[neibCellId] = totalCost;
          queue.queue({cellId: neibCellId, cultureId, priority: totalCost});
        }
      });
    }

    function getBiomeCost(c, biome, type) {
      if (cells.biome[cultures[c].center] === biome) return 10; // tiny penalty for native biome
      if (type === "Hunting") return biomesData.cost[biome] * 5; // non-native biome penalty for hunters
      if (type === "Nomadic" && biome > 4 && biome < 10) return biomesData.cost[biome] * 10; // forest biome penalty for nomads
      return biomesData.cost[biome] * 2; // general non-native biome penalty
    }

    function getHeightCost(i, h, type) {
      const f = pack.features[cells.f[i]],
        a = cells.area[i];
      if (type === "Lake" && f.type === "lake") return 10; // no lake crossing penalty for Lake cultures
      if (type === "Naval" && h < 20) return a * 2; // low sea/lake crossing penalty for Naval cultures
      if (type === "Nomadic" && h < 20) return a * 50; // giant sea/lake crossing penalty for Nomads
      if (h < 20) return a * 6; // general sea/lake crossing penalty
      if (type === "Highland" && h < 44) return 3000; // giant penalty for highlanders on lowlands
      if (type === "Highland" && h < 62) return 200; // giant penalty for highlanders on lowhills
      if (type === "Highland") return 0; // no penalty for highlanders on highlands
      if (h >= 67) return 200; // general mountains crossing penalty
      if (h >= 44) return 30; // general hills crossing penalty
      return 0;
    }

    function getRiverCost(riverId, cellId, type) {
      if (type === "River") return riverId ? 0 : 100; // penalty for river cultures
      if (!riverId) return 0; // no penalty for others if there is no river
      return minmax(cells.fl[cellId] / 10, 20, 100); // river penalty from 20 to 100 based on flux
    }

    function getTypeCost(t, type) {
      if (t === 1) return type === "Naval" || type === "Lake" ? 0 : type === "Nomadic" ? 60 : 20; // penalty for coastline
      if (t === 2) return type === "Naval" || type === "Nomadic" ? 30 : 0; // low penalty for land level 2 for Navals and nomads
      if (t !== -1) return type === "Naval" || type === "Lake" ? 100 : 0; // penalty for mainland for navals
      return 0;
    }

    TIME && console.timeEnd("expandCultures");
  };

  const getRandomShield = function () {
    const type = rw(COA.shields.types);
    return rw(COA.shields[type]);
  };

  return {generate, add, expand, getDefault, getRandomShield};
})();

window.BurgsAndStates = (function () {
  const generate = function () {
    const {cells, cultures} = pack;
    const n = cells.i.length;

    cells.burg = new Uint16Array(n); // cell burg
    cells.road = new Uint16Array(n); // cell road power
    cells.crossroad = new Uint16Array(n); // cell crossroad power

    const burgs = (pack.burgs = placeCapitals());
    pack.states = createStates();
    const capitalRoutes = Routes.getRoads();

    placeTowns();
    expandStates();
    normalizeStates();
    const townRoutes = Routes.getTrails();
    specifyBurgs();

    const oceanRoutes = Routes.getSearoutes();

    collectStatistics();
    assignColors();

    generateCampaigns();
    generateDiplomacy();
    Routes.draw(capitalRoutes, townRoutes, oceanRoutes);
    drawBurgs();

    function placeCapitals() {
      TIME && console.time("placeCapitals");
      let count = +regionsOutput.value;
      let burgs = [0];

      const rand = () => 0.5 + Math.random() * 0.5;
      const score = new Int16Array(cells.s.map(s => s * rand())); // cell score for capitals placement
      const sorted = cells.i.filter(i => score[i] > 0 && cells.culture[i]).sort((a, b) => score[b] - score[a]); // filtered and sorted array of indexes

      if (sorted.length < count * 10) {
        count = Math.floor(sorted.length / 10);
        if (!count) {
          WARN && console.warn("There is no populated cells. Cannot generate states");
          return burgs;
        } else {
          WARN && console.warn(`Not enough populated cells (${sorted.length}). Will generate only ${count} states`);
        }
      }

      let burgsTree = d3.quadtree();
      let spacing = (graphWidth + graphHeight) / 2 / count; // min distance between capitals

      for (let i = 0; burgs.length <= count; i++) {
        const cell = sorted[i];
        const [x, y] = cells.p[cell];

        if (burgsTree.find(x, y, spacing) === undefined) {
          burgs.push({cell, x, y});
          burgsTree.add([x, y]);
        }

        if (i === sorted.length - 1) {
          WARN && console.warn("Cannot place capitals with current spacing. Trying again with reduced spacing");
          burgsTree = d3.quadtree();
          i = -1;
          burgs = [0];
          spacing /= 1.2;
        }
      }

      burgs[0] = burgsTree;
      TIME && console.timeEnd("placeCapitals");
      return burgs;
    }

    // For each capital create a state
    function createStates() {
      TIME && console.time("createStates");
      const states = [{i: 0, name: "Neutrals"}];
      const colors = getColors(burgs.length - 1);
      const each5th = each(5);

      burgs.forEach(function (b, i) {
        if (!i) return; // skip first element

        // burgs data
        b.i = b.state = i;
        b.culture = cells.culture[b.cell];
        b.name = Names.getCultureShort(b.culture);
        b.feature = cells.f[b.cell];
        b.capital = 1;

        // states data
        const expansionism = rn(Math.random() * powerInput.value + 1, 1);
        const basename = b.name.length < 9 && each5th(b.cell) ? b.name : Names.getCultureShort(b.culture);
        const name = Names.getState(basename, b.culture);
        const type = cultures[b.culture].type;

        const coa = COA.generate(null, null, null, type);
        coa.shield = COA.getShield(b.culture, null);
        states.push({
          i,
          color: colors[i - 1],
          name,
          expansionism,
          capital: i,
          type,
          center: b.cell,
          culture: b.culture,
          coa
        });
        cells.burg[b.cell] = i;
      });

      TIME && console.timeEnd("createStates");
      return states;
    }

    // place secondary settlements based on geo and economical evaluation
    function placeTowns() {
      TIME && console.time("placeTowns");
      const score = new Int16Array(cells.s.map(s => s * gauss(1, 3, 0, 20, 3))); // a bit randomized cell score for towns placement
      const sorted = cells.i
        .filter(i => !cells.burg[i] && score[i] > 0 && cells.culture[i])
        .sort((a, b) => score[b] - score[a]); // filtered and sorted array of indexes

      const desiredNumber =
        manorsInput.value == 1000
          ? rn(sorted.length / 5 / (grid.points.length / 10000) ** 0.8)
          : manorsInput.valueAsNumber;
      const burgsNumber = Math.min(desiredNumber, sorted.length); // towns to generate
      let burgsAdded = 0;

      const burgsTree = burgs[0];
      let spacing = (graphWidth + graphHeight) / 150 / (burgsNumber ** 0.7 / 66); // min distance between towns

      while (burgsAdded < burgsNumber && spacing > 1) {
        for (let i = 0; burgsAdded < burgsNumber && i < sorted.length; i++) {
          if (cells.burg[sorted[i]]) continue;
          const cell = sorted[i],
            x = cells.p[cell][0],
            y = cells.p[cell][1];
          const s = spacing * gauss(1, 0.3, 0.2, 2, 2); // randomize to make placement not uniform
          if (burgsTree.find(x, y, s) !== undefined) continue; // to close to existing burg
          const burg = burgs.length;
          const culture = cells.culture[cell];
          const name = Names.getCulture(culture);
          burgs.push({cell, x, y, state: 0, i: burg, culture, name, capital: 0, feature: cells.f[cell]});
          burgsTree.add([x, y]);
          cells.burg[cell] = burg;
          burgsAdded++;
        }
        spacing *= 0.5;
      }

      if (manorsInput.value != 1000 && burgsAdded < desiredNumber) {
        ERROR && console.error(`Cannot place all burgs. Requested ${desiredNumber}, placed ${burgsAdded}`);
      }

      burgs[0] = {name: undefined}; // do not store burgsTree anymore
      TIME && console.timeEnd("placeTowns");
    }
  };

  // define burg coordinates, coa, port status and define details
  const specifyBurgs = function () {
    TIME && console.time("specifyBurgs");
    const cells = pack.cells,
      vertices = pack.vertices,
      features = pack.features,
      temp = grid.cells.temp;

    for (const b of pack.burgs) {
      if (!b.i || b.lock) continue;
      const i = b.cell;

      // asign port status to some coastline burgs with temp > 0 °C
      const haven = cells.haven[i];
      if (haven && temp[cells.g[i]] > 0) {
        const f = cells.f[haven]; // water body id
        // port is a capital with any harbor OR town with good harbor
        const port = features[f].cells > 1 && ((b.capital && cells.harbor[i]) || cells.harbor[i] === 1);
        b.port = port ? f : 0; // port is defined by water body id it lays on
      } else b.port = 0;

      // define burg population (keep urbanization at about 10% rate)
      b.population = rn(Math.max((cells.s[i] + cells.road[i] / 2) / 8 + b.i / 1000 + (i % 100) / 1000, 0.1), 3);
      if (b.capital) b.population = rn(b.population * 1.3, 3); // increase capital population

      if (b.port) {
        b.population = b.population * 1.3; // increase port population
        const [x, y] = getMiddlePoint(i, haven);
        b.x = x;
        b.y = y;
      }

      // add random factor
      b.population = rn(b.population * gauss(2, 3, 0.6, 20, 3), 3);

      // shift burgs on rivers semi-randomly and just a bit
      if (!b.port && cells.r[i]) {
        const shift = Math.min(cells.fl[i] / 150, 1);
        if (i % 2) b.x = rn(b.x + shift, 2);
        else b.x = rn(b.x - shift, 2);
        if (cells.r[i] % 2) b.y = rn(b.y + shift, 2);
        else b.y = rn(b.y - shift, 2);
      }

      // define emblem
      const state = pack.states[b.state];
      const stateCOA = state.coa;
      let kinship = 0.25;
      if (b.capital) kinship += 0.1;
      else if (b.port) kinship -= 0.1;
      if (b.culture !== state.culture) kinship -= 0.25;
      b.type = getType(i, b.port);
      const type = b.capital && P(0.2) ? "Capital" : b.type === "Generic" ? "City" : b.type;
      b.coa = COA.generate(stateCOA, kinship, null, type);
      b.coa.shield = COA.getShield(b.culture, b.state);
    }

    // de-assign port status if it's the only one on feature
    const ports = pack.burgs.filter(b => !b.removed && b.port > 0);
    for (const f of features) {
      if (!f.i || f.land || f.border) continue;
      const featurePorts = ports.filter(b => b.port === f.i);
      if (featurePorts.length === 1) featurePorts[0].port = 0;
    }

    TIME && console.timeEnd("specifyBurgs");
  };

  const getType = function (i, port) {
    const cells = pack.cells;
    if (port) return "Naval";
    if (cells.haven[i] && pack.features[cells.f[cells.haven[i]]].type === "lake") return "Lake";
    if (cells.h[i] > 60) return "Highland";
    if (cells.r[i] && cells.r[i].length > 100 && cells.r[i].length >= pack.rivers[0].length) return "River";

    if (!cells.burg[i] || pack.burgs[cells.burg[i]].population < 6) {
      if (population < 5 && [1, 2, 3, 4].includes(cells.biome[i])) return "Nomadic";
      if (cells.biome[i] > 4 && cells.biome[i] < 10) return "Hunting";
    }

    return "Generic";
  };

  const defineBurgFeatures = function (newburg) {
    const cells = pack.cells;
    pack.burgs
      .filter(b => (newburg ? b.i == newburg.i : b.i && !b.removed))
      .forEach(b => {
        const pop = b.population;
        b.citadel = b.capital || (pop > 50 && P(0.75)) || P(0.5) ? 1 : 0;
        b.plaza = pop > 50 || (pop > 30 && P(0.75)) || (pop > 10 && P(0.5)) || P(0.25) ? 1 : 0;
        b.walls = b.capital || pop > 30 || (pop > 20 && P(0.75)) || (pop > 10 && P(0.5)) || P(0.2) ? 1 : 0;
        b.shanty = pop > 60 || (pop > 40 && P(0.75)) || (pop > 20 && b.walls && P(0.4)) ? 1 : 0;
        const religion = cells.religion[b.cell];
        const theocracy = pack.states[b.state].form === "Theocracy";
        b.temple = (religion && theocracy) || pop > 50 || (pop > 35 && P(0.75)) || (pop > 20 && P(0.5)) ? 1 : 0;
      });
  };

  const drawBurgs = function () {
    TIME && console.time("drawBurgs");

    // remove old data
    burgIcons.selectAll("circle").remove();
    burgLabels.selectAll("text").remove();
    icons.selectAll("use").remove();

    // capitals
    const capitals = pack.burgs.filter(b => b.capital && !b.removed);
    const capitalIcons = burgIcons.select("#cities");
    const capitalLabels = burgLabels.select("#cities");
    const capitalSize = capitalIcons.attr("size") || 1;
    const capitalAnchors = anchors.selectAll("#cities");
    const caSize = capitalAnchors.attr("size") || 2;

    capitalIcons
      .selectAll("circle")
      .data(capitals)
      .enter()
      .append("circle")
      .attr("id", d => "burg" + d.i)
      .attr("data-id", d => d.i)
      .attr("cx", d => d.x)
      .attr("cy", d => d.y)
      .attr("r", capitalSize);

    capitalLabels
      .selectAll("text")
      .data(capitals)
      .enter()
      .append("text")
      .attr("id", d => "burgLabel" + d.i)
      .attr("data-id", d => d.i)
      .attr("x", d => d.x)
      .attr("y", d => d.y)
      .attr("dy", `${capitalSize * -1.5}px`)
      .text(d => d.name);

    capitalAnchors
      .selectAll("use")
      .data(capitals.filter(c => c.port))
      .enter()
      .append("use")
      .attr("xlink:href", "#icon-anchor")
      .attr("data-id", d => d.i)
      .attr("x", d => rn(d.x - caSize * 0.47, 2))
      .attr("y", d => rn(d.y - caSize * 0.47, 2))
      .attr("width", caSize)
      .attr("height", caSize);

    // towns
    const towns = pack.burgs.filter(b => b.i && !b.capital && !b.removed);
    const townIcons = burgIcons.select("#towns");
    const townLabels = burgLabels.select("#towns");
    const townSize = townIcons.attr("size") || 0.5;
    const townsAnchors = anchors.selectAll("#towns");
    const taSize = townsAnchors.attr("size") || 1;

    townIcons
      .selectAll("circle")
      .data(towns)
      .enter()
      .append("circle")
      .attr("id", d => "burg" + d.i)
      .attr("data-id", d => d.i)
      .attr("cx", d => d.x)
      .attr("cy", d => d.y)
      .attr("r", townSize);

    townLabels
      .selectAll("text")
      .data(towns)
      .enter()
      .append("text")
      .attr("id", d => "burgLabel" + d.i)
      .attr("data-id", d => d.i)
      .attr("x", d => d.x)
      .attr("y", d => d.y)
      .attr("dy", `${townSize * -1.5}px`)
      .text(d => d.name);

    townsAnchors
      .selectAll("use")
      .data(towns.filter(c => c.port))
      .enter()
      .append("use")
      .attr("xlink:href", "#icon-anchor")
      .attr("data-id", d => d.i)
      .attr("x", d => rn(d.x - taSize * 0.47, 2))
      .attr("y", d => rn(d.y - taSize * 0.47, 2))
      .attr("width", taSize)
      .attr("height", taSize);

    TIME && console.timeEnd("drawBurgs");
  };

  // expand cultures across the map (Dijkstra-like algorithm)
  const expandStates = function () {
    TIME && console.time("expandStates");
    const {cells, states, cultures, burgs} = pack;

    cells.state = cells.state || new Uint16Array(cells.i.length);
    const queue = new PriorityQueue({comparator: (a, b) => a.p - b.p});
    const cost = [];

    const globalNeutralRate = byId("neutralInput")?.valueAsNumber || 1;
    const statesNeutralRate = byId("statesNeutral")?.valueAsNumber || 1;
    const neutral = (cells.i.length / 2) * globalNeutralRate * statesNeutralRate; // limit cost for state growth

    // remove state from all cells except of locked
    for (const cellId of cells.i) {
      const state = states[cells.state[cellId]];
      if (state.lock) continue;
      cells.state[cellId] = 0;
    }

    for (const state of states) {
      if (!state.i || state.removed) continue;

      const capitalCell = burgs[state.capital].cell;
      cells.state[capitalCell] = state.i;
      const cultureCenter = cultures[state.culture].center;
      const b = cells.biome[cultureCenter]; // state native biome
      queue.queue({e: state.center, p: 0, s: state.i, b});
      cost[state.center] = 1;
    }

    while (queue.length) {
      const next = queue.dequeue();
      const {e, p, s, b} = next;
      const {type, culture} = states[s];

      cells.c[e].forEach(e => {
        const state = states[cells.state[e]];
        if (state.lock) return; // do not overwrite cell of locked states
        if (cells.state[e] && e === state.center) return; // do not overwrite capital cells

        const cultureCost = culture === cells.culture[e] ? -9 : 100;
        const populationCost = cells.h[e] < 20 ? 0 : cells.s[e] ? Math.max(20 - cells.s[e], 0) : 5000;
        const biomeCost = getBiomeCost(b, cells.biome[e], type);
        const heightCost = getHeightCost(pack.features[cells.f[e]], cells.h[e], type);
        const riverCost = getRiverCost(cells.r[e], e, type);
        const typeCost = getTypeCost(cells.t[e], type);
        const cellCost = Math.max(cultureCost + populationCost + biomeCost + heightCost + riverCost + typeCost, 0);
        const totalCost = p + 10 + cellCost / states[s].expansionism;

        if (totalCost > neutral) return;

        if (!cost[e] || totalCost < cost[e]) {
          if (cells.h[e] >= 20) cells.state[e] = s; // assign state to cell
          cost[e] = totalCost;
          queue.queue({e, p: totalCost, s, b});
        }
      });
    }

    burgs.filter(b => b.i && !b.removed).forEach(b => (b.state = cells.state[b.cell])); // assign state to burgs

    function getBiomeCost(b, biome, type) {
      if (b === biome) return 10; // tiny penalty for native biome
      if (type === "Hunting") return biomesData.cost[biome] * 2; // non-native biome penalty for hunters
      if (type === "Nomadic" && biome > 4 && biome < 10) return biomesData.cost[biome] * 3; // forest biome penalty for nomads
      return biomesData.cost[biome]; // general non-native biome penalty
    }

    function getHeightCost(f, h, type) {
      if (type === "Lake" && f.type === "lake") return 10; // low lake crossing penalty for Lake cultures
      if (type === "Naval" && h < 20) return 300; // low sea crossing penalty for Navals
      if (type === "Nomadic" && h < 20) return 10000; // giant sea crossing penalty for Nomads
      if (h < 20) return 1000; // general sea crossing penalty
      if (type === "Highland" && h < 62) return 1100; // penalty for highlanders on lowlands
      if (type === "Highland") return 0; // no penalty for highlanders on highlands
      if (h >= 67) return 2200; // general mountains crossing penalty
      if (h >= 44) return 300; // general hills crossing penalty
      return 0;
    }

    function getRiverCost(r, i, type) {
      if (type === "River") return r ? 0 : 100; // penalty for river cultures
      if (!r) return 0; // no penalty for others if there is no river
      return minmax(cells.fl[i] / 10, 20, 100); // river penalty from 20 to 100 based on flux
    }

    function getTypeCost(t, type) {
      if (t === 1) return type === "Naval" || type === "Lake" ? 0 : type === "Nomadic" ? 60 : 20; // penalty for coastline
      if (t === 2) return type === "Naval" || type === "Nomadic" ? 30 : 0; // low penalty for land level 2 for Navals and nomads
      if (t !== -1) return type === "Naval" || type === "Lake" ? 100 : 0; // penalty for mainland for navals
      return 0;
    }

    TIME && console.timeEnd("expandStates");
  };

  const normalizeStates = function () {
    TIME && console.time("normalizeStates");
    const cells = pack.cells,
      burgs = pack.burgs;

    for (const i of cells.i) {
      if (cells.h[i] < 20 || cells.burg[i]) continue; // do not overwrite burgs
      if (pack.states[cells.state[i]]?.lock) continue; // do not overwrite cells of locks states
      if (cells.c[i].some(c => burgs[cells.burg[c]].capital)) continue; // do not overwrite near capital
      const neibs = cells.c[i].filter(c => cells.h[c] >= 20);
      const adversaries = neibs.filter(c => !pack.states[cells.state[c]]?.lock && cells.state[c] !== cells.state[i]);
      if (adversaries.length < 2) continue;
      const buddies = neibs.filter(c => !pack.states[cells.state[c]]?.lock && cells.state[c] === cells.state[i]);
      if (buddies.length > 2) continue;
      if (adversaries.length <= buddies.length) continue;
      cells.state[i] = cells.state[adversaries[0]];
    }
    TIME && console.timeEnd("normalizeStates");
  };

  // Resets the cultures of all burgs and states to their
  // cell or center cell's (respectively) culture.
  const updateCultures = function () {
    TIME && console.time("updateCulturesForBurgsAndStates");

    // Assign the culture associated with the burgs cell.
    pack.burgs = pack.burgs.map((burg, index) => {
      // Ignore metadata burg
      if (index === 0) {
        return burg;
      }
      return {...burg, culture: pack.cells.culture[burg.cell]};
    });

    // Assign the culture associated with the states' center cell.
    pack.states = pack.states.map((state, index) => {
      // Ignore neutrals state
      if (index === 0) {
        return state;
      }
      return {...state, culture: pack.cells.culture[state.center]};
    });

    TIME && console.timeEnd("updateCulturesForBurgsAndStates");
  };

  // calculate and draw curved state labels for a list of states
  const drawStateLabels = function (list) {
    TIME && console.time("drawStateLabels");
    const {cells, features, states} = pack;
    const paths = []; // text paths
    lineGen.curve(d3.curveBundle.beta(1));
    const mode = options.stateLabelsMode || "auto";

    for (const s of states) {
      if (!s.i || s.removed || s.lock || !s.cells || (list && !list.includes(s.i))) continue;

      const used = [];
      const visualCenter = findCell(s.pole[0], s.pole[1]);
      const start = cells.state[visualCenter] === s.i ? visualCenter : s.center;
      const hull = getHull(start, s.i, s.cells / 10);
      const points = [...hull].map(v => pack.vertices.p[v]);
      const delaunay = Delaunator.from(points);
      const voronoi = new Voronoi(delaunay, points, points.length);
      const chain = connectCenters(voronoi.vertices, s.pole[1]);
      const relaxed = chain.map(i => voronoi.vertices.p[i]).filter((p, i) => i % 15 === 0 || i + 1 === chain.length);
      paths.push([s.i, relaxed]);

      function getHull(start, state, maxLake) {
        const queue = [start];
        const hull = new Set();

        while (queue.length) {
          const q = queue.pop();
          const sameStateNeibs = cells.c[q].filter(c => cells.state[c] === state);

          cells.c[q].forEach(function (c, d) {
            const passableLake = features[cells.f[c]].type === "lake" && features[cells.f[c]].cells < maxLake;
            if (cells.b[c] || (cells.state[c] !== state && !passableLake)) return hull.add(cells.v[q][d]);

            const hasCoadjacentSameStateCells = sameStateNeibs.some(neib => cells.c[c].includes(neib));
            if (hull.size > 20 && !hasCoadjacentSameStateCells && !passableLake) return hull.add(cells.v[q][d]);

            if (used[c]) return;
            used[c] = 1;
            queue.push(c);
          });
        }

        return hull;
      }

      function connectCenters(c, y) {
        // check if vertex is inside the area
        const inside = c.p.map(function (p) {
          if (p[0] <= 0 || p[1] <= 0 || p[0] >= graphWidth || p[1] >= graphHeight) return false; // out of the screen
          return used[findCell(p[0], p[1])];
        });

        const pointsInside = d3.range(c.p.length).filter(i => inside[i]);
        if (!pointsInside.length) return [0];
        const h = c.p.length < 200 ? 0 : c.p.length < 600 ? 0.5 : 1; // power of horyzontality shift
        const end =
          pointsInside[
            d3.scan(
              pointsInside,
              (a, b) => c.p[a][0] - c.p[b][0] + (Math.abs(c.p[a][1] - y) - Math.abs(c.p[b][1] - y)) * h
            )
          ]; // left point
        const start =
          pointsInside[
            d3.scan(
              pointsInside,
              (a, b) => c.p[b][0] - c.p[a][0] - (Math.abs(c.p[b][1] - y) - Math.abs(c.p[a][1] - y)) * h
            )
          ]; // right point

        // connect leftmost and rightmost points with shortest path
        const queue = new PriorityQueue({comparator: (a, b) => a.p - b.p});
        const cost = [],
          from = [];
        queue.queue({e: start, p: 0});

        while (queue.length) {
          const next = queue.dequeue(),
            n = next.e,
            p = next.p;
          if (n === end) break;

          for (const v of c.v[n]) {
            if (v === -1) continue;
            const totalCost = p + (inside[v] ? 1 : 100);
            if (from[v] || totalCost >= cost[v]) continue;
            cost[v] = totalCost;
            from[v] = n;
            queue.queue({e: v, p: totalCost});
          }
        }

        // restore path
        const chain = [end];
        let cur = end;
        while (cur !== start) {
          cur = from[cur];
          if (inside[cur]) chain.push(cur);
        }
        return chain;
      }
    }

    void (function drawLabels() {
      const g = labels.select("#states");
      const t = defs.select("#textPaths");
      const displayed = layerIsOn("toggleLabels");
      if (!displayed) toggleLabels();

      // remove state labels to be redrawn
      for (const state of pack.states) {
        if (!state.i || state.removed || state.lock) continue;
        if (list && !list.includes(state.i)) continue;

        byId(`stateLabel${state.i}`)?.remove();
        byId(`textPath_stateLabel${state.i}`)?.remove();
      }

      const example = g.append("text").attr("x", 0).attr("x", 0).text("Average");
      const letterLength = example.node().getComputedTextLength() / 7; // average length of 1 letter

      paths.forEach(p => {
        const id = p[0];
        const state = states[p[0]];
        const {name, fullName} = state;

        const path = p[1].length > 1 ? round(lineGen(p[1])) : `M${p[1][0][0] - 50},${p[1][0][1]}h${100}`;
        const textPath = t
          .append("path")
          .attr("d", path)
          .attr("id", "textPath_stateLabel" + id);
        const pathLength = p[1].length > 1 ? textPath.node().getTotalLength() / letterLength : 0; // path length in letters

        const [lines, ratio] = getLines(mode, name, fullName, pathLength);

        // prolongate path if it's too short
        if (pathLength && pathLength < lines[0].length) {
          const points = p[1];
          const f = points[0];
          const l = points[points.length - 1];
          const [dx, dy] = [l[0] - f[0], l[1] - f[1]];
          const mod = Math.abs((letterLength * lines[0].length) / dx) / 2;
          points[0] = [rn(f[0] - dx * mod), rn(f[1] - dy * mod)];
          points[points.length - 1] = [rn(l[0] + dx * mod), rn(l[1] + dy * mod)];
          textPath.attr("d", round(lineGen(points)));
        }

        example.attr("font-size", ratio + "%");
        const top = (lines.length - 1) / -2; // y offset
        const spans = lines.map((l, d) => {
          example.text(l);
          const left = example.node().getBBox().width / -2; // x offset
          return `<tspan x=${rn(left, 1)} dy="${d ? 1 : top}em">${l}</tspan>`;
        });

        const el = g
          .append("text")
          .attr("id", "stateLabel" + id)
          .append("textPath")
          .attr("xlink:href", "#textPath_stateLabel" + id)
          .attr("startOffset", "50%")
          .attr("font-size", ratio + "%")
          .node();

        el.insertAdjacentHTML("afterbegin", spans.join(""));
        if (mode === "full" || lines.length === 1) return;

        // check whether multilined label is generally inside the state. If no, replace with short name label
        const cs = pack.cells.state;
        const b = el.parentNode.getBBox();
        const c1 = () => +cs[findCell(b.x, b.y)] === id;
        const c2 = () => +cs[findCell(b.x + b.width / 2, b.y)] === id;
        const c3 = () => +cs[findCell(b.x + b.width, b.y)] === id;
        const c4 = () => +cs[findCell(b.x + b.width, b.y + b.height)] === id;
        const c5 = () => +cs[findCell(b.x + b.width / 2, b.y + b.height)] === id;
        const c6 = () => +cs[findCell(b.x, b.y + b.height)] === id;
        if (c1() + c2() + c3() + c4() + c5() + c6() > 3) return; // generally inside => exit

        // move to one-line name
        const text = pathLength > fullName.length * 1.8 ? fullName : name;
        example.text(text);
        const left = example.node().getBBox().width / -2; // x offset
        el.innerHTML = `<tspan x="${left}px">${text}</tspan>`;

        const correctedRatio = minmax(rn((pathLength / text.length) * 60), 40, 130);
        el.setAttribute("font-size", correctedRatio + "%");
      });

      example.remove();
      if (!displayed) toggleLabels();
    })();

    function getLines(mode, name, fullName, pathLength) {
      // short name
      if (mode === "short" || (mode === "auto" && pathLength < name.length)) {
        const lines = splitInTwo(name);
        const ratio = pathLength / lines[0].length;
        return [lines, minmax(rn(ratio * 60), 50, 150)];
      }

      // full name: one line
      if (pathLength > fullName.length * 2.5) {
        const lines = [fullName];
        const ratio = pathLength / lines[0].length;
        return [lines, minmax(rn(ratio * 70), 70, 170)];
      }

      // full name: two lines
      const lines = splitInTwo(fullName);
      const ratio = pathLength / lines[0].length;
      return [lines, minmax(rn(ratio * 60), 70, 150)];
    }

    TIME && console.timeEnd("drawStateLabels");
  };

  // calculate states data like area, population etc.
  const collectStatistics = function () {
    TIME && console.time("collectStatistics");
    const {cells, states} = pack;

    states.forEach(s => {
      if (s.removed) return;
      s.cells = s.area = s.burgs = s.rural = s.urban = 0;
      s.neighbors = new Set();
    });

    for (const i of cells.i) {
      if (cells.h[i] < 20) continue;
      const s = cells.state[i];

      // check for neighboring states
      cells.c[i]
        .filter(c => cells.h[c] >= 20 && cells.state[c] !== s)
        .forEach(c => states[s].neighbors.add(cells.state[c]));

      // collect stats
      states[s].cells += 1;
      states[s].area += cells.area[i];
      states[s].rural += cells.pop[i];
      if (cells.burg[i]) {
        states[s].urban += pack.burgs[cells.burg[i]].population;
        states[s].burgs++;
      }
    }

    // convert neighbors Set object into array
    states.forEach(s => {
      if (!s.neighbors) return;
      s.neighbors = Array.from(s.neighbors);
    });

    TIME && console.timeEnd("collectStatistics");
  };

  const assignColors = function () {
    TIME && console.time("assignColors");
    const colors = ["#66c2a5", "#fc8d62", "#8da0cb", "#e78ac3", "#a6d854", "#ffd92f"]; // d3.schemeSet2;

    // assign basic color using greedy coloring algorithm
    pack.states.forEach(s => {
      if (!s.i || s.removed || s.lock) return;
      const neibs = s.neighbors;
      s.color = colors.find(c => neibs.every(n => pack.states[n].color !== c));
      if (!s.color) s.color = getRandomColor();
      colors.push(colors.shift());
    });

    // randomize each already used color a bit
    colors.forEach(c => {
      const sameColored = pack.states.filter(s => s.color === c && !s.lock);
      sameColored.forEach((s, d) => {
        if (!d) return;
        s.color = getMixedColor(s.color);
      });
    });

    TIME && console.timeEnd("assignColors");
  };

  const wars = {
    War: 6,
    Conflict: 2,
    Campaign: 4,
    Invasion: 2,
    Rebellion: 2,
    Conquest: 2,
    Intervention: 1,
    Expedition: 1,
    Crusade: 1
  };
  const generateCampaign = state => {
    const neighbors = state.neighbors.length ? state.neighbors : [0];
    return neighbors
      .map(i => {
        const name = i && P(0.8) ? pack.states[i].name : Names.getCultureShort(state.culture);
        const start = gauss(options.year - 100, 150, 1, options.year - 6);
        const end = start + gauss(4, 5, 1, options.year - start - 1);
        return {name: getAdjective(name) + " " + rw(wars), start, end};
      })
      .sort((a, b) => a.start - b.start);
  };

  // generate historical conflicts of each state
  const generateCampaigns = function () {
    pack.states.forEach(s => {
      if (!s.i || s.removed) return;
      s.campaigns = generateCampaign(s);
    });
  };

  // generate Diplomatic Relationships
  const generateDiplomacy = function () {
    TIME && console.time("generateDiplomacy");
    const cells = pack.cells,
      states = pack.states;
    const chronicle = (states[0].diplomacy = []);
    const valid = states.filter(s => s.i && !states.removed);

    const neibs = {Ally: 1, Friendly: 2, Neutral: 1, Suspicion: 10, Rival: 9}; // relations to neighbors
    const neibsOfNeibs = {Ally: 10, Friendly: 8, Neutral: 5, Suspicion: 1}; // relations to neighbors of neighbors
    const far = {Friendly: 1, Neutral: 12, Suspicion: 2, Unknown: 6}; // relations to other
    const navals = {Neutral: 1, Suspicion: 2, Rival: 1, Unknown: 1}; // relations of naval powers

    valid.forEach(s => (s.diplomacy = new Array(states.length).fill("x"))); // clear all relationships
    if (valid.length < 2) return; // no states to renerate relations with
    const areaMean = d3.mean(valid.map(s => s.area)); // average state area

    // generic relations
    for (let f = 1; f < states.length; f++) {
      if (states[f].removed) continue;

      if (states[f].diplomacy.includes("Vassal")) {
        // Vassals copy relations from their Suzerains
        const suzerain = states[f].diplomacy.indexOf("Vassal");

        for (let i = 1; i < states.length; i++) {
          if (i === f || i === suzerain) continue;
          states[f].diplomacy[i] = states[suzerain].diplomacy[i];
          if (states[suzerain].diplomacy[i] === "Suzerain") states[f].diplomacy[i] = "Ally";
          for (let e = 1; e < states.length; e++) {
            if (e === f || e === suzerain) continue;
            if (states[e].diplomacy[suzerain] === "Suzerain" || states[e].diplomacy[suzerain] === "Vassal") continue;
            states[e].diplomacy[f] = states[e].diplomacy[suzerain];
          }
        }
        continue;
      }

      for (let t = f + 1; t < states.length; t++) {
        if (states[t].removed) continue;

        if (states[t].diplomacy.includes("Vassal")) {
          const suzerain = states[t].diplomacy.indexOf("Vassal");
          states[f].diplomacy[t] = states[f].diplomacy[suzerain];
          continue;
        }

        const naval =
          states[f].type === "Naval" &&
          states[t].type === "Naval" &&
          cells.f[states[f].center] !== cells.f[states[t].center];
        const neib = naval ? false : states[f].neighbors.includes(t);
        const neibOfNeib =
          naval || neib
            ? false
            : states[f].neighbors
                .map(n => states[n].neighbors)
                .join("")
                .includes(t);

        let status = naval ? rw(navals) : neib ? rw(neibs) : neibOfNeib ? rw(neibsOfNeibs) : rw(far);

        // add Vassal
        if (
          neib &&
          P(0.8) &&
          states[f].area > areaMean &&
          states[t].area < areaMean &&
          states[f].area / states[t].area > 2
        )
          status = "Vassal";
        states[f].diplomacy[t] = status === "Vassal" ? "Suzerain" : status;
        states[t].diplomacy[f] = status;
      }
    }

    // declare wars
    for (let attacker = 1; attacker < states.length; attacker++) {
      const ad = states[attacker].diplomacy; // attacker relations;
      if (states[attacker].removed) continue;
      if (!ad.includes("Rival")) continue; // no rivals to attack
      if (ad.includes("Vassal")) continue; // not independent
      if (ad.includes("Enemy")) continue; // already at war

      // random independent rival
      const defender = ra(
        ad.map((r, d) => (r === "Rival" && !states[d].diplomacy.includes("Vassal") ? d : 0)).filter(d => d)
      );
      let ap = states[attacker].area * states[attacker].expansionism,
        dp = states[defender].area * states[defender].expansionism;
      if (ap < dp * gauss(1.6, 0.8, 0, 10, 2)) continue; // defender is too strong
      const an = states[attacker].name,
        dn = states[defender].name; // names
      const attackers = [attacker],
        defenders = [defender]; // attackers and defenders array
      const dd = states[defender].diplomacy; // defender relations;

      // start a war
      const war = [`${an}-${trimVowels(dn)}ian War`, `${an} declared a war on its rival ${dn}`];
      const end = options.year;
      const start = end - gauss(2, 2, 0, 5);
      states[attacker].campaigns.push({name: `${trimVowels(dn)}ian War`, start, end});
      states[defender].campaigns.push({name: `${trimVowels(an)}ian War`, start, end});

      // attacker vassals join the war
      ad.forEach((r, d) => {
        if (r === "Suzerain") {
          attackers.push(d);
          war.push(`${an}'s vassal ${states[d].name} joined the war on attackers side`);
        }
      });

      // defender vassals join the war
      dd.forEach((r, d) => {
        if (r === "Suzerain") {
          defenders.push(d);
          war.push(`${dn}'s vassal ${states[d].name} joined the war on defenders side`);
        }
      });

      ap = d3.sum(attackers.map(a => states[a].area * states[a].expansionism)); // attackers joined power
      dp = d3.sum(defenders.map(d => states[d].area * states[d].expansionism)); // defender joined power

      // defender allies join
      dd.forEach((r, d) => {
        if (r !== "Ally" || states[d].diplomacy.includes("Vassal")) return;
        if (states[d].diplomacy[attacker] !== "Rival" && ap / dp > 2 * gauss(1.6, 0.8, 0, 10, 2)) {
          const reason = states[d].diplomacy.includes("Enemy") ? "Being already at war," : `Frightened by ${an},`;
          war.push(`${reason} ${states[d].name} severed the defense pact with ${dn}`);
          dd[d] = states[d].diplomacy[defender] = "Suspicion";
          return;
        }
        defenders.push(d);
        dp += states[d].area * states[d].expansionism;
        war.push(`${dn}'s ally ${states[d].name} joined the war on defenders side`);

        // ally vassals join
        states[d].diplomacy
          .map((r, d) => (r === "Suzerain" ? d : 0))
          .filter(d => d)
          .forEach(v => {
            defenders.push(v);
            dp += states[v].area * states[v].expansionism;
            war.push(`${states[d].name}'s vassal ${states[v].name} joined the war on defenders side`);
          });
      });

      // attacker allies join if the defender is their rival or joined power > defenders power and defender is not an ally
      ad.forEach((r, d) => {
        if (r !== "Ally" || states[d].diplomacy.includes("Vassal") || defenders.includes(d)) return;
        const name = states[d].name;
        if (states[d].diplomacy[defender] !== "Rival" && (P(0.2) || ap <= dp * 1.2)) {
          war.push(`${an}'s ally ${name} avoided entering the war`);
          return;
        }
        const allies = states[d].diplomacy.map((r, d) => (r === "Ally" ? d : 0)).filter(d => d);
        if (allies.some(ally => defenders.includes(ally))) {
          war.push(`${an}'s ally ${name} did not join the war as its allies are in war on both sides`);
          return;
        }

        attackers.push(d);
        ap += states[d].area * states[d].expansionism;
        war.push(`${an}'s ally ${name} joined the war on attackers side`);

        // ally vassals join
        states[d].diplomacy
          .map((r, d) => (r === "Suzerain" ? d : 0))
          .filter(d => d)
          .forEach(v => {
            attackers.push(v);
            dp += states[v].area * states[v].expansionism;
            war.push(`${states[d].name}'s vassal ${states[v].name} joined the war on attackers side`);
          });
      });

      // change relations to Enemy for all participants
      attackers.forEach(a => defenders.forEach(d => (states[a].diplomacy[d] = states[d].diplomacy[a] = "Enemy")));
      chronicle.push(war); // add a record to diplomatical history
    }

    TIME && console.timeEnd("generateDiplomacy");
    //console.table(states.map(s => s.diplomacy));
  };

  // select a forms for listed or all valid states
  const defineStateForms = function (list) {
    TIME && console.time("defineStateForms");
    const states = pack.states.filter(s => s.i && !s.removed && !s.lock);
    if (states.length < 1) return;

    const generic = {Monarchy: 25, Republic: 2, Union: 1};
    const naval = {Monarchy: 25, Republic: 8, Union: 3};

    const median = d3.median(pack.states.map(s => s.area));
    const empireMin = states.map(s => s.area).sort((a, b) => b - a)[Math.max(Math.ceil(states.length ** 0.4) - 2, 0)];
    const expTiers = pack.states.map(s => {
      let tier = Math.min(Math.floor((s.area / median) * 2.6), 4);
      if (tier === 4 && s.area < empireMin) tier = 3;
      return tier;
    });

    const monarchy = ["Duchy", "Grand Duchy", "Principality", "Kingdom", "Empire"]; // per expansionism tier
    const republic = {
      Republic: 75,
      Federation: 4,
      "Trade Company": 4,
      "Most Serene Republic": 2,
      Oligarchy: 2,
      Tetrarchy: 1,
      Triumvirate: 1,
      Diarchy: 1,
      Junta: 1
    }; // weighted random
    const union = {
      Union: 3,
      League: 4,
      Confederation: 1,
      "United Kingdom": 1,
      "United Republic": 1,
      "United Provinces": 2,
      Commonwealth: 1,
      Heptarchy: 1
    }; // weighted random
    const theocracy = {Theocracy: 20, Brotherhood: 1, Thearchy: 2, See: 1, "Holy State": 1};
    const anarchy = {"Free Territory": 2, Council: 3, Commune: 1, Community: 1};

    for (const s of states) {
      if (list && !list.includes(s.i)) continue;
      const tier = expTiers[s.i];

      const religion = pack.cells.religion[s.center];
      const isTheocracy =
        (religion && pack.religions[religion].expansion === "state") ||
        (P(0.1) && ["Organized", "Cult"].includes(pack.religions[religion].type));
      const isAnarchy = P(0.01 - tier / 500);

      if (isTheocracy) s.form = "Theocracy";
      else if (isAnarchy) s.form = "Anarchy";
      else s.form = s.type === "Naval" ? rw(naval) : rw(generic);
      s.formName = selectForm(s, tier);
      s.fullName = getFullName(s);
    }

    function selectForm(s, tier) {
      const base = pack.cultures[s.culture].base;

      if (s.form === "Monarchy") {
        const form = monarchy[tier];
        // Default name depends on exponent tier, some culture bases have special names for tiers
        if (s.diplomacy) {
          if (
            form === "Duchy" &&
            s.neighbors.length > 1 &&
            rand(6) < s.neighbors.length &&
            s.diplomacy.includes("Vassal")
          )
            return "Marches"; // some vassal duchies on borderland
          if (base === 1 && P(0.3) && s.diplomacy.includes("Vassal")) return "Dominion"; // English vassals
          if (P(0.3) && s.diplomacy.includes("Vassal")) return "Protectorate"; // some vassals
        }

        if (base === 16 && (form === "Empire" || form === "Kingdom")) return "Khaganate"; // Turkic
        if (base === 5 && (form === "Empire" || form === "Kingdom")) return "Tsardom"; // Ruthenian
        if ([16, 31].includes(base) && (form === "Empire" || form === "Kingdom")) return "Khaganate"; // Turkic, Mongolian
        if (base === 12 && (form === "Kingdom" || form === "Grand Duchy")) return "Shogunate"; // Japanese
        if ([18, 17].includes(base) && form === "Empire") return "Caliphate"; // Arabic, Berber
        if (base === 18 && (form === "Grand Duchy" || form === "Duchy")) return "Emirate"; // Arabic
        if (base === 7 && (form === "Grand Duchy" || form === "Duchy")) return "Despotate"; // Greek
        if (base === 31 && (form === "Grand Duchy" || form === "Duchy")) return "Ulus"; // Mongolian
        if (base === 16 && (form === "Grand Duchy" || form === "Duchy")) return "Horde"; // Turkic
        if (base === 24 && (form === "Grand Duchy" || form === "Duchy")) return "Satrapy"; // Iranian
        return form;
      }

      if (s.form === "Republic") {
        // Default name is from weighted array, special case for small states with only 1 burg
        if (tier < 2 && s.burgs === 1) {
          if (trimVowels(s.name) === trimVowels(pack.burgs[s.capital].name)) {
            s.name = pack.burgs[s.capital].name;
            return "Free City";
          }
          if (P(0.3)) return "City-state";
        }
        return rw(republic);
      }

      if (s.form === "Union") return rw(union);
      if (s.form === "Anarchy") return rw(anarchy);

      if (s.form === "Theocracy") {
        // European
        if ([0, 1, 2, 3, 4, 6, 8, 9, 13, 15, 20].includes(base)) {
          if (P(0.1)) return "Divine " + monarchy[tier];
          if (tier < 2 && P(0.5)) return "Diocese";
          if (tier < 2 && P(0.5)) return "Bishopric";
        }
        if (P(0.9) && [7, 5].includes(base)) {
          // Greek, Ruthenian
          if (tier < 2) return "Eparchy";
          if (tier === 2) return "Exarchate";
          if (tier > 2) return "Patriarchate";
        }
        if (P(0.9) && [21, 16].includes(base)) return "Imamah"; // Nigerian, Turkish
        if (tier > 2 && P(0.8) && [18, 17, 28].includes(base)) return "Caliphate"; // Arabic, Berber, Swahili
        return rw(theocracy);
      }
    }

    TIME && console.timeEnd("defineStateForms");
  };

  // state forms requiring Adjective + Name, all other forms use scheme Form + Of + Name
  const adjForms = [
    "Empire",
    "Sultanate",
    "Khaganate",
    "Shogunate",
    "Caliphate",
    "Despotate",
    "Theocracy",
    "Oligarchy",
    "Union",
    "Confederation",
    "Trade Company",
    "League",
    "Tetrarchy",
    "Triumvirate",
    "Diarchy",
    "Horde",
    "Marches"
  ];

  const getFullName = function (s) {
    if (!s.formName) return s.name;
    if (!s.name && s.formName) return "The " + s.formName;
    const adjName = adjForms.includes(s.formName) && !/-| /.test(s.name);
    return adjName ? `${getAdjective(s.name)} ${s.formName}` : `${s.formName} of ${s.name}`;
  };

  const generateProvinces = function (regenerate = false, regenerateInLockedStates = false) {
    TIME && console.time("generateProvinces");
    const localSeed = regenerate ? generateSeed() : seed;
    Math.random = aleaPRNG(localSeed);

    const {cells, states, burgs} = pack;
    const provinces = [0];
    const provinceIds = new Uint16Array(cells.i.length);

    const isProvinceLocked = province => province.lock || (!regenerateInLockedStates && states[province.state]?.lock);
    const isProvinceCellLocked = cell => provinceIds[cell] && isProvinceLocked(provinces[provinceIds[cell]]);

    if (regenerate) {
      pack.provinces.forEach(province => {
        if (!province.i || province.removed || !isProvinceLocked(province)) return;

        const newId = provinces.length;
        for (const i of cells.i) {
          if (cells.province[i] === province.i) provinceIds[i] = newId;
        }

        province.i = newId;
        provinces.push(province);
      });
    }

    const percentage = +provincesInput.value;

    const max = percentage == 100 ? 1000 : gauss(20, 5, 5, 100) * percentage ** 0.5; // max growth

    const forms = {
      Monarchy: {County: 22, Earldom: 6, Shire: 2, Landgrave: 2, Margrave: 2, Barony: 2, Captaincy: 1, Seneschalty: 1},
      Republic: {Province: 6, Department: 2, Governorate: 2, District: 1, Canton: 1, Prefecture: 1},
      Theocracy: {Parish: 3, Deanery: 1},
      Union: {Province: 1, State: 1, Canton: 1, Republic: 1, County: 1, Council: 1},
      Anarchy: {Council: 1, Commune: 1, Community: 1, Tribe: 1},
      Wild: {Territory: 10, Land: 5, Region: 2, Tribe: 1, Clan: 1, Dependency: 1, Area: 1}
    };

    // generate provinces for selected burgs
    states.forEach(s => {
      s.provinces = [];
      if (!s.i || s.removed) return;
      if (provinces.length) s.provinces = provinces.filter(p => p.state === s.i).map(p => p.i); // locked provinces ids
      if (s.lock && !regenerateInLockedStates) return; // don't regenerate provinces of a locked state

      const stateBurgs = burgs
        .filter(b => b.state === s.i && !b.removed && !provinceIds[b.cell])
        .sort((a, b) => b.population * gauss(1, 0.2, 0.5, 1.5, 3) - a.population)
        .sort((a, b) => b.capital - a.capital);
      if (stateBurgs.length < 2) return; // at least 2 provinces are required
      const provincesNumber = Math.max(Math.ceil((stateBurgs.length * percentage) / 100), 2);

      const form = Object.assign({}, forms[s.form]);

      for (let i = 0; i < provincesNumber; i++) {
        const provinceId = provinces.length;
        const center = stateBurgs[i].cell;
        const burg = stateBurgs[i].i;
        const c = stateBurgs[i].culture;
        const nameByBurg = P(0.5);
        const name = nameByBurg ? stateBurgs[i].name : Names.getState(Names.getCultureShort(c), c);
        const formName = rw(form);
        form[formName] += 10;
        const fullName = name + " " + formName;
        const color = getMixedColor(s.color);
        const kinship = nameByBurg ? 0.8 : 0.4;
        const type = getType(center, burg.port);
        const coa = COA.generate(stateBurgs[i].coa, kinship, null, type);
        coa.shield = COA.getShield(c, s.i);

        s.provinces.push(provinceId);
        provinces.push({i: provinceId, state: s.i, center, burg, name, formName, fullName, color, coa});
      }
    });

    // expand generated provinces
    const queue = new PriorityQueue({comparator: (a, b) => a.p - b.p});
    const cost = [];

    provinces.forEach(p => {
      if (!p.i || p.removed || isProvinceLocked(p)) return;
      provinceIds[p.center] = p.i;
      queue.queue({e: p.center, p: 0, province: p.i, state: p.state});
      cost[p.center] = 1;
    });

    while (queue.length) {
      const {e, p, province, state} = queue.dequeue();

      cells.c[e].forEach(e => {
        if (isProvinceCellLocked(e)) return; // do not overwrite cell of locked provinces

        const land = cells.h[e] >= 20;
        if (!land && !cells.t[e]) return; // cannot pass deep ocean
        if (land && cells.state[e] !== state) return;
        const evevation = cells.h[e] >= 70 ? 100 : cells.h[e] >= 50 ? 30 : cells.h[e] >= 20 ? 10 : 100;
        const totalCost = p + evevation;

        if (totalCost > max) return;
        if (!cost[e] || totalCost < cost[e]) {
          if (land) provinceIds[e] = province; // assign province to a cell
          cost[e] = totalCost;
          queue.queue({e, p: totalCost, province, state});
        }
      });
    }

    // justify provinces shapes a bit
    for (const i of cells.i) {
      if (cells.burg[i]) continue; // do not overwrite burgs
      if (isProvinceCellLocked(i)) continue; // do not overwrite cell of locked provinces

      const neibs = cells.c[i]
        .filter(c => cells.state[c] === cells.state[i] && !isProvinceCellLocked(c))
        .map(c => provinceIds[c]);
      const adversaries = neibs.filter(c => c !== provinceIds[i]);
      if (adversaries.length < 2) continue;

      const buddies = neibs.filter(c => c === provinceIds[i]).length;
      if (buddies.length > 2) continue;

      const competitors = adversaries.map(p => adversaries.reduce((s, v) => (v === p ? s + 1 : s), 0));
      const max = d3.max(competitors);
      if (buddies >= max) continue;

      provinceIds[i] = adversaries[competitors.indexOf(max)];
    }

    // add "wild" provinces if some cells don't have a province assigned
    const noProvince = Array.from(cells.i).filter(i => cells.state[i] && !provinceIds[i]); // cells without province assigned
    states.forEach(s => {
      if (!s.i || s.removed) return;
      if (s.lock && !regenerateInLockedStates) return;
      if (!s.provinces.length) return;

      const coreProvinceNames = s.provinces.map(p => provinces[p]?.name);
      const colonyNamePool = [s.name, ...coreProvinceNames].filter(name => name && !/new/i.test(name));
      const getColonyName = () => {
        if (colonyNamePool.length < 1) return null;

        const index = rand(colonyNamePool.length - 1);
        const spliced = colonyNamePool.splice(index, 1);
        return spliced[0] ? `New ${spliced[0]}` : null;
      };

      let stateNoProvince = noProvince.filter(i => cells.state[i] === s.i && !provinceIds[i]);
      while (stateNoProvince.length) {
        // add new province
        const provinceId = provinces.length;
        const burgCell = stateNoProvince.find(i => cells.burg[i]);
        const center = burgCell ? burgCell : stateNoProvince[0];
        const burg = burgCell ? cells.burg[burgCell] : 0;
        provinceIds[center] = provinceId;

        // expand province
        const cost = [];
        cost[center] = 1;
        queue.queue({e: center, p: 0});
        while (queue.length) {
          const {e, p} = queue.dequeue();

          cells.c[e].forEach(nextCellId => {
            if (provinceIds[nextCellId]) return;
            const land = cells.h[nextCellId] >= 20;
            if (cells.state[nextCellId] && cells.state[nextCellId] !== s.i) return;
            const ter = land ? (cells.state[nextCellId] === s.i ? 3 : 20) : cells.t[nextCellId] ? 10 : 30;
            const totalCost = p + ter;

            if (totalCost > max) return;
            if (!cost[nextCellId] || totalCost < cost[nextCellId]) {
              if (land && cells.state[nextCellId] === s.i) provinceIds[nextCellId] = provinceId; // assign province to a cell
              cost[nextCellId] = totalCost;
              queue.queue({e: nextCellId, p: totalCost});
            }
          });
        }

        // generate "wild" province name
        const c = cells.culture[center];
        const f = pack.features[cells.f[center]];
        const color = getMixedColor(s.color);

        const provCells = stateNoProvince.filter(i => provinceIds[i] === provinceId);
        const singleIsle = provCells.length === f.cells && !provCells.find(i => cells.f[i] !== f.i);
        const isleGroup = !singleIsle && !provCells.find(i => pack.features[cells.f[i]].group !== "isle");
        const colony = !singleIsle && !isleGroup && P(0.5) && !isPassable(s.center, center);

        const name = (function () {
          const colonyName = colony && P(0.8) && getColonyName();
          if (colonyName) return colonyName;
          if (burgCell && P(0.5)) return burgs[burg].name;
          return Names.getState(Names.getCultureShort(c), c);
        })();

        const formName = (function () {
          if (singleIsle) return "Island";
          if (isleGroup) return "Islands";
          if (colony) return "Colony";
          return rw(forms["Wild"]);
        })();

        const fullName = name + " " + formName;

        const dominion = colony ? P(0.95) : singleIsle || isleGroup ? P(0.7) : P(0.3);
        const kinship = dominion ? 0 : 0.4;
        const type = getType(center, burgs[burg]?.port);
        const coa = COA.generate(s.coa, kinship, dominion, type);
        coa.shield = COA.getShield(c, s.i);

        provinces.push({i: provinceId, state: s.i, center, burg, name, formName, fullName, color, coa});
        s.provinces.push(provinceId);

        // check if there is a land way within the same state between two cells
        function isPassable(from, to) {
          if (cells.f[from] !== cells.f[to]) return false; // on different islands
          const queue = [from],
            used = new Uint8Array(cells.i.length),
            state = cells.state[from];
          while (queue.length) {
            const current = queue.pop();
            if (current === to) return true; // way is found
            cells.c[current].forEach(c => {
              if (used[c] || cells.h[c] < 20 || cells.state[c] !== state) return;
              queue.push(c);
              used[c] = 1;
            });
          }
          return false; // way is not found
        }

        // re-check
        stateNoProvince = noProvince.filter(i => cells.state[i] === s.i && !provinceIds[i]);
      }
    });

    cells.province = provinceIds;
    pack.provinces = provinces;

    TIME && console.timeEnd("generateProvinces");
  };

  return {
    generate,
    expandStates,
    normalizeStates,
    assignColors,
    drawBurgs,
    specifyBurgs,
    defineBurgFeatures,
    getType,
    drawStateLabels,
    collectStatistics,
    generateCampaign,
    generateCampaigns,
    generateDiplomacy,
    defineStateForms,
    getFullName,
    generateProvinces,
    updateCultures
  };
})();


"use strict";

window.Religions = (function () {
  // name generation approach and relative chance to be selected
  const approach = {
    Number: 1,
    Being: 3,
    Adjective: 5,
    "Color + Animal": 5,
    "Adjective + Animal": 5,
    "Adjective + Being": 5,
    "Adjective + Genitive": 1,
    "Color + Being": 3,
    "Color + Genitive": 3,
    "Being + of + Genitive": 2,
    "Being + of the + Genitive": 1,
    "Animal + of + Genitive": 1,
    "Adjective + Being + of + Genitive": 2,
    "Adjective + Animal + of + Genitive": 2
  };

  // turn weighted array into simple array
  const approaches = [];
  for (const a in approach) {
    for (let j = 0; j < approach[a]; j++) {
      approaches.push(a);
    }
  }

  const base = {
    number: ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve"],
    being: [
      "Ancestor",
      "Ancient",
      "Avatar",
      "Brother",
      "Champion",
      "Chief",
      "Council",
      "Creator",
      "Deity",
      "Divine One",
      "Elder",
      "Enlightened Being",
      "Father",
      "Forebear",
      "Forefather",
      "Giver",
      "God",
      "Goddess",
      "Guardian",
      "Guide",
      "Hierach",
      "Lady",
      "Lord",
      "Maker",
      "Master",
      "Mother",
      "Numen",
      "Oracle",
      "Overlord",
      "Protector",
      "Reaper",
      "Ruler",
      "Sage",
      "Seer",
      "Sister",
      "Spirit",
      "Supreme Being",
      "Transcendent",
      "Virgin"
    ],
    animal: [
      "Antelope",
      "Ape",
      "Badger",
      "Basilisk",
      "Bear",
      "Beaver",
      "Bison",
      "Boar",
      "Buffalo",
      "Camel",
      "Cat",
      "Centaur",
      "Cerberus",
      "Chimera",
      "Cobra",
      "Cockatrice",
      "Crane",
      "Crocodile",
      "Crow",
      "Cyclope",
      "Deer",
      "Dog",
      "Direwolf",
      "Drake",
      "Dragon",
      "Eagle",
      "Elephant",
      "Elk",
      "Falcon",
      "Fox",
      "Goat",
      "Goose",
      "Gorgon",
      "Gryphon",
      "Hare",
      "Hawk",
      "Heron",
      "Hippogriff",
      "Horse",
      "Hound",
      "Hyena",
      "Ibis",
      "Jackal",
      "Jaguar",
      "Kitsune",
      "Kraken",
      "Lark",
      "Leopard",
      "Lion",
      "Manticore",
      "Mantis",
      "Marten",
      "Minotaur",
      "Moose",
      "Mule",
      "Narwhal",
      "Owl",
      "Ox",
      "Panther",
      "Pegasus",
      "Phoenix",
      "Python",
      "Rat",
      "Raven",
      "Roc",
      "Rook",
      "Scorpion",
      "Serpent",
      "Shark",
      "Sheep",
      "Snake",
      "Sphinx",
      "Spider",
      "Swan",
      "Tiger",
      "Turtle",
      "Unicorn",
      "Viper",
      "Vulture",
      "Walrus",
      "Wolf",
      "Wolverine",
      "Worm",
      "Wyvern",
      "Yeti"
    ],
    adjective: [
      "Aggressive",
      "Almighty",
      "Ancient",
      "Beautiful",
      "Benevolent",
      "Big",
      "Blind",
      "Blond",
      "Bloody",
      "Brave",
      "Broken",
      "Brutal",
      "Burning",
      "Calm",
      "Celestial",
      "Cheerful",
      "Crazy",
      "Cruel",
      "Dead",
      "Deadly",
      "Devastating",
      "Distant",
      "Disturbing",
      "Divine",
      "Dying",
      "Eternal",
      "Ethernal",
      "Empyreal",
      "Enigmatic",
      "Enlightened",
      "Evil",
      "Explicit",
      "Fair",
      "Far",
      "Fat",
      "Fatal",
      "Favorable",
      "Flying",
      "Friendly",
      "Frozen",
      "Giant",
      "Good",
      "Grateful",
      "Great",
      "Happy",
      "High",
      "Holy",
      "Honest",
      "Huge",
      "Hungry",
      "Illustrious",
      "Immutable",
      "Ineffable",
      "Infallible",
      "Inherent",
      "Last",
      "Latter",
      "Lost",
      "Loud",
      "Lucky",
      "Mad",
      "Magical",
      "Main",
      "Major",
      "Marine",
      "Mythical",
      "Mystical",
      "Naval",
      "New",
      "Noble",
      "Old",
      "Otherworldly",
      "Patient",
      "Peaceful",
      "Pregnant",
      "Prime",
      "Proud",
      "Pure",
      "Radiant",
      "Resplendent",
      "Sacred",
      "Sacrosanct",
      "Sad",
      "Scary",
      "Secret",
      "Selected",
      "Serene",
      "Severe",
      "Silent",
      "Sleeping",
      "Slumbering",
      "Sovereign",
      "Strong",
      "Sunny",
      "Superior",
      "Supernatural",
      "Sustainable",
      "Transcendent",
      "Transcendental",
      "Troubled",
      "Unearthly",
      "Unfathomable",
      "Unhappy",
      "Unknown",
      "Unseen",
      "Waking",
      "Wild",
      "Wise",
      "Worried",
      "Young"
    ],
    genitive: [
      "Cold",
      "Day",
      "Death",
      "Doom",
      "Fate",
      "Fire",
      "Fog",
      "Frost",
      "Gates",
      "Heaven",
      "Home",
      "Ice",
      "Justice",
      "Life",
      "Light",
      "Lightning",
      "Love",
      "Nature",
      "Night",
      "Pain",
      "Snow",
      "Springs",
      "Summer",
      "Thunder",
      "Time",
      "Victory",
      "War",
      "Winter"
    ],
    theGenitive: [
      "Abyss",
      "Blood",
      "Dawn",
      "Earth",
      "East",
      "Eclipse",
      "Fall",
      "Harvest",
      "Moon",
      "North",
      "Peak",
      "Rainbow",
      "Sea",
      "Sky",
      "South",
      "Stars",
      "Storm",
      "Sun",
      "Tree",
      "Underworld",
      "West",
      "Wild",
      "Word",
      "World"
    ],
    color: [
      "Amber",
      "Black",
      "Blue",
      "Bright",
      "Bronze",
      "Brown",
      "Coral",
      "Crimson",
      "Dark",
      "Emerald",
      "Golden",
      "Green",
      "Grey",
      "Indigo",
      "Lavender",
      "Light",
      "Magenta",
      "Maroon",
      "Orange",
      "Pink",
      "Plum",
      "Purple",
      "Red",
      "Ruby",
      "Sapphire",
      "Teal",
      "Turquoise",
      "White",
      "Yellow"
    ]
  };

  const forms = {
    Folk: {
      Shamanism: 4,
      Animism: 4,
      Polytheism: 4,
      "Ancestor Worship": 2,
      "Nature Worship": 1,
      Totemism: 1
    },
    Organized: {
      Polytheism: 14,
      Monotheism: 12,
      Dualism: 6,
      Pantheism: 6,
      "Non-theism": 4
    },
    Cult: {
      Cult: 5,
      "Dark Cult": 5,
      Sect: 1
    },
    Heresy: {
      Heresy: 1
    }
  };

  const namingMethods = {
    Folk: {
      "Culture + type": 1
    },

    Organized: {
      "Random + type": 3,
      "Random + ism": 1,
      "Supreme + ism": 5,
      "Faith of + Supreme": 5,
      "Place + ism": 1,
      "Culture + ism": 2,
      "Place + ian + type": 6,
      "Culture + type": 4
    },

    Cult: {
      "Burg + ian + type": 2,
      "Random + ian + type": 1,
      "Type + of the + meaning": 2
    },

    Heresy: {
      "Burg + ian + type": 3,
      "Random + ism": 3,
      "Random + ian + type": 2,
      "Type + of the + meaning": 1
    }
  };

  const types = {
    Shamanism: {Beliefs: 3, Shamanism: 2, Druidism: 1, Spirits: 1},
    Animism: {Spirits: 3, Beliefs: 1},
    Polytheism: {Deities: 3, Faith: 1, Gods: 1, Pantheon: 1},
    "Ancestor worship": {Beliefs: 1, Forefathers: 2, Ancestors: 2},
    "Nature Worship": {Beliefs: 3, Druids: 1},
    Totemism: {Beliefs: 2, Totems: 2, Idols: 1},

    Monotheism: {Religion: 2, Church: 3, Faith: 1},
    Dualism: {Religion: 3, Faith: 1, Cult: 1},
    "Non-theism": {Beliefs: 3, Spirits: 1},

    Cult: {Cult: 4, Sect: 2, Arcanum: 1, Order: 1, Worship: 1},
    "Dark Cult": {Cult: 2, Blasphemy: 1, Circle: 1, Coven: 1, Idols: 1, Occultism: 1},
    Sect: {Sect: 3, Society: 1},

    Heresy: {
      Heresy: 3,
      Sect: 2,
      Apostates: 1,
      Brotherhood: 1,
      Circle: 1,
      Dissent: 1,
      Dissenters: 1,
      Iconoclasm: 1,
      Schism: 1,
      Society: 1
    }
  };

  const expansionismMap = {
    Folk: () => 0,
    Organized: () => gauss(5, 3, 0, 10, 1),
    Cult: () => gauss(0.5, 0.5, 0, 5, 1),
    Heresy: () => gauss(1, 0.5, 0, 5, 1)
  };

  function generate() {
    TIME && console.time("generateReligions");
    const lockedReligions = pack.religions?.filter(r => r.i && r.lock && !r.removed) || [];

    const folkReligions = generateFolkReligions();
    const organizedReligions = generateOrganizedReligions(+religionsInput.value, lockedReligions);

    const namedReligions = specifyReligions([...folkReligions, ...organizedReligions]);
    const indexedReligions = combineReligions(namedReligions, lockedReligions);
    const religionIds = expandReligions(indexedReligions);
    const religions = defineOrigins(religionIds, indexedReligions);

    pack.religions = religions;
    pack.cells.religion = religionIds;

    checkCenters();

    TIME && console.timeEnd("generateReligions");
  }

  function generateFolkReligions() {
    return pack.cultures
      .filter(c => c.i && !c.removed)
      .map(culture => ({type: "Folk", form: rw(forms.Folk), culture: culture.i, center: culture.center}));
  }

  function generateOrganizedReligions(desiredReligionNumber, lockedReligions) {
    const cells = pack.cells;
    const lockedReligionCount = lockedReligions.filter(({type}) => type !== "Folk").length || 0;
    const requiredReligionsNumber = desiredReligionNumber - lockedReligionCount;
    if (requiredReligionsNumber < 1) return [];

    const candidateCells = getCandidateCells();
    const religionCores = placeReligions();

    const cultsCount = Math.floor((rand(1, 4) / 10) * religionCores.length); // 10-40%
    const heresiesCount = Math.floor((rand(0, 3) / 10) * religionCores.length); // 0-30%
    const organizedCount = religionCores.length - cultsCount - heresiesCount;

    const getType = index => {
      if (index < organizedCount) return "Organized";
      if (index < organizedCount + cultsCount) return "Cult";
      return "Heresy";
    };

    return religionCores.map((cellId, index) => {
      const type = getType(index);
      const form = rw(forms[type]);
      const cultureId = cells.culture[cellId];

      return {type, form, culture: cultureId, center: cellId};
    });

    function placeReligions() {
      const religionCells = [];
      const religionsTree = d3.quadtree();

      // pre-populate with locked centers
      lockedReligions.forEach(({center}) => religionsTree.add(cells.p[center]));

      // min distance between religion inceptions
      const spacing = (graphWidth + graphHeight) / 2 / desiredReligionNumber;

      for (const cellId of candidateCells) {
        const [x, y] = cells.p[cellId];

        if (religionsTree.find(x, y, spacing) === undefined) {
          religionCells.push(cellId);
          religionsTree.add([x, y]);

          if (religionCells.length === requiredReligionsNumber) return religionCells;
        }
      }

      WARN && console.warn(`Placed only ${religionCells.length} of ${requiredReligionsNumber} religions`);
      return religionCells;
    }

    function getCandidateCells() {
      const validBurgs = pack.burgs.filter(b => b.i && !b.removed);

      if (validBurgs.length >= requiredReligionsNumber)
        return validBurgs.sort((a, b) => b.population - a.population).map(burg => burg.cell);
      return cells.i.filter(i => cells.s[i] > 2).sort((a, b) => cells.s[b] - cells.s[a]);
    }
  }

  function specifyReligions(newReligions) {
    const {cells, cultures} = pack;

    const rawReligions = newReligions.map(({type, form, culture: cultureId, center}) => {
      const supreme = getDeityName(cultureId);
      const deity = form === "Non-theism" || form === "Animism" ? null : supreme;

      const stateId = cells.state[center];

      let [name, expansion] = generateReligionName(type, form, supreme, center);
      if (expansion === "state" && !stateId) expansion = "global";

      const expansionism = expansionismMap[type]();
      const color = getReligionColor(cultures[cultureId], type);

      return {name, type, form, culture: cultureId, center, deity, expansion, expansionism, color};
    });

    return rawReligions;

    function getReligionColor(culture, type) {
      if (!culture.i) return getRandomColor();

      if (type === "Folk") return culture.color;
      if (type === "Heresy") return getMixedColor(culture.color, 0.35, 0.2);
      if (type === "Cult") return getMixedColor(culture.color, 0.5, 0);
      return getMixedColor(culture.color, 0.25, 0.4);
    }
  }

  // indexes, conditionally renames, and abbreviates religions
  function combineReligions(namedReligions, lockedReligions) {
    const indexedReligions = [{name: "No religion", i: 0}];

    const {lockedReligionQueue, highestLockedIndex, codes, numberLockedFolk} = parseLockedReligions();
    const maxIndex = Math.max(
      highestLockedIndex,
      namedReligions.length + lockedReligions.length + 1 - numberLockedFolk
    );

    for (let index = 1, progress = 0; index < maxIndex; index = indexedReligions.length) {
      // place locked religion back at its old index
      if (index === lockedReligionQueue[0]?.i) {
        const nextReligion = lockedReligionQueue.shift();
        indexedReligions.push(nextReligion);
        continue;
      }

      // slot the new religions
      if (progress < namedReligions.length) {
        const nextReligion = namedReligions[progress];
        progress++;

        if (
          nextReligion.type === "Folk" &&
          lockedReligions.some(({type, culture}) => type === "Folk" && culture === nextReligion.culture)
        )
          continue; // when there is a locked Folk religion for this culture discard duplicate

        const newName = renameOld(nextReligion);
        const code = abbreviate(newName, codes);
        codes.push(code);
        indexedReligions.push({...nextReligion, i: index, name: newName, code});
        continue;
      }

      indexedReligions.push({i: index, type: "Folk", culture: 0, name: "Removed religion", removed: true});
    }
    return indexedReligions;

    function parseLockedReligions() {
      // copy and sort the locked religions list
      const lockedReligionQueue = lockedReligions
        .map(religion => {
          // and filter their origins to locked religions
          let newOrigin = religion.origins.filter(n => lockedReligions.some(({i: index}) => index === n));
          if (newOrigin === []) newOrigin = [0];
          return {...religion, origins: newOrigin};
        })
        .sort((a, b) => a.i - b.i);

      const highestLockedIndex = Math.max(...lockedReligions.map(r => r.i));
      const codes = lockedReligions.length > 0 ? lockedReligions.map(r => r.code) : [];
      const numberLockedFolk = lockedReligions.filter(({type}) => type === "Folk").length;

      return {lockedReligionQueue, highestLockedIndex, codes, numberLockedFolk};
    }

    // prepend 'Old' to names of folk religions which have organized competitors
    function renameOld({name, type, culture: cultureId}) {
      if (type !== "Folk") return name;

      const haveOrganized =
        namedReligions.some(
          ({type, culture, expansion}) => culture === cultureId && type === "Organized" && expansion === "culture"
        ) ||
        lockedReligions.some(
          ({type, culture, expansion}) => culture === cultureId && type === "Organized" && expansion === "culture"
        );
      if (haveOrganized && name.slice(0, 3) !== "Old") return `Old ${name}`;
      return name;
    }
  }

  // finally generate and stores origins trees
  function defineOrigins(religionIds, indexedReligions) {
    const religionOriginsParamsMap = {
      Organized: {clusterSize: 100, maxReligions: 2},
      Cult: {clusterSize: 50, maxReligions: 3},
      Heresy: {clusterSize: 50, maxReligions: 4}
    };

    const origins = indexedReligions.map(({i, type, culture: cultureId, expansion, center}) => {
      if (i === 0) return null; // no religion
      if (type === "Folk") return [0]; // folk religions originate from its parent culture only

      const folkReligion = indexedReligions.find(({culture, type}) => type === "Folk" && culture === cultureId);
      const isFolkBased = folkReligion && cultureId && expansion === "culture" && each(2)(center);
      if (isFolkBased) return [folkReligion.i];

      const {clusterSize, maxReligions} = religionOriginsParamsMap[type];
      const fallbackOrigin = folkReligion?.i || 0;
      return getReligionsInRadius(pack.cells.c, center, religionIds, i, clusterSize, maxReligions, fallbackOrigin);
    });

    return indexedReligions.map((religion, index) => ({...religion, origins: origins[index]}));
  }

  function getReligionsInRadius(neighbors, center, religionIds, religionId, clusterSize, maxReligions, fallbackOrigin) {
    const foundReligions = new Set();
    const queue = [center];
    const checked = {};

    for (let size = 0; queue.length && size < clusterSize; size++) {
      const cellId = queue.shift();
      checked[cellId] = true;

      for (const neibId of neighbors[cellId]) {
        if (checked[neibId]) continue;
        checked[neibId] = true;

        const neibReligion = religionIds[neibId];
        if (neibReligion && neibReligion < religionId) foundReligions.add(neibReligion);
        if (foundReligions.size >= maxReligions) return [...foundReligions];
        queue.push(neibId);
      }
    }

    return foundReligions.size ? [...foundReligions] : [fallbackOrigin];
  }

  // growth algorithm to assign cells to religions
  function expandReligions(religions) {
    const cells = pack.cells;
    const religionIds = spreadFolkReligions(religions);

    const queue = new PriorityQueue({comparator: (a, b) => a.p - b.p});
    const cost = [];

    const maxExpansionCost = (cells.i.length / 20) * neutralInput.value; // limit cost for organized religions growth

    const biomePassageCost = cellId => biomesData.cost[cells.biome[cellId]];

    religions
      .filter(r => r.i && !r.lock && r.type !== "Folk" && !r.removed)
      .forEach(r => {
        religionIds[r.center] = r.i;
        queue.queue({e: r.center, p: 0, r: r.i, s: cells.state[r.center]});
        cost[r.center] = 1;
      });

    const religionsMap = new Map(religions.map(r => [r.i, r]));

    const isMainRoad = cellId => cells.road[cellId] - cells.crossroad[cellId] > 4;
    const isTrail = cellId => cells.h[cellId] > 19 && cells.road[cellId] - cells.crossroad[cellId] === 1;
    const isSeaRoute = cellId => cells.h[cellId] < 20 && cells.road[cellId];
    const isWater = cellId => cells.h[cellId] < 20;

    while (queue.length) {
      const {e: cellId, p, r, s: state} = queue.dequeue();
      const {culture, expansion, expansionism} = religionsMap.get(r);

      cells.c[cellId].forEach(nextCell => {
        if (expansion === "culture" && culture !== cells.culture[nextCell]) return;
        if (expansion === "state" && state !== cells.state[nextCell]) return;
        if (religionsMap.get(religionIds[nextCell])?.lock) return;

        const cultureCost = culture !== cells.culture[nextCell] ? 10 : 0;
        const stateCost = state !== cells.state[nextCell] ? 10 : 0;
        const passageCost = getPassageCost(nextCell);

        const cellCost = cultureCost + stateCost + passageCost;
        const totalCost = p + 10 + cellCost / expansionism;
        if (totalCost > maxExpansionCost) return;

        if (!cost[nextCell] || totalCost < cost[nextCell]) {
          if (cells.culture[nextCell]) religionIds[nextCell] = r; // assign religion to cell
          cost[nextCell] = totalCost;

          queue.queue({e: nextCell, p: totalCost, r, s: state});
        }
      });
    }

    return religionIds;

    function getPassageCost(cellId) {
      if (isWater(cellId)) return isSeaRoute ? 50 : 500;
      if (isMainRoad(cellId)) return 1;
      const biomeCost = biomePassageCost(cellId);
      return isTrail(cellId) ? biomeCost / 1.5 : biomeCost;
    }
  }

  // folk religions initially get all cells of their culture, and locked religions are retained
  function spreadFolkReligions(religions) {
    const cells = pack.cells;
    const hasPrior = cells.religion && true;
    const religionIds = new Uint16Array(cells.i.length);

    const folkReligions = religions.filter(religion => religion.type === "Folk" && !religion.removed);
    const cultureToReligionMap = new Map(folkReligions.map(({i, culture}) => [culture, i]));

    for (const cellId of cells.i) {
      const oldId = (hasPrior && cells.religion[cellId]) || 0;
      if (oldId && religions[oldId]?.lock && !religions[oldId]?.removed) {
        religionIds[cellId] = oldId;
        continue;
      }
      const cultureId = cells.culture[cellId];
      religionIds[cellId] = cultureToReligionMap.get(cultureId) || 0;
    }

    return religionIds;
  }

  function checkCenters() {
    const cells = pack.cells;
    pack.religions.forEach(r => {
      if (!r.i) return;
      // move religion center if it's not within religion area after expansion
      if (cells.religion[r.center] === r.i) return; // in area
      const firstCell = cells.i.find(i => cells.religion[i] === r.i);
      const cultureHome = pack.cultures[r.culture]?.center;
      if (firstCell) r.center = firstCell; // move center, othervise it's an extinct religion
      else if (r.type === "Folk" && cultureHome) r.center = cultureHome; // reset extinct culture centers
    });
  }

  function recalculate() {
    const newReligionIds = expandReligions(pack.religions);
    pack.cells.religion = newReligionIds;

    checkCenters();
  }

  const add = function (center) {
    const {cells, cultures, religions} = pack;
    const religionId = cells.religion[center];
    const i = religions.length;

    const cultureId = cells.culture[center];
    const missingFolk =
      cultureId !== 0 &&
      !religions.some(({type, culture, removed}) => type === "Folk" && culture === cultureId && !removed);
    const color = missingFolk ? cultures[cultureId].color : getMixedColor(religions[religionId].color, 0.3, 0);

    const type = missingFolk
      ? "Folk"
      : religions[religionId].type === "Organized"
      ? rw({Organized: 4, Cult: 1, Heresy: 2})
      : rw({Organized: 5, Cult: 2});
    const form = rw(forms[type]);
    const deity =
      type === "Heresy"
        ? religions[religionId].deity
        : form === "Non-theism" || form === "Animism"
        ? null
        : getDeityName(cultureId);

    const [name, expansion] = generateReligionName(type, form, deity, center);

    const formName = type === "Heresy" ? religions[religionId].form : form;
    const code = abbreviate(
      name,
      religions.map(r => r.code)
    );
    const influences = getReligionsInRadius(cells.c, center, cells.religion, i, 25, 3, 0);
    const origins = type === "Folk" ? [0] : influences;

    religions.push({
      i,
      name,
      color,
      culture: cultureId,
      type,
      form: formName,
      deity,
      expansion,
      expansionism: expansionismMap[type](),
      center,
      cells: 0,
      area: 0,
      rural: 0,
      urban: 0,
      origins,
      code
    });
    cells.religion[center] = i;
  };

  function updateCultures() {
    pack.religions = pack.religions.map((religion, index) => {
      if (index === 0) return religion;
      return {...religion, culture: pack.cells.culture[religion.center]};
    });
  }

  // get supreme deity name
  const getDeityName = function (culture) {
    if (culture === undefined) {
      ERROR && console.error("Please define a culture");
      return;
    }
    const meaning = generateMeaning();
    const cultureName = Names.getCulture(culture, null, null, "", 0.8);
    return cultureName + ", The " + meaning;
  };

  function generateMeaning() {
    const a = ra(approaches); // select generation approach
    if (a === "Number") return ra(base.number);
    if (a === "Being") return ra(base.being);
    if (a === "Adjective") return ra(base.adjective);
    if (a === "Color + Animal") return `${ra(base.color)} ${ra(base.animal)}`;
    if (a === "Adjective + Animal") return `${ra(base.adjective)} ${ra(base.animal)}`;
    if (a === "Adjective + Being") return `${ra(base.adjective)} ${ra(base.being)}`;
    if (a === "Adjective + Genitive") return `${ra(base.adjective)} ${ra(base.genitive)}`;
    if (a === "Color + Being") return `${ra(base.color)} ${ra(base.being)}`;
    if (a === "Color + Genitive") return `${ra(base.color)} ${ra(base.genitive)}`;
    if (a === "Being + of + Genitive") return `${ra(base.being)} of ${ra(base.genitive)}`;
    if (a === "Being + of the + Genitive") return `${ra(base.being)} of the ${ra(base.theGenitive)}`;
    if (a === "Animal + of + Genitive") return `${ra(base.animal)} of ${ra(base.genitive)}`;
    if (a === "Adjective + Being + of + Genitive")
      return `${ra(base.adjective)} ${ra(base.being)} of ${ra(base.genitive)}`;
    if (a === "Adjective + Animal + of + Genitive")
      return `${ra(base.adjective)} ${ra(base.animal)} of ${ra(base.genitive)}`;

    ERROR && console.error("Unkown generation approach");
  }

  function generateReligionName(variety, form, deity, center) {
    const {cells, cultures, burgs, states} = pack;

    const random = () => Names.getCulture(cells.culture[center], null, null, "", 0);
    const type = rw(types[form]);
    const supreme = deity.split(/[ ,]+/)[0];
    const culture = cultures[cells.culture[center]].name;

    const place = adj => {
      const burgId = cells.burg[center];
      const stateId = cells.state[center];

      const base = burgId ? burgs[burgId].name : states[stateId].name;
      let name = trimVowels(base.split(/[ ,]+/)[0]);
      return adj ? getAdjective(name) : name;
    };

    const m = rw(namingMethods[variety]);
    if (m === "Random + type") return [random() + " " + type, "global"];
    if (m === "Random + ism") return [trimVowels(random()) + "ism", "global"];
    if (m === "Supreme + ism" && deity) return [trimVowels(supreme) + "ism", "global"];
    if (m === "Faith of + Supreme" && deity)
      return [ra(["Faith", "Way", "Path", "Word", "Witnesses"]) + " of " + supreme, "global"];
    if (m === "Place + ism") return [place() + "ism", "state"];
    if (m === "Culture + ism") return [trimVowels(culture) + "ism", "culture"];
    if (m === "Place + ian + type") return [place("adj") + " " + type, "state"];
    if (m === "Culture + type") return [culture + " " + type, "culture"];
    if (m === "Burg + ian + type") return [`${place("adj")} ${type}`, "global"];
    if (m === "Random + ian + type") return [`${getAdjective(random())} ${type}`, "global"];
    if (m === "Type + of the + meaning") return [`${type} of the ${generateMeaning()}`, "global"];
    return [trimVowels(random()) + "ism", "global"]; // else
  }

  return {generate, add, getDeityName, updateCultures, recalculate};
})();



function drawStates() {
  TIME && console.time("drawStates");
  regions.selectAll("path").remove();

  const {cells, vertices, features} = pack;
  const states = pack.states;
  const n = cells.i.length;

  const used = new Uint8Array(cells.i.length);
  const vArray = new Array(states.length); // store vertices array
  const body = new Array(states.length).fill(""); // path around each state
  const gap = new Array(states.length).fill(""); // path along water for each state to fill the gaps
  const halo = new Array(states.length).fill(""); // path around states, but not lakes

  const getStringPoint = v => vertices.p[v[0]].join(",");

  // define inner-state lakes to omit on border render
  const innerLakes = features.map(feature => {
    if (feature.type !== "lake") return false;
    if (!feature.shoreline) Lakes.getShoreline(feature);

    const states = feature.shoreline.map(i => cells.state[i]);
    return new Set(states).size > 1 ? false : true;
  });

  for (const i of cells.i) {
    if (!cells.state[i] || used[i]) continue;
    const state = cells.state[i];

    const onborder = cells.c[i].some(n => cells.state[n] !== state);
    if (!onborder) continue;

    const borderWith = cells.c[i].map(c => cells.state[c]).find(n => n !== state);
    const vertex = cells.v[i].find(v => vertices.c[v].some(i => cells.state[i] === borderWith));
    const chain = connectVertices(vertex, state);

    const noInnerLakes = chain.filter(v => v[1] !== "innerLake");
    if (noInnerLakes.length < 3) continue;

    // get path around the state
    if (!vArray[state]) vArray[state] = [];
    const points = noInnerLakes.map(v => vertices.p[v[0]]);
    vArray[state].push(points);
    body[state] += "M" + points.join("L");

    // connect path for halo
    let discontinued = true;
    halo[state] += noInnerLakes
      .map(v => {
        if (v[1] === "border") {
          discontinued = true;
          return "";
        }

        const operation = discontinued ? "M" : "L";
        discontinued = false;
        return `${operation}${getStringPoint(v)}`;
      })
      .join("");

    // connect gaps between state and water into a single path
    discontinued = true;
    gap[state] += chain
      .map(v => {
        if (v[1] === "land") {
          discontinued = true;
          return "";
        }

        const operation = discontinued ? "M" : "L";
        discontinued = false;
        return `${operation}${getStringPoint(v)}`;
      })
      .join("");
  }

  // find state visual center
  vArray.forEach((ar, i) => {
    const sorted = ar.sort((a, b) => b.length - a.length); // sort by points number
    states[i].pole = polylabel(sorted, 1.0); // pole of inaccessibility
  });

  const bodyData = body.map((p, s) => [p.length > 10 ? p : null, s, states[s].color]).filter(d => d[0]);
  const gapData = gap.map((p, s) => [p.length > 10 ? p : null, s, states[s].color]).filter(d => d[0]);
  const haloData = halo.map((p, s) => [p.length > 10 ? p : null, s, states[s].color]).filter(d => d[0]);

  const bodyString = bodyData.map(d => `<path id="state${d[1]}" d="${d[0]}" fill="${d[2]}" stroke="none"/>`).join("");
  const gapString = gapData.map(d => `<path id="state-gap${d[1]}" d="${d[0]}" fill="none" stroke="${d[2]}"/>`).join("");
  const clipString = bodyData
    .map(d => `<clipPath id="state-clip${d[1]}"><use href="#state${d[1]}"/></clipPath>`)
    .join("");
  const haloString = haloData
    .map(
      d =>
        `<path id="state-border${d[1]}" d="${d[0]}" clip-path="url(#state-clip${d[1]})" stroke="${
          d3.color(d[2]) ? d3.color(d[2]).darker().hex() : "#666666"
        }"/>`
    )
    .join("");

  statesBody.html(bodyString + gapString);
  defs.select("#statePaths").html(clipString);
  statesHalo.html(haloString);

  // connect vertices to chain
  function connectVertices(start, state) {
    const chain = []; // vertices chain to form a path
    const getType = c => {
      const borderCell = c.find(i => cells.b[i]);
      if (borderCell) return "border";

      const waterCell = c.find(i => cells.h[i] < 20);
      if (!waterCell) return "land";
      if (innerLakes[cells.f[waterCell]]) return "innerLake";
      return features[cells.f[waterCell]].type;
    };

    for (let i = 0, current = start; i === 0 || (current !== start && i < 20000); i++) {
      const prev = chain.length ? chain[chain.length - 1][0] : -1; // previous vertex in chain

      const c = vertices.c[current]; // cells adjacent to vertex
      chain.push([current, getType(c)]); // add current vertex to sequence

      c.filter(c => cells.state[c] === state).forEach(c => (used[c] = 1));
      const c0 = c[0] >= n || cells.state[c[0]] !== state;
      const c1 = c[1] >= n || cells.state[c[1]] !== state;
      const c2 = c[2] >= n || cells.state[c[2]] !== state;

      const v = vertices.v[current]; // neighboring vertices

      if (v[0] !== prev && c0 !== c1) current = v[0];
      else if (v[1] !== prev && c1 !== c2) current = v[1];
      else if (v[2] !== prev && c0 !== c2) current = v[2];

      if (current === prev) {
        ERROR && console.error("Next vertex is not found");
        break;
      }
    }

    if (chain.length) chain.push(chain[0]);
    return chain;
  }

  invokeActiveZooming();
  TIME && console.timeEnd("drawStates");
}

// draw state and province borders
function drawBorders() {
  TIME && console.time("drawBorders");
  borders.selectAll("path").remove();

  const {cells, vertices} = pack;
  const n = cells.i.length;

  const sPath = [];
  const pPath = [];

  const sUsed = new Array(pack.states.length).fill("").map(_ => []);
  const pUsed = new Array(pack.provinces.length).fill("").map(_ => []);

  for (let i = 0; i < cells.i.length; i++) {
    if (!cells.state[i]) continue;
    const p = cells.province[i];
    const s = cells.state[i];

    // if cell is on province border
    const provToCell = cells.c[i].find(
      n => cells.state[n] === s && p > cells.province[n] && pUsed[p][n] !== cells.province[n]
    );

    if (provToCell) {
      const provTo = cells.province[provToCell];
      pUsed[p][provToCell] = provTo;
      const vertex = cells.v[i].find(v => vertices.c[v].some(i => cells.province[i] === provTo));
      const chain = connectVertices(vertex, p, cells.province, provTo, pUsed);

      if (chain.length > 1) {
        pPath.push("M" + chain.map(c => vertices.p[c]).join(" "));
        i--;
        continue;
      }
    }

    // if cell is on state border
    const stateToCell = cells.c[i].find(n => cells.h[n] >= 20 && s > cells.state[n] && sUsed[s][n] !== cells.state[n]);
    if (stateToCell !== undefined) {
      const stateTo = cells.state[stateToCell];
      sUsed[s][stateToCell] = stateTo;
      const vertex = cells.v[i].find(v => vertices.c[v].some(i => cells.h[i] >= 20 && cells.state[i] === stateTo));
      const chain = connectVertices(vertex, s, cells.state, stateTo, sUsed);

      if (chain.length > 1) {
        sPath.push("M" + chain.map(c => vertices.p[c]).join(" "));
        i--;
        continue;
      }
    }
  }

  stateBorders.append("path").attr("d", sPath.join(" "));
  provinceBorders.append("path").attr("d", pPath.join(" "));

  // connect vertices to chain
  function connectVertices(current, f, array, t, used) {
    let chain = [];
    const checkCell = c => c >= n || array[c] !== f;
    const checkVertex = v =>
      vertices.c[v].some(c => array[c] === f) && vertices.c[v].some(c => array[c] === t && cells.h[c] >= 20);

    // find starting vertex
    for (let i = 0; i < 1000; i++) {
      if (i === 999) ERROR && console.error("Find starting vertex: limit is reached", current, f, t);
      const p = chain[chain.length - 2] || -1; // previous vertex
      const v = vertices.v[current],
        c = vertices.c[current];

      const v0 = checkCell(c[0]) !== checkCell(c[1]) && checkVertex(v[0]);
      const v1 = checkCell(c[1]) !== checkCell(c[2]) && checkVertex(v[1]);
      const v2 = checkCell(c[0]) !== checkCell(c[2]) && checkVertex(v[2]);
      if (v0 + v1 + v2 === 1) break;
      current = v0 && p !== v[0] ? v[0] : v1 && p !== v[1] ? v[1] : v[2];

      if (current === chain[0]) break;
      if (current === p) return [];
      chain.push(current);
    }

    chain = [current]; // vertices chain to form a path
    // find path
    for (let i = 0; i < 1000; i++) {
      if (i === 999) ERROR && console.error("Find path: limit is reached", current, f, t);
      const p = chain[chain.length - 2] || -1; // previous vertex
      const v = vertices.v[current],
        c = vertices.c[current];
      c.filter(c => array[c] === t).forEach(c => (used[f][c] = t));

      const v0 = checkCell(c[0]) !== checkCell(c[1]) && checkVertex(v[0]);
      const v1 = checkCell(c[1]) !== checkCell(c[2]) && checkVertex(v[1]);
      const v2 = checkCell(c[0]) !== checkCell(c[2]) && checkVertex(v[2]);
      current = v0 && p !== v[0] ? v[0] : v1 && p !== v[1] ? v[1] : v[2];

      if (current === p) break;
      if (current === chain[chain.length - 1]) break;
      if (chain.length > 1 && v0 + v1 + v2 < 2) break;
      chain.push(current);
      if (current === chain[0]) break;
    }

    return chain;
  }

  TIME && console.timeEnd("drawBorders");
}

```

### Analysis



cells.t = tile.shore_distance
cells.haven = tile.closest_water
cells.harbor = tile.water_count

* Rank Cells
* Generate cultures
* Expand cultures
* Generate Burgs and States
* Generate Religions
* Define State Forms (Burgs and States)
* Generate Provinces (Burgs and States)
* Define Burg Features (Burgs and States)
* Draw States?
* Draw Borders?
* Draw State Labels (Burgs and States)

### Analysis: Rank Cells

* let suitability = array of int the length of the tiles 
* let population = array of float the length of the tiles -- TODO: Why is population a float?
* let fl_mean = average of water flow on tiles
* let fl_max = max of water flow on tiles
* let area_mean = mean area of cells
* for each cell
  * if cell is ocean, continue
  * let s = biomes_data.habitability(tile biome)
  * if s == 0 then continue; // biome is uninhabitable
  * if fl_mean > 0: s += normalize(tile waterflow + tile confluence, fl_mean, fl_max) * 250; -- "big rivers and confluences are valued" 
    * normalize(value, min, max) = clamp((val - min)/max-min),0,1)
  * s -= (tile.elevation_scaled - 50) / 5 -- "low elevation is valued"
  * if cell is coast of ocean or lake: 
    * if cell has a river (of subtantial flow): s += 15 -- "estuary is valued" -- Cell is in a river mouth. For me, I'll have to base it on waterflow instead.
    * if cell is next to a lake:
      * if lake is "freshwater": s += 30
      * else if lake is salt: s += 10
      * else if lake is frozen: s += 1
      * else if lake is dry: s -= 5
      * else if lake is sinkhole: s -= 5
      * else if lake is lava: s -= 30 -- TODO: WTF? How do we get lava?
    * else if cell is an ocean
      * s += 5 -- "ocean coast is valued" 
      * if cell is a harbor: s += 20 -- TODO: How do I determine this.
  * tile.pop_scale (cells.s) = s/5 -- This is the general population rate
  * tile.population = if tiles.pop_scale > 0 ? (cells.pop_scale * cells.area) / area_mean : 0 -- population is scaled by the area -- TODO: How do I find the area?

### My Algorithm: Rank Cells

* *Input*: estuary_threshold -- amount of flow on a coastal tile to consider it an estuary, which increases population
* New Field on Tiles: Habitability: f64
* New Field on Tiles: Population: f64
* let biomes_data = map of biomes_data by name
* let flow_sum = 0;
* let flow_max = 0;
* let (tile_map,work_queue) = map and vec of all tiles, while creating:
  * flow_sum += tile.water_flow
  * flow_max = flow_max.max(tile.water_flow)
* let flow_mean = flow_sum / tile.count
* let flow_divisor = flow_max - flow_mean
* while fid = work_queue.pop:
  * let habitability = 0;
  * let population = 0;
  * lifetime block:
    * let tile = tile_map.get(fid)
    * let suitability = biomes_data.get(tile.biome) or error
    * if suitability > 0:
      * if flow_mean > 0:
        * suitability += ((tile.water_flow - flow_mean)/flow_divisor).clamp(0,1) * 250; // TODO: Is there a number I can just multiply by here?
      * suitability -= (tile.elevation_scaled - 50) / 5 -- low elevation is better
      * if cell.shore_distance == 1:
        * if cell.water_flow > estuary_threshold: suitability += 15 -- estuary
        * if cell.closest_water 
          * if tile_map[cell.closest_water].lake_type:
            * match lake_type
              * lake_type is fresh: suitability += 30
              * salt: suitability += 10
              * frozen: suitability += 1
              * pluvial or marsh: suitability -= 2
              * dry: suitability -= 5
          * else if tile_map[cell.closest_water].is_ocean:
            * suitability += 5
            * if cell.water_count == 1: suitability += 20 -- this means it's a single cell of ocean, which implies a small bay, which could be a harbor
      * habitability = suitability / 5
      * population = (habitability * tile.area) / area_mean
  * let tile = tile_map.get_mut(&fid)
    * tile.habitability = habitability;
    * tile.population = population;
* Write tile_map to layer.

### Analysis: Generate Cultures

* *Input*: num_cultures (culturesInput.value = number of cultures)
* *Input*: culture_set (culturesSet.selectedOptions[0].dataset = selected culture set to use (a choice of several))
* let culture_ids = Create array of culture IDs for each cell (cells.i is an array of indexes)
* let count = minimum of num_cultures and culture_set.length

* let populated = filter of tiles where tile.pop_scale > 0
* if populated.len < (count * 25):
  * count (same var as above) = (populated.len / 50).floor
  * if count == 0:
    * report warning "The climate is too harsh and the people cannot live here, no cultures, nations, or cities can be created." -- TODO: Except, the problem could be that there's no land, or all the elevations are too high, or something else.
    * list of cultures consists only of "Wildlands"
    * map cultures to tiles
    * return
  * else:
    * report warning "Not enough populated cells for requested number of cultures." -- TODO: This should also warn about why the cells can't be populated.

* let cultures = select_cultures(count) 
* let centers = d3.quadtree -- This seems to be some sort of graphical index thingie
* let colors = get_colors(count) -- I think this is just colors for the map and can safely be ignored
* let codes = []
* for culture in cultures:
  * let new_id = generate a new idea
  * let cell = culture.center = place_center(culture.sort ? culture.sort : |tile| tile.pop_scale) -- 
  * centers.add(tile.site) 
  * culture.id = new_id
  * delete culture.odd
  * delete culture.sort
  * culture.type = define_culture_type(cell) 
  * culture.expansionism = define_culture_expansionism(culture.type) 
  * culture.origins = [0] 
  * culture.code = abbreviate(culture.name,codes) -- This seems to generate a "code" to use for the culture. Not sure if I need this.
  * codes.push(culture.code)
  * culture_ids[cell] = new_id;

* cells.culture = culture_ids -- This basically writes the culture_ids to the tiles. 
* insert "Wildlands" culture into the beginning of cultures
* name_bases = get the name base, either something provided or from default.
* for each culture:
  * culture.base = culture.base % name_bases.len -- This is assigning a name base for the culture by assigning an index into the name_bases.

* place_center(v: closure for sorting the tiles by preference): -- I get that this is finding the "center" of the culture, but I don't get how it works.
  * let spacing = (graph_width + graph_height) / 2 / count
  * const MAX_ATTEMPTS = 100
  * let sorted = [..populated].sort((a,b) => v(b) - v(a))
  * let max = (sorted.length / 2).floor()
  * let cell_id = 0;
  * for i in 0..MAX_ATTEMPTS:
    * cell_id = sorted[biased(0, max, 5)] -- TODO: What is biased?
    * spacing *= 0.9
    * if culture_ids[cell_id] == 0 and !centers.find(cells.p[cell_id][0], cells.p[cellId][1], spacing) break; -- TODO: The 'find' is from the quadtree thingie above.
  * return cell_id

* select_cultures(culture_number)
  * let def = get_default(culture_number) 
  * let cultures = [];
  * -- a whole bunch of stuff revolving around "locked" cultures when regenerating
  * let culture = 0;
  * let rnd = 0;
  * let i = 0;
  * while cultures.length < culture_number && def.length > 0:
    * loop:
      * rnd = rand(def.length - 1)
      * culture = def[rnd]
      * i += 1;
      * if i < 200 && !P(culture.odd): continue; -- TODO: What is that function P -- I think it's a random number generation thingie...
      * break -- I'm just trying to emulate the process of a do..while loop
    * cultures.push(culture)
    * def.splice(rnd,1) -- remove that from the defaults
  * return cultures

* define_culture_type(tile)
  * if tile.elevation_Scaled < 70 and biome is hot desert, cold desert or grassland: return "Nomadic" -- NOTE: This is fairly stereotyped for those environments, I don't think I should do this in the future
  * if tile.elevation_scaled > 50: return "Highland"
  * let f = -- I don't even know what this is, but I think it's checking for the feature in the "opposite" tile.
  * if f.type == "lake" and f.cells > 5: return "Lake" -- f.cells is the count of cells in a feature, so the number of cells in the lake. TODO: I don't have this data readily available.
  * if -- TODO: And now, I seem to need to know if there's a harbor here.
     * tile.harbor and f.type != "lake" && P(0.1) ||
     * tile.harbor == 1 && P(0.6) ||
     * tile is an "isle" && P(0.4):
       * return "Naval"
  * if cell is a river with a water_flow > 100: return "River"
  * if cells is land surrounded by land biome is savanna, TDF, TempRain, Taiga, Tundra, Wetland: return "Hunting"
    -- Note that the code in AFMG syas cells.t[i] > 2, but 2 appears to be the highest value. so it should never be hunting. But yet it appears.
  * return "Generic"

* define_culture_expansionism(type):
  * match type:
    * "lake" -> base = 0.8
    * "Naval" -> base = 1.5;
    * "River" -> base = 0.9;
    * "Nomadic" -> base = 1.5;
    * "Hunting" -> base = 0.7;
    * "Highland" -> base = 1.2;
    * else -> base = 1
  * return rn(((Math.random() * powerInput.value) / 2 + 1) * base, 1); // powerInput is a float from 0 to 10 that defines "how much states and cultures can vary in size"

* get_default(count):
  * cells = tiles
  * s = tile suitabilities
  * s_max = d3.max(s) -- TODO: What is this?
  * t = cells.t -- This relates to whether the tile is land, on the coast, or on a shoreline of a water body
  * temp = cells.temp -- temperature
  * n = |cell| ((s[cell]/s_max) * 3).ceil()
  * td = |cell,goal| (temp[cells.g[cell]] - goal).max(0) + 1
  * bd = |cell,biomes,fee=4| biomes.includes(cell.biome) ? 1 : fee -- biome difference fee
  * sf = |cell,fee=4| cells.haven[cell] && cell.feature !== "lake" ? 1 : fee -- fee for not on a sea coast
  * given a culture_set input get a built-in list of cultures, each has:
    * name: a name:
    * base: a number that seems to increase for each one in the set
    * odd: a float from 0 to 1
    * sort: a closure based on combinations of the closures above. -- I think this is a way of guaranteeing that the cultures are "different".

### My Algorithm: Generate Cultures

I'm going to divide this into two parts, so the user can edit the chosen cultures before placing them. But first, I need culture sets.

#### 0) Culture Sets:

At risk of avoiding copyright issues with AFMG, culture sets (which are information, not code) are not going to be stored in the application. The user will have to download or create their own. I will have my own versions on my computer, and once I have a release, perhaps I can get permission to copy AFMG, or perhaps I can create my own.

Culture sets will be read in JSON format, probably using serde. I will also need a different serde to rust notation for the sort types.

A culture set contains a list of basic cultures. Each culture has the following fields:
* name: A string representing the name of the culture
* base: an integer which links to a name base. TODO: Need to define this.
* odd: a float from 0..1 indicating how unlikely it be that this culture should be chosen from the set.
* sort: an enum which represents different "land preferences".

In addition, there are two things you can do with culture sets to generate new ones:
* Random_Rename: take a culture set and rename all the cultures according to a name_base.
* Random: randomly generate names and the other fields for a set of specified number of cultures.

The sort enum consists of the following possible values, some of which are self-referential. Basically, a function on it will return a number, which is compared between the two to find preferred tiles.

by_habitability = s = |tile| tile.habitability
by_shore_distance = t = |tile| tile.shore_distance
by_elevation = h = |tile| tile.elevation
by_normalized_habitability = n = |tile| ((tile.habitability / maximum of tiles habitability) * 3).ceil(); // normalized cell score
by_temperature_difference(goal) = td = |tile,goal| (tile.temperature - goal).abs() + 1; // temperature difference fee
by_biome(biomes,float) = bd = |tile,biomes,fee| if biomes.contains(tile.biome) { 1 } else { float }; // biome difference fee
by_biome_default(biomes) = by_biome(biomes,4)
by_sea_coast(float) = sf = |tile,fee| if tile.closest_water && closest_water.type != "lake" { 1 } else { float }; // not on sea coast fee
by_sea_coast_default = by_sea_coast(4)
negate(sort) = |tile| -sort(tile)
multiply(sore,sort) = |tile| sort(tile) * sort(tile)
divide(sort,sort) = |tile| sort(tile) / sort(tile) -- TODO: What to return if a value is 0? probably just return Infinity.
add(sort,sort) = |tile| sort(tile) + sort(tile)
pow(sort,float) = |tile| sort(tile)^float

#### 0.5) Calculate Terrain

TODO: To be done as part of the water command. This is a new command.

* create a list of "terrains".
* add all tiles that aren't ocean to a table
* while let Some(tile) = table.pop(): -- or otherwise pick a tile off the table, actually remove it
  * make a terrain list of tiles and add this tile
  * if there is a lake on the tile, then this becomes a lake feature, with the lake's ID
  * if there is no lake on the tile, then this becomes a land feature
  * let found_non_lake_neighbor = false
  * make a neighbor list of tiles
  * add the tiles' neighbors to the neighbor list (do not remove them)
  * while let Some(neighbor) = neighbors.pop():
    * if the neighbor's tile is on the table:
      * if this is a lake feature and the neighbor has a matching lake id:
        * pick the neighbor off of the table and add it to the terrain list
        * add all of it's neighbors to the neighbor list
      * if this is a land feature and the neighbor is not water:
        * pick the neighbor off of the table and add it to the terrain list
        * add all of it's neighbors to the neighbor list
    * else: found_non_lake_neighbor = true;
  * we should now have a list of all tiles of the same terrain connected
  * determine the type of terrain it is based on count and add this to the list of terrains
    * if land and found_non_lake_neighbor = false, then the land type is "lake island".
     -- NOTE: This isn't right, the tile might have been removed for this one, think about this a little.
* finally, go through the list of terrains and apply the new types to the tiles layer for each tile.


#### 1) Generate Cultures

TODO: Working on this next...

TODO: I need an is_nomadic field on biome or something like that. It's true for hot desert, cold desert, and grassland.
TODO: I need an is_hunting field on biome, it's true for savanna, TDF, TempRain, Taiga, Tundra, Wetland

* *Input*: culture_set = A culture set to use
* *Input*: culture_count = number of cultures to use
* *Input*: power_input = a float from 0 to 10 that defines "how much cultures can vary in size"
* if culture_count > culture_set.len:
  * print warning: The provided culture set is not large enough to produce the requested number of cultures. Culture count will be limited to the size of the culture set.
* let populated = tile entities where tile.habitability > 0:
* let cultures = []
* if work_queue.len < culture_count * 25:
  * culture_count = (work_queue.len() / 50).floor(); -- NOTE: It seems to me that this should be divided by 25 to match the previous condition.
  * if culture_count == 0:
    * print warning: There are not enough habitable tiles to support urban societies. No cultures, nations, or cities can be created.
  * else:
    * print warning: There are not enough habitable tiles to support the requested number of cultures. Culture count will be limited to {culture_count}.
* if culture_count == 0:
  * cultures = [wildlands culture] 
* else:
  * cultures = select_cultures(culture_set,culture_count)
* let placed_centers = []

* for culture in cultures:
  * let culture_center = find_culture_center(populated,culture,culture_count,placed_centers)
  * place_centers.push(culture_center)
  * centers.add(culture_center.site)
  * let culture_type = define_culture_type(culture_center)
  * let expansionism = define_culture_expansionism(culture_type,power_input) 
  * let name_base = select_name_base(culture,index)
  * write culture to cultures table including the original culture plus the data above

* select_cultures(culture_set,culture_count):
  * let available_cultures = culture_set.get_default(culture_set)
  * let cultures = []
  * let i = 0;
  * while (cultures.length < culture_count ) && (available_cultures.len > 0):
    * loop: -- basically, there are two randoms here: first we pick a random spot in the cultures, then we give another chance if the culture is very strange, but we only do that sort of thing if we've made less than 200 attempts so far.
      * let rnd = random(0..available_cultures.len() - 1)
      * let culture = available_cultures[rnd]
      * i += 1
      * if !((i < 200) && !(random(0..1) < culture.odd)): break; 
    * cultures.push(culture)
    * available_cultures.remove(culture)
  * return cultures

* find_culture_center(populated,culture,culture_count,placed_centers):
  * let spacing = map extent / 2 / culture_count;
  * const MAX_ATTEMPTS = 100
  * sorted = populated.sort(culture.preference)
  * let max = (populated.length / 2).floor();
  * let tile_id = None
  * for i in 0..MAX_ATTEMPTS:
    * tile_id = sorted[biased(0,max,5)]
    * spacing *= 0.9 -- reduce the spacing in case that's what the problem was.
    * if !center_placed(placed_centers,tile_id,tile.site,spacing): break;
  * return tile_id,lat,long

* center_placed(list,tile_id,tile_site,spacing): AFMG used a quadtree structure to do this, however I couldn't find any simple implementation for rust (there are plenty of implementations, but none had an API that gave me a find(x,y,radius) function). Since I don't expect a lot of cultures, I didn't see the need for a separate structure, finds should be quick in placed_centers.
  * for tile in list:
    * if id == tile: return true;
    * if tile_site.distance(tile) < spacing: return true;
  * return false

* biased(min,max,ex):
  -- generates a random number between min and max the leans towards the beginning
  * (min + ((max - min) * random(0..1).pow(ex))).round()

* define_culture_type(tile):
  * if tile.elevation_scaled < 70 and tile.biome.is_nomadic: return "Nomadic"
  * if tile.elevation_scaled > 50: return "Highland"
  * if tile.closest_water:
    * if tile.closest_water is lake and lake cell count > 5: "Lake"
    * if tile.water_count > 0 && tile.closest_water.is_ocean and (random() < 0.1) ||
      tile.water_count == 1 && (random() < 0.6) ||
      tile.terrain.isle && (random() < 0.4): return Naval 
      and bigger than an island. There are numbers for this.
  * if tile.flow > 100: return River
  * if tile.shore_Distance > 2 and tile.biome.is_huntable: return "Hunting" 
  * return "Generic"

* define_culture_expansionism(power_input,type): -- TODO: This should really be something configurable, shouldn't it?
  * let base = match type:
    * "lake" -> 0.8
    * "Naval" -> 1.5;
    * "River" -> 0.9;
    * "Nomadic" -> 1.5;
    * "Hunting" -> 0.7;
    * "Highland" -> 1.2;
    * else -> 1
  * return ((random() * powerInput) / 2 + 1) * base 

#### 2) Place Cultures:

* get list of cultures from cultures table
* index the cultures by culture_center
* for each tile:
  * if the tile matches a culture, then place that culture by assigning the culture name to the field
  * otherwise, mark the culture name as blank (we're re-writing any existing cultures)

And that's it. I know it's simple, but it allows the user to go in and edit the cultures before placing, and re-place if they want to change something.    


### Analysis: Expand Cultures

* queue = PriorityQueue -- some sort of object that includes a closure for sorting by priority, I'm thinking this is an auto-sorting vector that sorts with the higher priority towards the end so you always pop off the highest priority while processing..
* cost = []
* neutral_rate = This seems to be an input, but I can't find it. There is a neutralInput which is a number from 0 to 2, which is labelled as Growth rate and defines how many lands will remain neutral
* max_expansion_cost = tiles.len() * 0.6 * neutral_rate
* clear tile cultures
* for cultur in cultures
  * queue.add({cell_id: culture.center, culture_id: culture.i, priority: 0})
* while queue.length:
  * cell_id, culture_id, priority = queue.pop
  * type, expansinism from cultures[culture_id]
  * for each cell neighbor:
    * biome = cell.biome
    * biome_cost = get_biome_cost(culture_id,biome,type)
    * biome_change_cost = if biome == neighbor.biome ? 0 : 20;
    * height_cost = get_height_cost(neighbor,neighbor.height,type) -- TODO:
    * river_cost = get_river_cost(neighbor.water_flow,neighbor,type) -- TODO:
    * type_cost = get_type_cost(neighbor.type (cells.t), type) -- TODO:
    * cell_cost = (all of those costs) / expansionism
    * total_cost = priority + cell_cost
    * if total_Cost > max_expansion_cost: return
    * if cost[neighbor] || total_cost < cost[neighbor] 
      * if neighbor.population > 0: neighbor.culture = culture_id;
      * cost[neighbor] = total_cost
      * queue.add(neighbor_cell,culture_id, priority: total_cost)

* get_biome_cost(culture,biome,type):
  * if culture.center.biome == biome return 10 // native penalty
  * if type == "Hunting" return biomes_data.cost[biome] * 5;
  * if type == "Nomadic" and biome > 4 and biome < 10 return biomes_data.cost[biome] * 10; -- forest penalty for nomads
  * return biomes_data.cost[biome] * 2;

* get height_cost(cell,elevation,type):
  * f = cell.features
  * a = cell.area
  * if type is Lake and the tile is a lake: return 10
  * if type is Naval and tile is ocean: return a * 2;
  * if type is Nomadic and tile is ocean: return a * 50;
  * if is ocean: return a * 6;
  * if type is Highland and elevation < 44: return 3000
  * if type is highland and elevation < 62: return 200
  * if type is highland: return 0
  * if elevation >= 67: return 200
  * if elevation >= 44: return 30
  * return 0

* get_river_cost(river_id,cell_id,type):
  * if type is River return 100 if river_id is not none
  * if river_id is none return 0 -- Will need a "water_flow" means a river thing.
  * return (tile.water_flow/10).clamp(20,100)

* get_type_cost(t,type)
  * if t === 1: 
    * if Naval or Lake: 0
    * if Nomadic 60
    * else 20
  * if t == 2:
    * if Naval or Nomadic: 30
    * else 0
  * if t != -1:
    * if Naval or Lake: 100
    * else 0
  * return 0


### Analysis: Generate Burgs and States

TODO: 

### Analysis: Generate Religions

TODO: 

### Analysis: Define State Forms (Burgs and States)

TODO: 

### Analysis: Generate Provinces (Burgs and States)

TODO: 

### Analysis: Define Burg Features (Burgs and States)

TODO: 

### Analysis: Draw States?

TODO: 

### Analysis: Draw Borders?

TODO: 

### Analysis: Draw State Labels (Burgs and States)

TODO: 



# Testing Commands:

The following commands were used, in this order, to generate the testing maps of Inannak during development. `time` is not the bash command, but a GNU program you might have to install on your machine and call by path.

```sh
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- convert-heightmap ~/Cartography/Inannak/Inannak-Elevation.tif testing_output/Inannak.world.gpkg --overwrite --ocean /home/neil/Cartography/Inannak/Inannak-Ocean.tif --seed 9543572450198918714
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-climate testing_output/Inannak.world.gpkg 
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-water testing_output/Inannak.world.gpkg --overwrite
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-biome testing_output/Inannak.world.gpkg --overwrite
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-people-population testing_output/Inannak.world.gpkg

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
[ ] `gen-people` command
    [ ] various auxiliary files
    [ ] Review AFMG people generation algorithms -- again, wait on improvements until later
    [ ] Figure out how to break the task apart into sub commands and create those commands.
[ ] `curve-borders` command
    [ ] Creates new layers for several thematic layers that have less blocky borders. This is a matter of taking the shape line segments, and converting them to beziers. It makes for better visual appeal. One issue is making sure they all match up with the ocean shorelines, and that their edges line up.
    [ ] is_ocean
    [ ] biomes
    [ ] nations and provinces
    [ ] cultures
    [ ] religions
[ ] `create-terrain` commands
    [ ] terrain template files
    [ ] Review AFMG terrain generation algorithms
[ ] I need some default QGIS project with some nice styles and appearance which can be saved to the same directory. Any way to specify the filename when we create it (how difficult is it to "template" the project)? Or, do we come up with a standard file name as well?
[ ] Documentation
    [ ] Include a caveat that this is not intended to be used for scientific purposes (analyzing streams, etc.) and the algorithms are not meant to model actual physical processes.
    [ ] Include explanation of all commands
    [ ] Include explanation of the data in the output file.
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

