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

# Algorithm Analysis

## Terrain Generation

### Original AFMG code:

```js
"use strict";

window.HeightmapGenerator = (function () {
  let grid = null;
  let heights = null;
  let blobPower;
  let linePower;

  const setGraph = graph => {
    const {cellsDesired, cells, points} = graph;
    heights = cells.h ? Uint8Array.from(cells.h) : createTypedArray({maxValue: 100, length: points.length});
    blobPower = getBlobPower(cellsDesired);
    linePower = getLinePower(cellsDesired);
    grid = graph;
  };

  const getHeights = () => heights;

  const clearData = () => {
    heights = null;
    grid = null;
  };

  const fromTemplate = (graph, id) => {
    const templateString = heightmapTemplates[id]?.template || "";
    const steps = templateString.split("\n");

    if (!steps.length) throw new Error(`Heightmap template: no steps. Template: ${id}. Steps: ${steps}`);
    setGraph(graph);

    for (const step of steps) {
      const elements = step.trim().split(" ");
      if (elements.length < 2) throw new Error(`Heightmap template: steps < 2. Template: ${id}. Step: ${elements}`);
      addStep(...elements);
    }

    return heights;
  };

  const fromPrecreated = (graph, id) => {
    return new Promise(resolve => {
      // create canvas where 1px corresponts to a cell
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");
      const {cellsX, cellsY} = graph;
      canvas.width = cellsX;
      canvas.height = cellsY;

      // load heightmap into image and render to canvas
      const img = new Image();
      img.src = `./heightmaps/${id}.png`;
      img.onload = () => {
        ctx.drawImage(img, 0, 0, cellsX, cellsY);
        const imageData = ctx.getImageData(0, 0, cellsX, cellsY);
        setGraph(graph);
        getHeightsFromImageData(imageData.data);
        canvas.remove();
        img.remove();
        resolve(heights);
      };
    });
  };

  const generate = async function (graph) {
    TIME && console.time("defineHeightmap");
    const id = byId("templateInput").value;

    Math.random = aleaPRNG(seed);
    const isTemplate = id in heightmapTemplates;
    const heights = isTemplate ? fromTemplate(graph, id) : await fromPrecreated(graph, id);
    TIME && console.timeEnd("defineHeightmap");

    clearData();
    return heights;
  };

  function addStep(tool, a2, a3, a4, a5) {
    if (tool === "Hill") return addHill(a2, a3, a4, a5);
    if (tool === "Pit") return addPit(a2, a3, a4, a5);
    if (tool === "Range") return addRange(a2, a3, a4, a5);
    if (tool === "Trough") return addTrough(a2, a3, a4, a5);
    if (tool === "Strait") return addStrait(a2, a3);
    if (tool === "Mask") return mask(a2);
    if (tool === "Invert") return invert(a2, a3);
    if (tool === "Add") return modify(a3, +a2, 1);
    if (tool === "Multiply") return modify(a3, 0, +a2);
    if (tool === "Smooth") return smooth(a2);
  }

  function getBlobPower(cells) {
    const blobPowerMap = {
      1000: 0.93,
      2000: 0.95,
      5000: 0.97,
      10000: 0.98,
      20000: 0.99,
      30000: 0.991,
      40000: 0.993,
      50000: 0.994,
      60000: 0.995,
      70000: 0.9955,
      80000: 0.996,
      90000: 0.9964,
      100000: 0.9973
    };
    return blobPowerMap[cells] || 0.98;
  }

  function getLinePower() {
    const linePowerMap = {
      1000: 0.75,
      2000: 0.77,
      5000: 0.79,
      10000: 0.81,
      20000: 0.82,
      30000: 0.83,
      40000: 0.84,
      50000: 0.86,
      60000: 0.87,
      70000: 0.88,
      80000: 0.91,
      90000: 0.92,
      100000: 0.93
    };

    return linePowerMap[cells] || 0.81;
  }

  const addHill = (count, height, rangeX, rangeY) => {
    count = getNumberInRange(count);
    while (count > 0) {
      addOneHill();
      count--;
    }

    function addOneHill() {
      const change = new Uint8Array(heights.length);
      let limit = 0;
      let start;
      let h = lim(getNumberInRange(height));

      do {
        const x = getPointInRange(rangeX, graphWidth);
        const y = getPointInRange(rangeY, graphHeight);
        start = findGridCell(x, y, grid);
        limit++;
      } while (heights[start] + h > 90 && limit < 50);

      change[start] = h;
      const queue = [start];
      while (queue.length) {
        const q = queue.shift();

        for (const c of grid.cells.c[q]) {
          if (change[c]) continue;
          change[c] = change[q] ** blobPower * (Math.random() * 0.2 + 0.9);
          if (change[c] > 1) queue.push(c);
        }
      }

      heights = heights.map((h, i) => lim(h + change[i]));
    }
  };

  const addPit = (count, height, rangeX, rangeY) => {
    count = getNumberInRange(count);
    while (count > 0) {
      addOnePit();
      count--;
    }

    function addOnePit() {
      const used = new Uint8Array(heights.length);
      let limit = 0,
        start;
      let h = lim(getNumberInRange(height));

      do {
        const x = getPointInRange(rangeX, graphWidth);
        const y = getPointInRange(rangeY, graphHeight);
        start = findGridCell(x, y, grid);
        limit++;
      } while (heights[start] < 20 && limit < 50);

      const queue = [start];
      while (queue.length) {
        const q = queue.shift();
        h = h ** blobPower * (Math.random() * 0.2 + 0.9);
        if (h < 1) return;

        grid.cells.c[q].forEach(function (c, i) {
          if (used[c]) return;
          heights[c] = lim(heights[c] - h * (Math.random() * 0.2 + 0.9));
          used[c] = 1;
          queue.push(c);
        });
      }
    }
  };

  // fromCell, toCell are options cell ids
  const addRange = (count, height, rangeX, rangeY, startCell, endCell) => {
    count = getNumberInRange(count);
    while (count > 0) {
      addOneRange();
      count--;
    }

    function addOneRange() {
      const used = new Uint8Array(heights.length);
      let h = lim(getNumberInRange(height));

      if (rangeX && rangeY) {
        // find start and end points
        const startX = getPointInRange(rangeX, graphWidth);
        const startY = getPointInRange(rangeY, graphHeight);

        let dist = 0,
          limit = 0,
          endX,
          endY;

        do {
          endX = Math.random() * graphWidth * 0.8 + graphWidth * 0.1;
          endY = Math.random() * graphHeight * 0.7 + graphHeight * 0.15;
          dist = Math.abs(endY - startY) + Math.abs(endX - startX);
          limit++;
        } while ((dist < graphWidth / 8 || dist > graphWidth / 3) && limit < 50);

        startCell = findGridCell(startX, startY, grid);
        endCell = findGridCell(endX, endY, grid);
      }

      let range = getRange(startCell, endCell);

      // get main ridge
      function getRange(cur, end) {
        const range = [cur];
        const p = grid.points;
        used[cur] = 1;

        while (cur !== end) {
          let min = Infinity;
          grid.cells.c[cur].forEach(function (e) {
            if (used[e]) return;
            let diff = (p[end][0] - p[e][0]) ** 2 + (p[end][1] - p[e][1]) ** 2;
            if (Math.random() > 0.85) diff = diff / 2;
            if (diff < min) {
              min = diff;
              cur = e;
            }
          });
          if (min === Infinity) return range;
          range.push(cur);
          used[cur] = 1;
        }

        return range;
      }

      // add height to ridge and cells around
      let queue = range.slice(),
        i = 0;
      while (queue.length) {
        const frontier = queue.slice();
        (queue = []), i++;
        frontier.forEach(i => {
          heights[i] = lim(heights[i] + h * (Math.random() * 0.3 + 0.85));
        });
        h = h ** linePower - 1;
        if (h < 2) break;
        frontier.forEach(f => {
          grid.cells.c[f].forEach(i => {
            if (!used[i]) {
              queue.push(i);
              used[i] = 1;
            }
          });
        });
      }

      // generate prominences
      range.forEach((cur, d) => {
        if (d % 6 !== 0) return;
        for (const l of d3.range(i)) {
          const min = grid.cells.c[cur][d3.scan(grid.cells.c[cur], (a, b) => heights[a] - heights[b])]; // downhill cell
          heights[min] = (heights[cur] * 2 + heights[min]) / 3;
          cur = min;
        }
      });
    }
  };

  const addTrough = (count, height, rangeX, rangeY, startCell, endCell) => {
    count = getNumberInRange(count);
    while (count > 0) {
      addOneTrough();
      count--;
    }

    function addOneTrough() {
      const used = new Uint8Array(heights.length);
      let h = lim(getNumberInRange(height));

      if (rangeX && rangeY) {
        // find start and end points
        let limit = 0,
          startX,
          startY,
          dist = 0,
          endX,
          endY;
        do {
          startX = getPointInRange(rangeX, graphWidth);
          startY = getPointInRange(rangeY, graphHeight);
          startCell = findGridCell(startX, startY, grid);
          limit++;
        } while (heights[startCell] < 20 && limit < 50);

        limit = 0;
        do {
          endX = Math.random() * graphWidth * 0.8 + graphWidth * 0.1;
          endY = Math.random() * graphHeight * 0.7 + graphHeight * 0.15;
          dist = Math.abs(endY - startY) + Math.abs(endX - startX);
          limit++;
        } while ((dist < graphWidth / 8 || dist > graphWidth / 2) && limit < 50);

        endCell = findGridCell(endX, endY, grid);
      }

      let range = getRange(startCell, endCell);

      // get main ridge
      function getRange(cur, end) { // NMS: Same as in add_range
        const range = [cur];
        const p = grid.points;
        used[cur] = 1;

        while (cur !== end) {
          let min = Infinity;
          grid.cells.c[cur].forEach(function (e) {
            if (used[e]) return;
            let diff = (p[end][0] - p[e][0]) ** 2 + (p[end][1] - p[e][1]) ** 2;
            if (Math.random() > 0.8) diff = diff / 2;
            if (diff < min) {
              min = diff;
              cur = e;
            }
          });
          if (min === Infinity) return range;
          range.push(cur);
          used[cur] = 1;
        }

        return range;
      }

      // add height to ridge and cells around
      let queue = range.slice(),
        i = 0;
      while (queue.length) {
        const frontier = queue.slice();
        (queue = []), i++;
        frontier.forEach(i => {
          heights[i] = lim(heights[i] - h * (Math.random() * 0.3 + 0.85));
        });
        h = h ** linePower - 1;
        if (h < 2) break;
        frontier.forEach(f => {
          grid.cells.c[f].forEach(i => {
            if (!used[i]) {
              queue.push(i);
              used[i] = 1;
            }
          });
        });
      }

      // generate prominences
      range.forEach((cur, d) => {
        if (d % 6 !== 0) return;
        for (const l of d3.range(i)) {
          const min = grid.cells.c[cur][d3.scan(grid.cells.c[cur], (a, b) => heights[a] - heights[b])]; // downhill cell
          //debug.append("circle").attr("cx", p[min][0]).attr("cy", p[min][1]).attr("r", 1);
          heights[min] = (heights[cur] * 2 + heights[min]) / 3;
          cur = min;
        }
      });
    }
  };

  const addStrait = (width, direction = "vertical") => {
    width = Math.min(getNumberInRange(width), grid.cellsX / 3);
    if (width < 1 && P(width)) return;
    const used = new Uint8Array(heights.length);
    const vert = direction === "vertical";
    const startX = vert ? Math.floor(Math.random() * graphWidth * 0.4 + graphWidth * 0.3) : 5;
    const startY = vert ? 5 : Math.floor(Math.random() * graphHeight * 0.4 + graphHeight * 0.3);
    const endX = vert
      ? Math.floor(graphWidth - startX - graphWidth * 0.1 + Math.random() * graphWidth * 0.2)
      : graphWidth - 5;
    const endY = vert
      ? graphHeight - 5
      : Math.floor(graphHeight - startY - graphHeight * 0.1 + Math.random() * graphHeight * 0.2);

    const start = findGridCell(startX, startY, grid);
    const end = findGridCell(endX, endY, grid);
    let range = getRange(start, end);
    const query = [];

    function getRange(cur, end) { // NMS: this is *slightly different than what is used in add_range, basically, it allows you to reuse links on a path and doesn't stop if there are no neighbors... I feel like this might be an old version that didn't catch infinite loops.
      const range = [];
      const p = grid.points;

      while (cur !== end) {
        let min = Infinity;
        grid.cells.c[cur].forEach(function (e) {
          let diff = (p[end][0] - p[e][0]) ** 2 + (p[end][1] - p[e][1]) ** 2;
          if (Math.random() > 0.8) diff = diff / 2;
          if (diff < min) {
            min = diff;
            cur = e;
          }
        });
        range.push(cur);
      }

      return range;
    }

    const step = 0.1 / width;

    while (width > 0) {
      const exp = 0.9 - step * width;
      range.forEach(function (r) {
        grid.cells.c[r].forEach(function (e) {
          if (used[e]) return;
          used[e] = 1;
          query.push(e);
          heights[e] **= exp;
          if (heights[e] > 100) heights[e] = 5;
        });
      });
      range = query.slice();

      width--;
    }
  };

  const modify = (range, add, mult, power) => {
    const min = range === "land" ? 20 : range === "all" ? 0 : +range.split("-")[0];
    const max = range === "land" || range === "all" ? 100 : +range.split("-")[1];
    const isLand = min === 20;

    heights = heights.map(h => {
      if (h < min || h > max) return h;

      if (add) h = isLand ? Math.max(h + add, 20) : h + add;
      if (mult !== 1) h = isLand ? (h - 20) * mult + 20 : h * mult;
      if (power) h = isLand ? (h - 20) ** power + 20 : h ** power;
      return lim(h);
    });
  };

  const smooth = (fr = 2, add = 0) => {
    heights = heights.map((h, i) => {
      const a = [h];
      grid.cells.c[i].forEach(c => a.push(heights[c]));
      if (fr === 1) return d3.mean(a) + add;
      return lim((h * (fr - 1) + d3.mean(a) + add) / fr);
    });
  };

  const mask = (power = 1) => {
    const fr = power ? Math.abs(power) : 1;

    heights = heights.map((h, i) => {
      const [x, y] = grid.points[i];
      const nx = (2 * x) / graphWidth - 1; // [-1, 1], 0 is center
      const ny = (2 * y) / graphHeight - 1; // [-1, 1], 0 is center
      let distance = (1 - nx ** 2) * (1 - ny ** 2); // 1 is center, 0 is edge
      if (power < 0) distance = 1 - distance; // inverted, 0 is center, 1 is edge
      const masked = h * distance;
      return lim((h * (fr - 1) + masked) / fr);
    });
  };

  const invert = (count, axes) => {
    if (!P(count)) return;

    const invertX = axes !== "y";
    const invertY = axes !== "x";
    const {cellsX, cellsY} = grid;

    const inverted = heights.map((h, i) => {
      const x = i % cellsX;
      const y = Math.floor(i / cellsX);

      const nx = invertX ? cellsX - x - 1 : x;
      const ny = invertY ? cellsY - y - 1 : y;
      const invertedI = nx + ny * cellsX;
      return heights[invertedI];
    });

    heights = inverted;
  };

  function getPointInRange(range, length) {
    if (typeof range !== "string") {
      ERROR && console.error("Range should be a string");
      return;
    }

    const min = range.split("-")[0] / 100 || 0;
    const max = range.split("-")[1] / 100 || min;
    return rand(min * length, max * length);
  }

  function getHeightsFromImageData(imageData) {
    for (let i = 0; i < heights.length; i++) {
      const lightness = imageData[i * 4] / 255;
      const powered = lightness < 0.2 ? lightness : 0.2 + (lightness - 0.2) ** 0.8;
      heights[i] = minmax(Math.floor(powered * 100), 0, 100);
    }
  }

  return {
    setGraph,
    getHeights,
    generate,
    fromTemplate,
    fromPrecreated,
    addHill,
    addRange,
    addTrough,
    addStrait,
    addPit,
    smooth,
    modify,
    mask,
    invert
  };
})();
```

### Analysys

TODO: I think I'm going to have to have each task separate, in order to get it to work with clap. When clap supports chained or multiple subcommands, that can change. But I can still do multiple tasks with recipes. 

TODO: Since I don't necessarily need to skip this stuff with a heightmap, I'm rethinking the first commands. I'm going to get rid of convert-heightmap and add the following commands:
* [X] `create-from-heightmap`: takes the extent of the heightmap and generates points, then samples the elevations from that heightmap.
* [ ] `create-from-blank`: takes an extent and creates points in that extent with elevations of 0
* [ ] `create-from-random-uniform`: takes an extent and creates points with elevations randomly chosen from a given range of values. TODO: Actually, this would be better as a processing thing
* [X] `create-source-from-heightmap` -- same as above, but doesn't calc neighbors
* [ ] `create-source-from-blank`:  -- same as above, but doesn't calc neighbors
* [ ] `create-source-from-random-uniform` -- same as above, but doesn't calc neighbors
* [X] `create-calc-neighbors` -- for dividing up the neighbors after any creation.
* [X] `terrain-*` -- various tasks to run. These correspond to the tasks in the enum listed below. 
TODO: The "templates" of AFMG are converted to recipes which are to be processed.


*Terrain Command*

* *input:* task: one of HeightMapTask
-- TODO: some of these inputs and pre-variables are not needed for all tasks. There needs to be a property on each task which indicates that those things are needed, so we can ignore, say, creating the point_index if it's not needed. -- or, better yet, the task runner should have the properties available as functions and they should be called when needed. This will be a trait, and the recipe one will also implement it, but automatically generate all of them.
* *input:* lowest_elevation -- lowest elevation, usually the bottom of the sea (default = around the challenger deep)
* *input:* highest_elevation -- highest elevation found on the world (default == around the height of Everest)
* let extent = width and height (in coordinates) of map
* let tile_count = number of tiles
* tile_map
* point_index = add all points in tiles to a QuadTree, along with their tile_index
* if tasks.len() < 0: error
* let blob_power = if tile_count:
  * ..1001: 0.93,
  * 1001..2001: 0.95,
  * 2001..5001: 0.97,
  * 5001..10001: 0.98,
  * 10001..20001: 0.99,
  * 20001..30001: 0.991,
  * 30001..40001: 0.993,
  * 40001..50001: 0.994,
  * 50001..60001: 0.995,
  * 60001..70001: 0.9955,
  * 70001..80001: 0.996,
  * 80001..90001: 0.9964,
  * 90001..100001: 0.9973
  * 100001..: 0.98
* let line_power = tile_count:
  * ..1001: 0.75,
  * 1001..2001: 0.77,
  * 2001..5001: 0.79,
  * 5001..10001: 0.81,
  * 10001..20001: 0.82,
  * 20001..30001: 0.83,
  * 30001..40001: 0.84,
  * 40001..50001: 0.86,
  * 50001..60001: 0.87,
  * 60001..70001: 0.88,
  * 70001..80001: 0.91,
  * 80001..90001: 0.92,
  * 90001..100001: 0.93
  * 100001: 0.81 -- TODO: I feel like this should be higher seeing what they have for the rest.
* task.run(point_index)

* enum HeightmapTask:
  * Recipe(path)
  * AddHill(count: Range<usize>, height: Range, range_x, range_y); -- can also work to add pits if the height range is negative
  * AddRange(count: Range, height: Range, range_x, range_y); -- can also work to add a trench if the height range is negative
  * AddStrait(width: Range, direction: (Horizontal,Vertical));
  * Mask(power = 1);
  * Invert(probability, axes: X, Y, or XY); -- probability is a probability that the inversion will actually happen
  * Modify(range: Range<f64>, add, mult); -- range is a range of elevations to process. add is a number to add to the elevation (or 0), mul is a number to multiply (or 1)
  * Smooth(a2);
  * SeedOcean(seeds: usize, range_x, range_y)
  * FillOcean
  * SampleHeightmap(path)
  * SampleOceanMask(path,method: OceanSamplingMethod)
  * FloodOcean

* run_recipe(path):
  * load tasks from file at path
  * run each task as above

* add_hill(point_index, (count_range, height_range, range_x, range_y))
  * count = rng.gen_range(count_range)
  * for _ in 0..count:
    * changed = HashSet
    * let limit = 0;
    * let start;
    * let h = rng.gen_range(height_range).clamp(min_elevation,max_elevation)
    * loop:
      * let x = rng.gen_range(range_x) * extent.width
      * let y = rng.gen_range(range_y) * extent.height
      * start = point_index.find_nearest(x,y)
      * limit += 1
      * if (limit >= 50) || ((tile_map[start].elevation + h <= (max_elevation * 0.9)) && ((tile_map[start].elevation + h >= (min_elevation * 0.9)))): break
    * let queue = VecDequeue[start] -- allows popping from front
    * while let Some(tile_id) = queue.pop_front()
      * tile_map[tile_id].elevation = (tile_map[tile_id].elevation + h).clamp(min_elevation,max_elevation);
      * h = h.pow(blob_power) * (rng.gen(0..0.2) + 0.9);
      * if h < 1 || h > -1:
        * for neighbor_id in tile_map[tile_id].neighbors:
          * if changed.contains(neighbor_id): continue;
          * changed.insert(neighbor_id)
          * if change_map[neighbor_id] > 1: queue.push(neighbor_id)

* AddRange(count: Range, height: Range, range_x, range_y);
  * let count = rng.gen_range(count)
  * for _ in 0..count:
    * let used = HashMap
    * let h = rng.gen_range(height).clamp(min_elevation,max_elevation)
    * let start_x = rng.gen_range(range_x) * extent.width
    * let start_y = rng.gen_range(range_y) * extent.height
    * let dist = 0
    * let limit = 0
    * let end_x
    * let end_y
    * loop: 
      * end_x = (rng.gen_range(0..1) * extent.width * 0.8) + (extent.width * 0.1)
      * end_y = (rng.gen_range(0..1) * extent.height * 0.7) + (extent.height * 0.15)
      * dist = (end_y - start_y).abs() + (end_x - start_x).abs()
      * limit += 1;
      * if (limit >= 50) || ((dist >= extent.width / 8) || dist <= extent.width / 3) -- ?? Only want "small" or "large" ranges?
    * start_tile = point_index.find_nearest(start_x,start_y)
    * end_tile = point_index.find_nearest(end_x,end_y)
    -- build main ridge
    * let range = get_range(start_tile,end_tile)
    * let next_frontier = range.clone()
    * let i = 0;
    * while next_frontier.len() > 0:
      * let frontier = next_frontier
      * next_frontier = Vec::new()
      * i += 1
      * for each tile_id in frontier:
        * tile_map.get(tile_id).elevation = tile_map.get(tile_id).elevation + h * (rng.gen_range(0..1) * 0.3 + 0.85)
        * let tile = tile_map.get(tile_id)
        * for each neighbor_id in tile:
          * if !used_map.contains(neighbor_id):
            * next_frontier.push(neighbor_id)
            * use_map.insert(neighbor_id)
      * h = (h^line_power) - 1
      * if h < 2 && h > -2: break;
    -- generate prominences
    * for (i,tile_id) in range.enumerate:
      * let current = tile_id
      * if i % 6 !== 0: continue
      * for l of 0..i:
        * let tile = tile_map.get(current)
        * let min = None;
        * for each neighbor_id in tile:
          * let neighbor = tile_map.get(neighbor_id)
          * if min is None or neighbor.elevation < min.elevation:
            * min = Some(neighbor_id,neighbor.elevation)
        * tile_map.get(min.id).elevation = (tile.elevation * 2 + min.elevation) / 3
        * current = min.id

* get_range(start_tile,end_tile,used (map)):
  * let current_id = start_tile
  * let end = tile_map.get(end_tile)
  * let range = vec![current]
  * while current_id != end_tile:
    * let current = tile_map.get(current_id)
    * let min = None
    * for neighbor_id in current.neighbors:
      * if used.has(neighbor_id): continue;
      * let neighbor = tile_map.get(neighbor_id)
      * let diff = (end.site_x - neighbor.site_x).pow(2) + (end.site_y - neighbor.site_y).pow(2) -- distance without the squareroot
      * if rng.gen_bool(0.15): diff = diff / 2 -- occasionally let it skip a boo
      * if min is None or diff < min:
        * min = diff
        * current_id = neighbor_id
    * if min is None: break -- there were no neighbors, or at least ones that haven't been used, so just stop here
    * range.push(current_id)
    * used.insert(current_id)
  * return range


* AddStrait(width: Range, direction: (Horizontal,Vertical));
  * let width: rng.gen_range(width).min(horizontal tile count / 3) -- TODO: But wouldn't you want to count by vertical tiles if it's a horizontal direction?
  * if width < 1 && rng.gen_bool(width): return -- TODO: I'm not sure what the point of this is
  * let used = HashMap
  * let is_vert = direction == Vertical
  * let start_x = if is_vertical:
    * (rng.gen_range(0..1) * extent.width * 0.4 + extent.width * 0.3).floor()
    * else: extent.west + 5
  * let start_y = if is_vertical: extent.south + 5
    * else: (rng.gen_range(0..1) * extent.height * 0.4 + extent.height * 0.3).floor()
  * let end_x = if is_vertical:
    * extent.width - start_x - extent.width * 0.1 + rng.gen_Range * extent.width * 0.2
    * else: extent.west + extent.width - 5
  * let start_x = if is_vertical:
    * extent.south + extent.height - 5
    * else: extent.height - start_x - extent.height * 0.1 + rng.gen_Range * extent.height * 0.2
  * start_tile = point_index.find_nearest(start_x,start_y)
  * end_tile = point_index.find_nearest(end_x,end_y)
  * let range = get_range(start,end,used)
  * let query = Vec::new()
  * let step = 0.1 / width
  * while width > 0:
    * let exp = 0.9 - step * width;
    * for tile_id in range:
      * for neighbor_id in tile.neighbors:
        * if used.contains(neighbor_id): continue
        * used.insert(neighbor_id);
        * query.push(neighbor_id)
        * tile_map[neighbor_id].elevation = (tile_map[neighbor_id].elevation).pow(exp)
        * if tile_map[neighbor_id].elevation > max_elevation: tile_map[neighbor_id].elevation = min_elevation * 0.75 -- NMS: Why?
    * range = query
    * query = Vec::new()
    * width = width -= 1

* Mask(power = 1): -- power doesn't need to be possitive, this seems to mask things based on distance from edge of map. Not sure the use of this.
  * let fr = power.abs()
  * for tile in tile_map:
    * let Point(x,y) = tile.site
    * let nx = (2*x)/extents.width - 1 
    * let ny = (2*y)/extents.height - 1
    * let distance = (1 - nx^2) * (1 - ny^2) -- basically distance from edge, scaled so that 1 is center of map
    * if power < 0: distance = 1 - distance -- inverted from that if it's negative
    * masked = tile.elevation * distance
    * tile.elevation = ((h * (fr - 1) + masked)/fr).clamp(min_elevation,max_elevation)

* Invert(probability, axes: X, Y, or XY):
  -- The AFMG code is able to map tiles by index in a grid. I can't do that, so this is going to be much different
  * if !rng.gen_bool(probability): return 
  * let invert_x = axes !== Y
  * let invert_y = axes !== X
  * changes = HashMap
  * for tile in tile_map.enumerate:
    * let nx = if invert_x { extent.east - ( x - extent.west) } else { x }
    * let ny = if invert_y { extent.north - (y - extent.south) } else { y }
    * let opposite_tile = point_finder.find_nearest(nx,ny)
    * changes.insert(tile,opposite_tile.elevation)
  * for (tile,elevation) in changes:
    * tile_map[tile].elevation = elevation

* Modify(range: Range<f64>, add, mult): -- NOTE: The AFMG has an option to specify "all" and "land" for range. This is replaced with using min_elevation..max_elevation or 0..max_elevation
-- NOTE 2: Except that AFMG would also use the "Land" range to indicate that items shouldn't clamp below land. I disagree that this is a useful feature.
  * for tile in tile_map:
    * let elevation = tile.elevation
    * if elevation < range.start || tile.elevation > range.end: continue
    * if add != 0: elevation += add
    * if mul != 1: elevation *= mul
    * tile.elevation = elevation.clamp(min_elevation,max_elevation)

* Smooth(fr = 2):
  * changes = map
  * for tile in tile_map:
    * let elevations = vec![tile.elevation]
    * for neighbor in neighbors:
      * elevation.push(neighbor.elevation)
    * let new_elevation = 
      * if fr == 1: elevations.average()
      * else: ((tile.elevation * (fr - 1) + elevations.average) / fr).clamp(min_elevation,max_elevation)

* SeedOcean(attempts: usize, range_x, range_y) -- seeds an ocean in a general area. Places random drops on the land, and follows their path until they find a suitable tile below 0 within the range. Must use flood ocean to complete the process
  * let seeds = Vec::new()
  * for _ in 0..attempts:
    * let start;
    * let x = rng.gen_range(range_x) * extent.width
    * let y = rng.gen_range(range_y) * extent.height
    * let start = point_index.find_nearest(x,y)
    * let queue = vec![start]
    * while let tile_id = queue.pop:
      * let tile = tile_map.get(tile_id);
      * if tile.elevation < 0:
        * seeds.push(tile_id)
        * break
      * for neighbor_id in tile.neighbors:
        * let neighbor = tile_map.get(neighbor_id);
        * if neighbor.site outside of range_x and range_y: continue
        -- follow all of the neighbors down hill.
        * if neighbor.elevation < tile.elevation:
          * queue.push(neighbor_id)
  * for seed in seeds:
    * tile_map.get[seed].is_ocean = true;

* SampleHeightmap(path)

-- This is just what we're doing already, but the extent of the map doesn't necessarily have to match the extent of the created world. 

* SampleOceanMask(path,method: OceanSamplingMethod)

-- This is just what we're doing already

* FloodOcean
  * checked = hash_map
  * queue = list
  * for tile in tile_map:
    * if tile.is_ocean: 
      * queue.push(tile_id)
      * checked.insert(tile_id,false)
  * while let tile_id = queue.pop:
    * let tile = tile_map.get(tile_id)
    * for neighbor_id in neighbors:
      * if checked.has(neighbor_id): continue
      * let neighbor = tile_map.get(neighbor_id)
      * let new_ocean = neighbor.elevation < 0 && !neighbor.is_ocean
      * checked.insert(neighbor_id,new_ocean)
      * if neighbor.elevation < 0: queue.push(neighbor_id) -- queue whether we're changing the ocean or not, so the algorithm spreads throughout the area.
  * for (tile_id,new_ocean) in checked:
    * if new_ocean: tiles.is_ocean = true

* FillOcean -- marks all elevations less than 0 as ocean. It's a rather brute-force method and removes anything like a death valley.
  * for tile in tile_map:
    * if tile.elevation < 0: tile.is_ocean = true





# Testing Commands:

The following commands were used, in this order, to generate the testing maps of Inannak during development. `time` is not the bash command, but a GNU program you might have to install on your machine and call by path.

```sh
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- create-from-heightmap ~/Cartography/Inannak/Inannak-Elevation.tif testing_output/Inannak.world.gpkg --overwrite --seed 9543572450198918714
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- terrain testing_output/Inannak.world.gpkg --min-elevation -3536 --max-elevation 10011 sample-ocean-masked /home/neil/Cartography/Inannak/Inannak-Ocean.tif
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-climate testing_output/Inannak.world.gpkg 
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-water testing_output/Inannak.world.gpkg --overwrite
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-biome testing_output/Inannak.world.gpkg --overwrite
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-people testing_output/Inannak.world.gpkg --cultures testing_output/afmg_culture_antique.json --overwrite --namers testing_output/afmg_namers.json --seed 11418135282022031501
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-towns testing_output/Inannak.world.gpkg --overwrite --namers testing_output/afmg_namers.json --default-namer English --no-builtin-namers --seed 11418135282022031501
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-nations testing_output/Inannak.world.gpkg --overwrite --namers testing_output/afmg_namers.json --default-namer English --no-builtin-namers --seed 11418135282022031501
/usr/bin/time -f 'Time:\t\t%E\nMax Mem:\t%M\nCPU:\t\t%P\nFile Out:\t%O' cargo run -- gen-subnations testing_output/Inannak.world.gpkg --overwrite --namers testing_output/afmg_namers.json --default-namer English --no-builtin-namers --seed 11418135282022031501

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
[ ] `create-terrain` commands
    [ ] terrain template files
    [ ] Review AFMG terrain generation algorithms
[ ] Speed up the shore_distance algorithm by using the cost-expand process as with cultures, states, etc.
[ ] I need some default QGIS project with some nice styles and appearance which can be saved to the same directory. Any way to specify the filename when we create it (how difficult is it to "template" the project)? Or, do we come up with a standard file name as well? TODO: I may need random colors for things like nations and the like, which I can't just set graduated symbology on.
[ ] Hide the sub-commands, but document them with an appropriate option flag on help. -- I wonder if this might work if I can do nested subcommands, but with defaults? Then maybe I could only display them when they do help on the other command.
[ ] Clean up and triage TODOs into things I must do now, simple things I could do now, and things that can wait until sometime later.
[ ] Documentation
    [ ] Include a caveat that this is not intended to be used for scientific purposes (analyzing streams, etc.) and the algorithms are not meant to model actual physical processes.
    [ ] Include a note that unlike it's predecessors, there are certain things I won't touch, like Coats of Arms, random zones and markers. There has to be a point where your imagination gets to take over, otherwise there is no real purpose for this tool.
    [ ] Make sure it's clear that, although the algorithms were inspired by AFMG, the tool is not guaranteed to, and indeed not designed to, behave exactly the same in regards to output given the same output parameters.
    [ ] Include explanation of all commands
    [ ] Include explanation of the data (layers and fields) in the output file.
[ ] Figure out how to compile and deploy this tool to various operating systems. At least arch linux and windows.
[ ] Announce beta release on Blog, Mammoth, Reddit (AFMG list, imaginarymapping, a few other places), and start updating those places when changes are made.
    -- I feel like having all the above is enough to announce, as long as "creating terrain", a large task, will be the next thing on the list.
[ ] Some additions to `gen-civil`, or perhaps another command:
    [ ] `gen-civil-roads`
    [ ] `gen-civil-trails`
    [ ] `gen-civil-ocean-routes`
    [ ] Update `gen-civil-town-details` so that the population of towns are effected by connection to roads
[ ] Improved, Similar-area voronoization algorithm vaguely described above
[ ] Improved climate generation commands
[ ] Improved people and culture generation commands
[ ] `gen-features` command?
    [ ] Various auxiliar files
    [ ] Review AFMG markers and zones algorithm
[ ] `regen-*` commands
    [ ] Based on what is done in `gen-people` and some other things, but keep things that shouldn't be regenerated. -- Do I want to allow them to "lock" things? This almost has to be the same algorithms that I'm using. In which case, do I really need this? The only way this would be useful is if I could lock, because otherwise you could just continue.
[ ] `dissolve` commands
[ ] `genesis` command and `genesis-heightmap` Which does everything.
[ ] Also a `regenesis` command that will let you start at a specific stage in the process and regenerate everything from that, but keep the previous stuff. This is different from just the sub-tasks, as it will finish all tasks after that.
[ ] Start working on QGIS scripts and tools and a plugin for installing them -- maybe, if there's call for it.
[ ] `convert-afmg` command -- for now, just convert CSV and GeoJSON exports. Don't worry and probably don't plan to support the ".map" file.
[ ] `submap` command
[ ] `convert-image` command if I can't just use convert-heightmap
[ ] `import-biomes` command

