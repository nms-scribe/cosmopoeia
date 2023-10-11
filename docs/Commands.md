# Command-Line Help for `cosmopoeia`

This document contains the help content for the `cosmopoeia` command-line program.

**Command Overview:**

* [`cosmopoeia`↴](#cosmopoeia)
* [`cosmopoeia create`↴](#cosmopoeia-create)
* [`cosmopoeia create from-heightmap`↴](#cosmopoeia-create-from-heightmap)
* [`cosmopoeia create from-heightmap recipe`↴](#cosmopoeia-create-from-heightmap-recipe)
* [`cosmopoeia create from-heightmap recipe-set`↴](#cosmopoeia-create-from-heightmap-recipe-set)
* [`cosmopoeia create from-heightmap clear`↴](#cosmopoeia-create-from-heightmap-clear)
* [`cosmopoeia create from-heightmap clear-ocean`↴](#cosmopoeia-create-from-heightmap-clear-ocean)
* [`cosmopoeia create from-heightmap random-uniform`↴](#cosmopoeia-create-from-heightmap-random-uniform)
* [`cosmopoeia create from-heightmap add-hill`↴](#cosmopoeia-create-from-heightmap-add-hill)
* [`cosmopoeia create from-heightmap add-range`↴](#cosmopoeia-create-from-heightmap-add-range)
* [`cosmopoeia create from-heightmap add-strait`↴](#cosmopoeia-create-from-heightmap-add-strait)
* [`cosmopoeia create from-heightmap mask`↴](#cosmopoeia-create-from-heightmap-mask)
* [`cosmopoeia create from-heightmap invert`↴](#cosmopoeia-create-from-heightmap-invert)
* [`cosmopoeia create from-heightmap add`↴](#cosmopoeia-create-from-heightmap-add)
* [`cosmopoeia create from-heightmap multiply`↴](#cosmopoeia-create-from-heightmap-multiply)
* [`cosmopoeia create from-heightmap smooth`↴](#cosmopoeia-create-from-heightmap-smooth)
* [`cosmopoeia create from-heightmap erode`↴](#cosmopoeia-create-from-heightmap-erode)
* [`cosmopoeia create from-heightmap seed-ocean`↴](#cosmopoeia-create-from-heightmap-seed-ocean)
* [`cosmopoeia create from-heightmap fill-ocean`↴](#cosmopoeia-create-from-heightmap-fill-ocean)
* [`cosmopoeia create from-heightmap flood-ocean`↴](#cosmopoeia-create-from-heightmap-flood-ocean)
* [`cosmopoeia create from-heightmap sample-ocean-masked`↴](#cosmopoeia-create-from-heightmap-sample-ocean-masked)
* [`cosmopoeia create from-heightmap sample-ocean-below`↴](#cosmopoeia-create-from-heightmap-sample-ocean-below)
* [`cosmopoeia create from-heightmap sample-elevation`↴](#cosmopoeia-create-from-heightmap-sample-elevation)
* [`cosmopoeia create blank`↴](#cosmopoeia-create-blank)
* [`cosmopoeia create blank recipe`↴](#cosmopoeia-create-blank-recipe)
* [`cosmopoeia create blank recipe-set`↴](#cosmopoeia-create-blank-recipe-set)
* [`cosmopoeia create blank clear`↴](#cosmopoeia-create-blank-clear)
* [`cosmopoeia create blank clear-ocean`↴](#cosmopoeia-create-blank-clear-ocean)
* [`cosmopoeia create blank random-uniform`↴](#cosmopoeia-create-blank-random-uniform)
* [`cosmopoeia create blank add-hill`↴](#cosmopoeia-create-blank-add-hill)
* [`cosmopoeia create blank add-range`↴](#cosmopoeia-create-blank-add-range)
* [`cosmopoeia create blank add-strait`↴](#cosmopoeia-create-blank-add-strait)
* [`cosmopoeia create blank mask`↴](#cosmopoeia-create-blank-mask)
* [`cosmopoeia create blank invert`↴](#cosmopoeia-create-blank-invert)
* [`cosmopoeia create blank add`↴](#cosmopoeia-create-blank-add)
* [`cosmopoeia create blank multiply`↴](#cosmopoeia-create-blank-multiply)
* [`cosmopoeia create blank smooth`↴](#cosmopoeia-create-blank-smooth)
* [`cosmopoeia create blank erode`↴](#cosmopoeia-create-blank-erode)
* [`cosmopoeia create blank seed-ocean`↴](#cosmopoeia-create-blank-seed-ocean)
* [`cosmopoeia create blank fill-ocean`↴](#cosmopoeia-create-blank-fill-ocean)
* [`cosmopoeia create blank flood-ocean`↴](#cosmopoeia-create-blank-flood-ocean)
* [`cosmopoeia create blank sample-ocean-masked`↴](#cosmopoeia-create-blank-sample-ocean-masked)
* [`cosmopoeia create blank sample-ocean-below`↴](#cosmopoeia-create-blank-sample-ocean-below)
* [`cosmopoeia create blank sample-elevation`↴](#cosmopoeia-create-blank-sample-elevation)
* [`cosmopoeia terrain`↴](#cosmopoeia-terrain)
* [`cosmopoeia terrain recipe`↴](#cosmopoeia-terrain-recipe)
* [`cosmopoeia terrain recipe-set`↴](#cosmopoeia-terrain-recipe-set)
* [`cosmopoeia terrain clear`↴](#cosmopoeia-terrain-clear)
* [`cosmopoeia terrain clear-ocean`↴](#cosmopoeia-terrain-clear-ocean)
* [`cosmopoeia terrain random-uniform`↴](#cosmopoeia-terrain-random-uniform)
* [`cosmopoeia terrain add-hill`↴](#cosmopoeia-terrain-add-hill)
* [`cosmopoeia terrain add-range`↴](#cosmopoeia-terrain-add-range)
* [`cosmopoeia terrain add-strait`↴](#cosmopoeia-terrain-add-strait)
* [`cosmopoeia terrain mask`↴](#cosmopoeia-terrain-mask)
* [`cosmopoeia terrain invert`↴](#cosmopoeia-terrain-invert)
* [`cosmopoeia terrain add`↴](#cosmopoeia-terrain-add)
* [`cosmopoeia terrain multiply`↴](#cosmopoeia-terrain-multiply)
* [`cosmopoeia terrain smooth`↴](#cosmopoeia-terrain-smooth)
* [`cosmopoeia terrain erode`↴](#cosmopoeia-terrain-erode)
* [`cosmopoeia terrain seed-ocean`↴](#cosmopoeia-terrain-seed-ocean)
* [`cosmopoeia terrain fill-ocean`↴](#cosmopoeia-terrain-fill-ocean)
* [`cosmopoeia terrain flood-ocean`↴](#cosmopoeia-terrain-flood-ocean)
* [`cosmopoeia terrain sample-ocean-masked`↴](#cosmopoeia-terrain-sample-ocean-masked)
* [`cosmopoeia terrain sample-ocean-below`↴](#cosmopoeia-terrain-sample-ocean-below)
* [`cosmopoeia terrain sample-elevation`↴](#cosmopoeia-terrain-sample-elevation)
* [`cosmopoeia gen-climate`↴](#cosmopoeia-gen-climate)
* [`cosmopoeia gen-water`↴](#cosmopoeia-gen-water)
* [`cosmopoeia gen-biome`↴](#cosmopoeia-gen-biome)
* [`cosmopoeia gen-people`↴](#cosmopoeia-gen-people)
* [`cosmopoeia gen-towns`↴](#cosmopoeia-gen-towns)
* [`cosmopoeia gen-nations`↴](#cosmopoeia-gen-nations)
* [`cosmopoeia gen-subnations`↴](#cosmopoeia-gen-subnations)
* [`cosmopoeia big-bang`↴](#cosmopoeia-big-bang)
* [`cosmopoeia big-bang from-heightmap`↴](#cosmopoeia-big-bang-from-heightmap)
* [`cosmopoeia big-bang from-heightmap recipe`↴](#cosmopoeia-big-bang-from-heightmap-recipe)
* [`cosmopoeia big-bang from-heightmap recipe-set`↴](#cosmopoeia-big-bang-from-heightmap-recipe-set)
* [`cosmopoeia big-bang from-heightmap clear`↴](#cosmopoeia-big-bang-from-heightmap-clear)
* [`cosmopoeia big-bang from-heightmap clear-ocean`↴](#cosmopoeia-big-bang-from-heightmap-clear-ocean)
* [`cosmopoeia big-bang from-heightmap random-uniform`↴](#cosmopoeia-big-bang-from-heightmap-random-uniform)
* [`cosmopoeia big-bang from-heightmap add-hill`↴](#cosmopoeia-big-bang-from-heightmap-add-hill)
* [`cosmopoeia big-bang from-heightmap add-range`↴](#cosmopoeia-big-bang-from-heightmap-add-range)
* [`cosmopoeia big-bang from-heightmap add-strait`↴](#cosmopoeia-big-bang-from-heightmap-add-strait)
* [`cosmopoeia big-bang from-heightmap mask`↴](#cosmopoeia-big-bang-from-heightmap-mask)
* [`cosmopoeia big-bang from-heightmap invert`↴](#cosmopoeia-big-bang-from-heightmap-invert)
* [`cosmopoeia big-bang from-heightmap add`↴](#cosmopoeia-big-bang-from-heightmap-add)
* [`cosmopoeia big-bang from-heightmap multiply`↴](#cosmopoeia-big-bang-from-heightmap-multiply)
* [`cosmopoeia big-bang from-heightmap smooth`↴](#cosmopoeia-big-bang-from-heightmap-smooth)
* [`cosmopoeia big-bang from-heightmap erode`↴](#cosmopoeia-big-bang-from-heightmap-erode)
* [`cosmopoeia big-bang from-heightmap seed-ocean`↴](#cosmopoeia-big-bang-from-heightmap-seed-ocean)
* [`cosmopoeia big-bang from-heightmap fill-ocean`↴](#cosmopoeia-big-bang-from-heightmap-fill-ocean)
* [`cosmopoeia big-bang from-heightmap flood-ocean`↴](#cosmopoeia-big-bang-from-heightmap-flood-ocean)
* [`cosmopoeia big-bang from-heightmap sample-ocean-masked`↴](#cosmopoeia-big-bang-from-heightmap-sample-ocean-masked)
* [`cosmopoeia big-bang from-heightmap sample-ocean-below`↴](#cosmopoeia-big-bang-from-heightmap-sample-ocean-below)
* [`cosmopoeia big-bang from-heightmap sample-elevation`↴](#cosmopoeia-big-bang-from-heightmap-sample-elevation)
* [`cosmopoeia big-bang blank`↴](#cosmopoeia-big-bang-blank)
* [`cosmopoeia big-bang blank recipe`↴](#cosmopoeia-big-bang-blank-recipe)
* [`cosmopoeia big-bang blank recipe-set`↴](#cosmopoeia-big-bang-blank-recipe-set)
* [`cosmopoeia big-bang blank clear`↴](#cosmopoeia-big-bang-blank-clear)
* [`cosmopoeia big-bang blank clear-ocean`↴](#cosmopoeia-big-bang-blank-clear-ocean)
* [`cosmopoeia big-bang blank random-uniform`↴](#cosmopoeia-big-bang-blank-random-uniform)
* [`cosmopoeia big-bang blank add-hill`↴](#cosmopoeia-big-bang-blank-add-hill)
* [`cosmopoeia big-bang blank add-range`↴](#cosmopoeia-big-bang-blank-add-range)
* [`cosmopoeia big-bang blank add-strait`↴](#cosmopoeia-big-bang-blank-add-strait)
* [`cosmopoeia big-bang blank mask`↴](#cosmopoeia-big-bang-blank-mask)
* [`cosmopoeia big-bang blank invert`↴](#cosmopoeia-big-bang-blank-invert)
* [`cosmopoeia big-bang blank add`↴](#cosmopoeia-big-bang-blank-add)
* [`cosmopoeia big-bang blank multiply`↴](#cosmopoeia-big-bang-blank-multiply)
* [`cosmopoeia big-bang blank smooth`↴](#cosmopoeia-big-bang-blank-smooth)
* [`cosmopoeia big-bang blank erode`↴](#cosmopoeia-big-bang-blank-erode)
* [`cosmopoeia big-bang blank seed-ocean`↴](#cosmopoeia-big-bang-blank-seed-ocean)
* [`cosmopoeia big-bang blank fill-ocean`↴](#cosmopoeia-big-bang-blank-fill-ocean)
* [`cosmopoeia big-bang blank flood-ocean`↴](#cosmopoeia-big-bang-blank-flood-ocean)
* [`cosmopoeia big-bang blank sample-ocean-masked`↴](#cosmopoeia-big-bang-blank-sample-ocean-masked)
* [`cosmopoeia big-bang blank sample-ocean-below`↴](#cosmopoeia-big-bang-blank-sample-ocean-below)
* [`cosmopoeia big-bang blank sample-elevation`↴](#cosmopoeia-big-bang-blank-sample-elevation)

## `cosmopoeia`

N M Sheldon's Fantasy Mapping Tools

**Usage:** `cosmopoeia <COMMAND>`

###### **Subcommands:**

* `create` — Creates a world map
* `terrain` — Runs a terrain process on the world to manipulate elevations or ocean status
* `gen-climate` — Generates climate data for a world
* `gen-water` — Generates water features for a world
* `gen-biome` — Generates biomes for a world
* `gen-people` — Generates populations and cultures for a world
* `gen-towns` — Generates towns, cities and other urban centers for a world
* `gen-nations` — Generates nations for a world
* `gen-subnations` — Generates subnations (provinces and other administrative divisions) for a world
* `big-bang` — Creates a world map, generates natural features, and populates it with nations and subnations



## `cosmopoeia create`

Creates a world map

**Usage:** `cosmopoeia create [OPTIONS] <TARGET> <SOURCE>`

###### **Subcommands:**

* `from-heightmap` — Creates voronoi tiles in the same extent as a heightmap with zero elevation
* `blank` — Creates voronoi tiles in the given extent with zero elevation

###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--tile-count <TILE_COUNT>` — The rough number of tiles to generate for the image

  Default value: `10000`
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--overwrite-tiles` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia create from-heightmap`

Creates voronoi tiles in the same extent as a heightmap with zero elevation

**Usage:** `cosmopoeia create from-heightmap <SOURCE> [COMMAND]`

###### **Subcommands:**

* `recipe` — Processes a series of pre-saved tasks
* `recipe-set` — Randomly chooses a recipe from a set of named recipes and follows it
* `clear` — Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0
* `clear-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `random-uniform` — Adds a uniform amount of random noise to the map
* `add-hill` — Adds hills or pits to a certain area of the map
* `add-range` — Adds a range of heights or a trough to a certain area of a map
* `add-strait` — Adds a long cut somewhere on the map
* `mask` — Changes the heights based on their distance from the edge of the map
* `invert` — Inverts the heights across the entire map
* `add` — Inverts the heights across the entier map
* `multiply` — Inverts the heights across the entier map
* `smooth` — Smooths elevations by averaging the value against it's neighbors
* `erode` — Runs an erosion process on the map
* `seed-ocean` — Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
* `fill-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `flood-ocean` — Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found
* `sample-ocean-masked` — Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean
* `sample-ocean-below` — Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean
* `sample-elevation` — Replaces elevations by sampling from a heightmap

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia create from-heightmap recipe`

Processes a series of pre-saved tasks

**Usage:** `cosmopoeia create from-heightmap recipe --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON File describing the tasks to complete



## `cosmopoeia create from-heightmap recipe-set`

Randomly chooses a recipe from a set of named recipes and follows it

**Usage:** `cosmopoeia create from-heightmap recipe-set [OPTIONS] --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON file containing a map of potential recipes to follow
* `--recipe <RECIPE>`



## `cosmopoeia create from-heightmap clear`

Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0

**Usage:** `cosmopoeia create from-heightmap clear`



## `cosmopoeia create from-heightmap clear-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia create from-heightmap clear-ocean`



## `cosmopoeia create from-heightmap random-uniform`

Adds a uniform amount of random noise to the map

**Usage:** `cosmopoeia create from-heightmap random-uniform [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia create from-heightmap add-hill`

Adds hills or pits to a certain area of the map

**Usage:** `cosmopoeia create from-heightmap add-hill --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create from-heightmap add-range`

Adds a range of heights or a trough to a certain area of a map

**Usage:** `cosmopoeia create from-heightmap add-range --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create from-heightmap add-strait`

Adds a long cut somewhere on the map

**Usage:** `cosmopoeia create from-heightmap add-strait --width <WIDTH> --direction <DIRECTION>`

###### **Options:**

* `--width <WIDTH>`
* `--direction <DIRECTION>`

  Possible values: `horizontal`, `vertical`




## `cosmopoeia create from-heightmap mask`

Changes the heights based on their distance from the edge of the map

**Usage:** `cosmopoeia create from-heightmap mask [OPTIONS]`

###### **Options:**

* `--power <POWER>`

  Default value: `1`



## `cosmopoeia create from-heightmap invert`

Inverts the heights across the entire map

**Usage:** `cosmopoeia create from-heightmap invert --probability <PROBABILITY> --axes <AXES>`

###### **Options:**

* `--probability <PROBABILITY>`
* `--axes <AXES>`

  Possible values: `x`, `y`, `both`




## `cosmopoeia create from-heightmap add`

Inverts the heights across the entier map

**Usage:** `cosmopoeia create from-heightmap add [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia create from-heightmap multiply`

Inverts the heights across the entier map

**Usage:** `cosmopoeia create from-heightmap multiply [OPTIONS] --height-factor <HEIGHT_FACTOR>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-factor <HEIGHT_FACTOR>`



## `cosmopoeia create from-heightmap smooth`

Smooths elevations by averaging the value against it's neighbors

**Usage:** `cosmopoeia create from-heightmap smooth [OPTIONS]`

###### **Options:**

* `--fr <FR>`

  Default value: `2`



## `cosmopoeia create from-heightmap erode`

Runs an erosion process on the map

**Usage:** `cosmopoeia create from-heightmap erode [OPTIONS]`

###### **Options:**

* `--weathering-amount <WEATHERING_AMOUNT>` — Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)

  Default value: `1000`
* `--iterations <ITERATIONS>`

  Default value: `10`



## `cosmopoeia create from-heightmap seed-ocean`

Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)

**Usage:** `cosmopoeia create from-heightmap seed-ocean --count <COUNT> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create from-heightmap fill-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia create from-heightmap fill-ocean`



## `cosmopoeia create from-heightmap flood-ocean`

Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found

**Usage:** `cosmopoeia create from-heightmap flood-ocean`



## `cosmopoeia create from-heightmap sample-ocean-masked`

Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean

**Usage:** `cosmopoeia create from-heightmap sample-ocean-masked <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data



## `cosmopoeia create from-heightmap sample-ocean-below`

Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean

**Usage:** `cosmopoeia create from-heightmap sample-ocean-below --elevation <ELEVATION> <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data

###### **Options:**

* `--elevation <ELEVATION>` — The elevation to compare to



## `cosmopoeia create from-heightmap sample-elevation`

Replaces elevations by sampling from a heightmap

**Usage:** `cosmopoeia create from-heightmap sample-elevation <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia create blank`

Creates voronoi tiles in the given extent with zero elevation

**Usage:** `cosmopoeia create blank [OPTIONS] <HEIGHT> <WIDTH> <SOUTH> <WEST> [COMMAND]`

###### **Subcommands:**

* `recipe` — Processes a series of pre-saved tasks
* `recipe-set` — Randomly chooses a recipe from a set of named recipes and follows it
* `clear` — Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0
* `clear-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `random-uniform` — Adds a uniform amount of random noise to the map
* `add-hill` — Adds hills or pits to a certain area of the map
* `add-range` — Adds a range of heights or a trough to a certain area of a map
* `add-strait` — Adds a long cut somewhere on the map
* `mask` — Changes the heights based on their distance from the edge of the map
* `invert` — Inverts the heights across the entire map
* `add` — Inverts the heights across the entier map
* `multiply` — Inverts the heights across the entier map
* `smooth` — Smooths elevations by averaging the value against it's neighbors
* `erode` — Runs an erosion process on the map
* `seed-ocean` — Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
* `fill-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `flood-ocean` — Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found
* `sample-ocean-masked` — Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean
* `sample-ocean-below` — Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean
* `sample-elevation` — Replaces elevations by sampling from a heightmap

###### **Arguments:**

* `<HEIGHT>` — the height (from north to south) in degrees of the world extents
* `<WIDTH>` — the width in degrees of the world extents
* `<SOUTH>` — the latitude of the southern border of the world extents
* `<WEST>` — the longitude of the western border of the world extents

###### **Options:**

* `--min-elevation <MIN_ELEVATION>` — minimum elevation for heightmap

  Default value: `-11000`
* `--max-elevation <MAX_ELEVATION>` — maximum elevation for heightmap

  Default value: `9000`



## `cosmopoeia create blank recipe`

Processes a series of pre-saved tasks

**Usage:** `cosmopoeia create blank recipe --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON File describing the tasks to complete



## `cosmopoeia create blank recipe-set`

Randomly chooses a recipe from a set of named recipes and follows it

**Usage:** `cosmopoeia create blank recipe-set [OPTIONS] --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON file containing a map of potential recipes to follow
* `--recipe <RECIPE>`



## `cosmopoeia create blank clear`

Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0

**Usage:** `cosmopoeia create blank clear`



## `cosmopoeia create blank clear-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia create blank clear-ocean`



## `cosmopoeia create blank random-uniform`

Adds a uniform amount of random noise to the map

**Usage:** `cosmopoeia create blank random-uniform [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia create blank add-hill`

Adds hills or pits to a certain area of the map

**Usage:** `cosmopoeia create blank add-hill --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create blank add-range`

Adds a range of heights or a trough to a certain area of a map

**Usage:** `cosmopoeia create blank add-range --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create blank add-strait`

Adds a long cut somewhere on the map

**Usage:** `cosmopoeia create blank add-strait --width <WIDTH> --direction <DIRECTION>`

###### **Options:**

* `--width <WIDTH>`
* `--direction <DIRECTION>`

  Possible values: `horizontal`, `vertical`




## `cosmopoeia create blank mask`

Changes the heights based on their distance from the edge of the map

**Usage:** `cosmopoeia create blank mask [OPTIONS]`

###### **Options:**

* `--power <POWER>`

  Default value: `1`



## `cosmopoeia create blank invert`

Inverts the heights across the entire map

**Usage:** `cosmopoeia create blank invert --probability <PROBABILITY> --axes <AXES>`

###### **Options:**

* `--probability <PROBABILITY>`
* `--axes <AXES>`

  Possible values: `x`, `y`, `both`




## `cosmopoeia create blank add`

Inverts the heights across the entier map

**Usage:** `cosmopoeia create blank add [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia create blank multiply`

Inverts the heights across the entier map

**Usage:** `cosmopoeia create blank multiply [OPTIONS] --height-factor <HEIGHT_FACTOR>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-factor <HEIGHT_FACTOR>`



## `cosmopoeia create blank smooth`

Smooths elevations by averaging the value against it's neighbors

**Usage:** `cosmopoeia create blank smooth [OPTIONS]`

###### **Options:**

* `--fr <FR>`

  Default value: `2`



## `cosmopoeia create blank erode`

Runs an erosion process on the map

**Usage:** `cosmopoeia create blank erode [OPTIONS]`

###### **Options:**

* `--weathering-amount <WEATHERING_AMOUNT>` — Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)

  Default value: `1000`
* `--iterations <ITERATIONS>`

  Default value: `10`



## `cosmopoeia create blank seed-ocean`

Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)

**Usage:** `cosmopoeia create blank seed-ocean --count <COUNT> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia create blank fill-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia create blank fill-ocean`



## `cosmopoeia create blank flood-ocean`

Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found

**Usage:** `cosmopoeia create blank flood-ocean`



## `cosmopoeia create blank sample-ocean-masked`

Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean

**Usage:** `cosmopoeia create blank sample-ocean-masked <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data



## `cosmopoeia create blank sample-ocean-below`

Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean

**Usage:** `cosmopoeia create blank sample-ocean-below --elevation <ELEVATION> <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data

###### **Options:**

* `--elevation <ELEVATION>` — The elevation to compare to



## `cosmopoeia create blank sample-elevation`

Replaces elevations by sampling from a heightmap

**Usage:** `cosmopoeia create blank sample-elevation <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia terrain`

Runs a terrain process on the world to manipulate elevations or ocean status

**Usage:** `cosmopoeia terrain [OPTIONS] <TARGET> <COMMAND>`

###### **Subcommands:**

* `recipe` — Processes a series of pre-saved tasks
* `recipe-set` — Randomly chooses a recipe from a set of named recipes and follows it
* `clear` — Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0
* `clear-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `random-uniform` — Adds a uniform amount of random noise to the map
* `add-hill` — Adds hills or pits to a certain area of the map
* `add-range` — Adds a range of heights or a trough to a certain area of a map
* `add-strait` — Adds a long cut somewhere on the map
* `mask` — Changes the heights based on their distance from the edge of the map
* `invert` — Inverts the heights across the entire map
* `add` — Inverts the heights across the entier map
* `multiply` — Inverts the heights across the entier map
* `smooth` — Smooths elevations by averaging the value against it's neighbors
* `erode` — Runs an erosion process on the map
* `seed-ocean` — Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
* `fill-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `flood-ocean` — Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found
* `sample-ocean-masked` — Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean
* `sample-ocean-below` — Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean
* `sample-elevation` — Replaces elevations by sampling from a heightmap

###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--serialize` — Instead of processing, display the serialized value for inclusion in a recipe file



## `cosmopoeia terrain recipe`

Processes a series of pre-saved tasks

**Usage:** `cosmopoeia terrain recipe --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON File describing the tasks to complete



## `cosmopoeia terrain recipe-set`

Randomly chooses a recipe from a set of named recipes and follows it

**Usage:** `cosmopoeia terrain recipe-set [OPTIONS] --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON file containing a map of potential recipes to follow
* `--recipe <RECIPE>`



## `cosmopoeia terrain clear`

Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0

**Usage:** `cosmopoeia terrain clear`



## `cosmopoeia terrain clear-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia terrain clear-ocean`



## `cosmopoeia terrain random-uniform`

Adds a uniform amount of random noise to the map

**Usage:** `cosmopoeia terrain random-uniform [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia terrain add-hill`

Adds hills or pits to a certain area of the map

**Usage:** `cosmopoeia terrain add-hill --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia terrain add-range`

Adds a range of heights or a trough to a certain area of a map

**Usage:** `cosmopoeia terrain add-range --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia terrain add-strait`

Adds a long cut somewhere on the map

**Usage:** `cosmopoeia terrain add-strait --width <WIDTH> --direction <DIRECTION>`

###### **Options:**

* `--width <WIDTH>`
* `--direction <DIRECTION>`

  Possible values: `horizontal`, `vertical`




## `cosmopoeia terrain mask`

Changes the heights based on their distance from the edge of the map

**Usage:** `cosmopoeia terrain mask [OPTIONS]`

###### **Options:**

* `--power <POWER>`

  Default value: `1`



## `cosmopoeia terrain invert`

Inverts the heights across the entire map

**Usage:** `cosmopoeia terrain invert --probability <PROBABILITY> --axes <AXES>`

###### **Options:**

* `--probability <PROBABILITY>`
* `--axes <AXES>`

  Possible values: `x`, `y`, `both`




## `cosmopoeia terrain add`

Inverts the heights across the entier map

**Usage:** `cosmopoeia terrain add [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia terrain multiply`

Inverts the heights across the entier map

**Usage:** `cosmopoeia terrain multiply [OPTIONS] --height-factor <HEIGHT_FACTOR>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-factor <HEIGHT_FACTOR>`



## `cosmopoeia terrain smooth`

Smooths elevations by averaging the value against it's neighbors

**Usage:** `cosmopoeia terrain smooth [OPTIONS]`

###### **Options:**

* `--fr <FR>`

  Default value: `2`



## `cosmopoeia terrain erode`

Runs an erosion process on the map

**Usage:** `cosmopoeia terrain erode [OPTIONS]`

###### **Options:**

* `--weathering-amount <WEATHERING_AMOUNT>` — Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)

  Default value: `1000`
* `--iterations <ITERATIONS>`

  Default value: `10`



## `cosmopoeia terrain seed-ocean`

Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)

**Usage:** `cosmopoeia terrain seed-ocean --count <COUNT> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia terrain fill-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia terrain fill-ocean`



## `cosmopoeia terrain flood-ocean`

Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found

**Usage:** `cosmopoeia terrain flood-ocean`



## `cosmopoeia terrain sample-ocean-masked`

Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean

**Usage:** `cosmopoeia terrain sample-ocean-masked <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data



## `cosmopoeia terrain sample-ocean-below`

Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean

**Usage:** `cosmopoeia terrain sample-ocean-below --elevation <ELEVATION> <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data

###### **Options:**

* `--elevation <ELEVATION>` — The elevation to compare to



## `cosmopoeia terrain sample-elevation`

Replaces elevations by sampling from a heightmap

**Usage:** `cosmopoeia terrain sample-elevation <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia gen-climate`

Generates climate data for a world

**Usage:** `cosmopoeia gen-climate [OPTIONS] <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--equator-temp <EQUATOR_TEMP>` — The rough temperature (in celsius) at the equator

  Default value: `27`
* `--polar-temp <POLAR_TEMP>` — The rough temperature (in celsius) at the poles

  Default value: `-30`
* `--north-polar-wind <NORTH_POLAR_WIND>` — Wind direction above latitude 60 N

  Default value: `225`
* `--north-middle-wind <NORTH_MIDDLE_WIND>` — Wind direction from latitude 30 N to 60 N

  Default value: `45`
* `--north-tropical-wind <NORTH_TROPICAL_WIND>` — Wind direction from the equator to latitude 30 N

  Default value: `225`
* `--south-tropical-wind <SOUTH_TROPICAL_WIND>` — Wind direction from the equator to latitude 30 S

  Default value: `315`
* `--south-middle-wind <SOUTH_MIDDLE_WIND>` — Wind direction from latitude 30 S to 60 S

  Default value: `135`
* `--south-polar-wind <SOUTH_POLAR_WIND>` — Wind direction below latitude 60 S

  Default value: `315`
* `--wind-range <WIND_RANGE>` — Specify a range of latitudes and a wind direction (S lat..N lat:Direction), later mappings will override earlier
* `--precipitation-factor <PRECIPITATION_FACTOR>` — Amount of global moisture on a scale of roughly 0-5, but there is no limit

  Default value: `1`



## `cosmopoeia gen-water`

Generates water features for a world

**Usage:** `cosmopoeia gen-water [OPTIONS] <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--lake-buffer-scale <LAKE_BUFFER_SCALE>` — This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes

  Default value: `2`
* `--overwrite-coastline` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-ocean` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-lakes` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-rivers` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-all` — If true and any layer already exists in the file, it will be overwritten. This overrides all of the other 'overwrite_' switches to true



## `cosmopoeia gen-biome`

Generates biomes for a world

**Usage:** `cosmopoeia gen-biome [OPTIONS] <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--overwrite-biomes` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia gen-people`

Generates populations and cultures for a world

**Usage:** `cosmopoeia gen-people [OPTIONS] --cultures <CULTURES> --namers <NAMERS> <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--cultures <CULTURES>` — Files to load culture sets from, more than one may be specified to load multiple culture sets
* `--culture-count <CULTURE_COUNT>` — The number of cultures to generate

  Default value: `15`
* `--river-threshold <RIVER_THRESHOLD>` — A waterflow threshold above which the tile will count as a river

  Default value: `10`
* `--expansion-factor <EXPANSION_FACTOR>` — A number, usually ranging from 0.1 to 2.0, which limits how far cultures and nations will expand. The higher the number, the fewer neutral lands

  Default value: `1`
* `--namers <NAMERS>` — Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones
* `--default-namer <DEFAULT_NAMER>` — The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
* `--size-variance <SIZE_VARIANCE>` — A number, clamped to 0-10, which controls how much cultures can vary in size

  Default value: `1`
* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--overwrite-cultures` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia gen-towns`

Generates towns, cities and other urban centers for a world

**Usage:** `cosmopoeia gen-towns [OPTIONS] --namers <NAMERS> <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--capital-count <CAPITAL_COUNT>` — The number of national capitals to create. If not specified 1 capital will be generated for every 1,000 square degrees of the world, subject to habitability and tile count limits
* `--town-count <TOWN_COUNT>` — The number of non-capital towns to create. If not specified, 1 town will be generated for every 100 square degrees, subject to habitability and tile count limits
* `--namers <NAMERS>` — Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones
* `--default-namer <DEFAULT_NAMER>` — The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--river-threshold <RIVER_THRESHOLD>` — A waterflow threshold above which the tile will count as a river

  Default value: `10`
* `--overwrite-towns` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia gen-nations`

Generates nations for a world

**Usage:** `cosmopoeia gen-nations [OPTIONS] --namers <NAMERS> <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--namers <NAMERS>` — Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones
* `--default-namer <DEFAULT_NAMER>` — The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
* `--size-variance <SIZE_VARIANCE>` — A number, clamped to 0-10, which controls how much cultures can vary in size

  Default value: `1`
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--river-threshold <RIVER_THRESHOLD>` — A waterflow threshold above which the tile will count as a river

  Default value: `10`
* `--expansion-factor <EXPANSION_FACTOR>` — A number, usually ranging from 0.1 to 2.0, which limits how far cultures and nations will expand. The higher the number, the fewer neutral lands

  Default value: `1`
* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--overwrite-nations` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia gen-subnations`

Generates subnations (provinces and other administrative divisions) for a world

**Usage:** `cosmopoeia gen-subnations [OPTIONS] --namers <NAMERS> <TARGET>`

###### **Subcommands:**


###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--namers <NAMERS>` — Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones
* `--default-namer <DEFAULT_NAMER>` — The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
* `--subnation-percentage <SUBNATION_PERCENTAGE>` — The percent of towns in each nation to use for subnations

  Default value: `20`
* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--overwrite-subnations` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists



## `cosmopoeia big-bang`

Creates a world map, generates natural features, and populates it with nations and subnations

**Usage:** `cosmopoeia big-bang [OPTIONS] --namers <NAMERS> --cultures <CULTURES> <TARGET> <SOURCE>`

###### **Subcommands:**

* `from-heightmap` — Creates voronoi tiles in the same extent as a heightmap with zero elevation
* `blank` — Creates voronoi tiles in the given extent with zero elevation

###### **Arguments:**

* `<TARGET>` — The path to the world map GeoPackage file

###### **Options:**

* `--namers <NAMERS>` — Files to load name generators from, more than one may be specified to load multiple languages. Later language names will override previous ones
* `--default-namer <DEFAULT_NAMER>` — The name generator to use for naming towns in tiles without a culture, or one will be randomly chosen
* `--cultures <CULTURES>` — Files to load culture sets from, more than one may be specified to load multiple culture sets
* `--culture-count <CULTURE_COUNT>` — The number of cultures to generate

  Default value: `15`
* `--seed <SEED>` — Seed for the random number generator, note that this might not reproduce the same over different versions and configurations of nfmt
* `--tile-count <TILE_COUNT>` — The rough number of tiles to generate for the image

  Default value: `10000`
* `--equator-temp <EQUATOR_TEMP>` — The rough temperature (in celsius) at the equator

  Default value: `27`
* `--polar-temp <POLAR_TEMP>` — The rough temperature (in celsius) at the poles

  Default value: `-30`
* `--north-polar-wind <NORTH_POLAR_WIND>` — Wind direction above latitude 60 N

  Default value: `225`
* `--north-middle-wind <NORTH_MIDDLE_WIND>` — Wind direction from latitude 30 N to 60 N

  Default value: `45`
* `--north-tropical-wind <NORTH_TROPICAL_WIND>` — Wind direction from the equator to latitude 30 N

  Default value: `225`
* `--south-tropical-wind <SOUTH_TROPICAL_WIND>` — Wind direction from the equator to latitude 30 S

  Default value: `315`
* `--south-middle-wind <SOUTH_MIDDLE_WIND>` — Wind direction from latitude 30 S to 60 S

  Default value: `135`
* `--south-polar-wind <SOUTH_POLAR_WIND>` — Wind direction below latitude 60 S

  Default value: `315`
* `--wind-range <WIND_RANGE>` — Specify a range of latitudes and a wind direction (S lat..N lat:Direction), later mappings will override earlier
* `--precipitation-factor <PRECIPITATION_FACTOR>` — Amount of global moisture on a scale of roughly 0-5, but there is no limit

  Default value: `1`
* `--bezier-scale <BEZIER_SCALE>` — This number is used for generating points to make curvy lines. The higher the number, the smoother the curves

  Default value: `100`
* `--lake-buffer-scale <LAKE_BUFFER_SCALE>` — This number is used for determining a buffer between the lake and the tile. The higher the number, the smaller and simpler the lakes

  Default value: `2`
* `--river-threshold <RIVER_THRESHOLD>` — A waterflow threshold above which the tile will count as a river

  Default value: `10`
* `--size-variance <SIZE_VARIANCE>` — A number, clamped to 0-10, which controls how much cultures can vary in size

  Default value: `1`
* `--expansion-factor <EXPANSION_FACTOR>` — A number, usually ranging from 0.1 to 2.0, which limits how far cultures and nations will expand. The higher the number, the fewer neutral lands

  Default value: `1`
* `--capital-count <CAPITAL_COUNT>` — The number of national capitals to create. If not specified 1 capital will be generated for every 1,000 square degrees of the world, subject to habitability and tile count limits
* `--town-count <TOWN_COUNT>` — The number of non-capital towns to create. If not specified, 1 town will be generated for every 100 square degrees, subject to habitability and tile count limits
* `--subnation-percentage <SUBNATION_PERCENTAGE>` — The percent of towns in each nation to use for subnations

  Default value: `20`
* `--overwrite-tiles` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-coastline` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-ocean` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-lakes` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-rivers` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-biomes` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-cultures` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-towns` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-nations` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-subnations` — If true and the [<$layer>] layer already exists in the file, it will be overwritten. Otherwise, an error will occur if the layer exists
* `--overwrite-all` — If true and any layer already exists in the file, it will be overwritten. This overrides all of the other 'overwrite_' switches to true



## `cosmopoeia big-bang from-heightmap`

Creates voronoi tiles in the same extent as a heightmap with zero elevation

**Usage:** `cosmopoeia big-bang from-heightmap <SOURCE> [COMMAND]`

###### **Subcommands:**

* `recipe` — Processes a series of pre-saved tasks
* `recipe-set` — Randomly chooses a recipe from a set of named recipes and follows it
* `clear` — Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0
* `clear-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `random-uniform` — Adds a uniform amount of random noise to the map
* `add-hill` — Adds hills or pits to a certain area of the map
* `add-range` — Adds a range of heights or a trough to a certain area of a map
* `add-strait` — Adds a long cut somewhere on the map
* `mask` — Changes the heights based on their distance from the edge of the map
* `invert` — Inverts the heights across the entire map
* `add` — Inverts the heights across the entier map
* `multiply` — Inverts the heights across the entier map
* `smooth` — Smooths elevations by averaging the value against it's neighbors
* `erode` — Runs an erosion process on the map
* `seed-ocean` — Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
* `fill-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `flood-ocean` — Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found
* `sample-ocean-masked` — Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean
* `sample-ocean-below` — Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean
* `sample-elevation` — Replaces elevations by sampling from a heightmap

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia big-bang from-heightmap recipe`

Processes a series of pre-saved tasks

**Usage:** `cosmopoeia big-bang from-heightmap recipe --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON File describing the tasks to complete



## `cosmopoeia big-bang from-heightmap recipe-set`

Randomly chooses a recipe from a set of named recipes and follows it

**Usage:** `cosmopoeia big-bang from-heightmap recipe-set [OPTIONS] --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON file containing a map of potential recipes to follow
* `--recipe <RECIPE>`



## `cosmopoeia big-bang from-heightmap clear`

Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0

**Usage:** `cosmopoeia big-bang from-heightmap clear`



## `cosmopoeia big-bang from-heightmap clear-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia big-bang from-heightmap clear-ocean`



## `cosmopoeia big-bang from-heightmap random-uniform`

Adds a uniform amount of random noise to the map

**Usage:** `cosmopoeia big-bang from-heightmap random-uniform [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia big-bang from-heightmap add-hill`

Adds hills or pits to a certain area of the map

**Usage:** `cosmopoeia big-bang from-heightmap add-hill --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang from-heightmap add-range`

Adds a range of heights or a trough to a certain area of a map

**Usage:** `cosmopoeia big-bang from-heightmap add-range --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang from-heightmap add-strait`

Adds a long cut somewhere on the map

**Usage:** `cosmopoeia big-bang from-heightmap add-strait --width <WIDTH> --direction <DIRECTION>`

###### **Options:**

* `--width <WIDTH>`
* `--direction <DIRECTION>`

  Possible values: `horizontal`, `vertical`




## `cosmopoeia big-bang from-heightmap mask`

Changes the heights based on their distance from the edge of the map

**Usage:** `cosmopoeia big-bang from-heightmap mask [OPTIONS]`

###### **Options:**

* `--power <POWER>`

  Default value: `1`



## `cosmopoeia big-bang from-heightmap invert`

Inverts the heights across the entire map

**Usage:** `cosmopoeia big-bang from-heightmap invert --probability <PROBABILITY> --axes <AXES>`

###### **Options:**

* `--probability <PROBABILITY>`
* `--axes <AXES>`

  Possible values: `x`, `y`, `both`




## `cosmopoeia big-bang from-heightmap add`

Inverts the heights across the entier map

**Usage:** `cosmopoeia big-bang from-heightmap add [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia big-bang from-heightmap multiply`

Inverts the heights across the entier map

**Usage:** `cosmopoeia big-bang from-heightmap multiply [OPTIONS] --height-factor <HEIGHT_FACTOR>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-factor <HEIGHT_FACTOR>`



## `cosmopoeia big-bang from-heightmap smooth`

Smooths elevations by averaging the value against it's neighbors

**Usage:** `cosmopoeia big-bang from-heightmap smooth [OPTIONS]`

###### **Options:**

* `--fr <FR>`

  Default value: `2`



## `cosmopoeia big-bang from-heightmap erode`

Runs an erosion process on the map

**Usage:** `cosmopoeia big-bang from-heightmap erode [OPTIONS]`

###### **Options:**

* `--weathering-amount <WEATHERING_AMOUNT>` — Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)

  Default value: `1000`
* `--iterations <ITERATIONS>`

  Default value: `10`



## `cosmopoeia big-bang from-heightmap seed-ocean`

Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)

**Usage:** `cosmopoeia big-bang from-heightmap seed-ocean --count <COUNT> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang from-heightmap fill-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia big-bang from-heightmap fill-ocean`



## `cosmopoeia big-bang from-heightmap flood-ocean`

Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found

**Usage:** `cosmopoeia big-bang from-heightmap flood-ocean`



## `cosmopoeia big-bang from-heightmap sample-ocean-masked`

Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean

**Usage:** `cosmopoeia big-bang from-heightmap sample-ocean-masked <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data



## `cosmopoeia big-bang from-heightmap sample-ocean-below`

Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean

**Usage:** `cosmopoeia big-bang from-heightmap sample-ocean-below --elevation <ELEVATION> <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data

###### **Options:**

* `--elevation <ELEVATION>` — The elevation to compare to



## `cosmopoeia big-bang from-heightmap sample-elevation`

Replaces elevations by sampling from a heightmap

**Usage:** `cosmopoeia big-bang from-heightmap sample-elevation <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



## `cosmopoeia big-bang blank`

Creates voronoi tiles in the given extent with zero elevation

**Usage:** `cosmopoeia big-bang blank [OPTIONS] <HEIGHT> <WIDTH> <SOUTH> <WEST> [COMMAND]`

###### **Subcommands:**

* `recipe` — Processes a series of pre-saved tasks
* `recipe-set` — Randomly chooses a recipe from a set of named recipes and follows it
* `clear` — Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0
* `clear-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `random-uniform` — Adds a uniform amount of random noise to the map
* `add-hill` — Adds hills or pits to a certain area of the map
* `add-range` — Adds a range of heights or a trough to a certain area of a map
* `add-strait` — Adds a long cut somewhere on the map
* `mask` — Changes the heights based on their distance from the edge of the map
* `invert` — Inverts the heights across the entire map
* `add` — Inverts the heights across the entier map
* `multiply` — Inverts the heights across the entier map
* `smooth` — Smooths elevations by averaging the value against it's neighbors
* `erode` — Runs an erosion process on the map
* `seed-ocean` — Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
* `fill-ocean` — Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
* `flood-ocean` — Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found
* `sample-ocean-masked` — Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean
* `sample-ocean-below` — Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean
* `sample-elevation` — Replaces elevations by sampling from a heightmap

###### **Arguments:**

* `<HEIGHT>` — the height (from north to south) in degrees of the world extents
* `<WIDTH>` — the width in degrees of the world extents
* `<SOUTH>` — the latitude of the southern border of the world extents
* `<WEST>` — the longitude of the western border of the world extents

###### **Options:**

* `--min-elevation <MIN_ELEVATION>` — minimum elevation for heightmap

  Default value: `-11000`
* `--max-elevation <MAX_ELEVATION>` — maximum elevation for heightmap

  Default value: `9000`



## `cosmopoeia big-bang blank recipe`

Processes a series of pre-saved tasks

**Usage:** `cosmopoeia big-bang blank recipe --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON File describing the tasks to complete



## `cosmopoeia big-bang blank recipe-set`

Randomly chooses a recipe from a set of named recipes and follows it

**Usage:** `cosmopoeia big-bang blank recipe-set [OPTIONS] --source <SOURCE>`

###### **Options:**

* `--source <SOURCE>` — JSON file containing a map of potential recipes to follow
* `--recipe <RECIPE>`



## `cosmopoeia big-bang blank clear`

Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0

**Usage:** `cosmopoeia big-bang blank clear`



## `cosmopoeia big-bang blank clear-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia big-bang blank clear-ocean`



## `cosmopoeia big-bang blank random-uniform`

Adds a uniform amount of random noise to the map

**Usage:** `cosmopoeia big-bang blank random-uniform [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia big-bang blank add-hill`

Adds hills or pits to a certain area of the map

**Usage:** `cosmopoeia big-bang blank add-hill --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang blank add-range`

Adds a range of heights or a trough to a certain area of a map

**Usage:** `cosmopoeia big-bang blank add-range --count <COUNT> --height-delta <HEIGHT_DELTA> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--height-delta <HEIGHT_DELTA>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang blank add-strait`

Adds a long cut somewhere on the map

**Usage:** `cosmopoeia big-bang blank add-strait --width <WIDTH> --direction <DIRECTION>`

###### **Options:**

* `--width <WIDTH>`
* `--direction <DIRECTION>`

  Possible values: `horizontal`, `vertical`




## `cosmopoeia big-bang blank mask`

Changes the heights based on their distance from the edge of the map

**Usage:** `cosmopoeia big-bang blank mask [OPTIONS]`

###### **Options:**

* `--power <POWER>`

  Default value: `1`



## `cosmopoeia big-bang blank invert`

Inverts the heights across the entire map

**Usage:** `cosmopoeia big-bang blank invert --probability <PROBABILITY> --axes <AXES>`

###### **Options:**

* `--probability <PROBABILITY>`
* `--axes <AXES>`

  Possible values: `x`, `y`, `both`




## `cosmopoeia big-bang blank add`

Inverts the heights across the entier map

**Usage:** `cosmopoeia big-bang blank add [OPTIONS] --height-delta <HEIGHT_DELTA>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-delta <HEIGHT_DELTA>`



## `cosmopoeia big-bang blank multiply`

Inverts the heights across the entier map

**Usage:** `cosmopoeia big-bang blank multiply [OPTIONS] --height-factor <HEIGHT_FACTOR>`

###### **Options:**

* `--height-filter <HEIGHT_FILTER>`
* `--height-factor <HEIGHT_FACTOR>`



## `cosmopoeia big-bang blank smooth`

Smooths elevations by averaging the value against it's neighbors

**Usage:** `cosmopoeia big-bang blank smooth [OPTIONS]`

###### **Options:**

* `--fr <FR>`

  Default value: `2`



## `cosmopoeia big-bang blank erode`

Runs an erosion process on the map

**Usage:** `cosmopoeia big-bang blank erode [OPTIONS]`

###### **Options:**

* `--weathering-amount <WEATHERING_AMOUNT>` — Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)

  Default value: `1000`
* `--iterations <ITERATIONS>`

  Default value: `10`



## `cosmopoeia big-bang blank seed-ocean`

Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)

**Usage:** `cosmopoeia big-bang blank seed-ocean --count <COUNT> --x-filter <X_FILTER> --y-filter <Y_FILTER>`

###### **Options:**

* `--count <COUNT>`
* `--x-filter <X_FILTER>`
* `--y-filter <Y_FILTER>`



## `cosmopoeia big-bang blank fill-ocean`

Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)

**Usage:** `cosmopoeia big-bang blank fill-ocean`



## `cosmopoeia big-bang blank flood-ocean`

Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found

**Usage:** `cosmopoeia big-bang blank flood-ocean`



## `cosmopoeia big-bang blank sample-ocean-masked`

Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean

**Usage:** `cosmopoeia big-bang blank sample-ocean-masked <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data



## `cosmopoeia big-bang blank sample-ocean-below`

Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean

**Usage:** `cosmopoeia big-bang blank sample-ocean-below --elevation <ELEVATION> <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the ocean data

###### **Options:**

* `--elevation <ELEVATION>` — The elevation to compare to



## `cosmopoeia big-bang blank sample-elevation`

Replaces elevations by sampling from a heightmap

**Usage:** `cosmopoeia big-bang blank sample-elevation <SOURCE>`

###### **Arguments:**

* `<SOURCE>` — The path to the heightmap containing the elevation data



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
