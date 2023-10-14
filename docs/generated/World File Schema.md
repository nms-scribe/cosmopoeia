# World File Schema


The world file output by Cosmpoeia is stored in a Geopackage (GPKG) file. This is a SQLite database that includes some pre-built tables for storing geographic information. It is best edited with GIS software that supports the format. Below is a description of the layers, or tables, contained inside the database, and field information. The field types given are internal to the software, and their database storage field type is defined in the Field Types scetion.

**On the FID field and Table Order**: Every layer in the file has an identifier field called `fid`, which contains a unique identifier for the field. This is handled by the gdal library, which Cosmopoeia uses for access to the file. Here are a few details:

* According to the [Geopackage standard](http://www.geopackage.org/spec131/index.html#feature_user_tables), the identifier field (which is called fid by default in gdal), is created with the following constraint in SQLite: `INTEGER PRIMARY KEY AUTOINCREMENT`.
* According to [SQLite documentation](https://www.sqlite.org/autoinc.html), a key defined in this way is guaranteed not to be reused, and appears to be possible to represent insertion order, as long as no parallel transactions are occurring, which I do not allow in the same instance of the program.
* According to tests, at least sometimes, when iterating through features, the features are returned from the database in fid order. I do not believe that this is guaranteed by any mechanism from gdal or sqlite.
* According to tests, a rust hashmap does not iterate over items in entry order. For this reason, I use a special map structure that iterates in fid order. This attempts to make it more likely that random operations with the same seed are always reproducible with the same input.


## Layer `tiles`
**geometry**: Polygon



### `site_x`
**database field type**: Real

longitude of the node point for the tile's voronoi

### `site_y`
**database field type**: Real

latitude of the node point for the tile's voronoi

### `elevation`
**database field type**: Real

elevation in meters of the node point for the tile's voronoi

### `elevation_scaled`
**database field type**: Signed Integer

elevation scaled into a value from 0 to 100, where 20 is sea-level.

### `grouping`
**database field type**: Grouping

Indicates whether the tile is part of the ocean, an island, a continent, a lake, and maybe others.

### `grouping_id`
**database field type**: ID Reference

A unique id for each grouping. These id's do not map to other tables, but will tell when tiles are in the same group. Use lake_id to link to the lake table.

### `temperature`
**database field type**: Real

average annual temperature of tile in imaginary units

### `wind`
**database field type**: Angle

roughly estimated average wind direction for tile

### `precipitation`
**database field type**: Real

average annual precipitation of tile in imaginary units

### `water_flow`
**database field type**: Real

amount of water flow through tile in imaginary units

### `water_accumulation`
**database field type**: Real

amount of water accumulating (because it couldn't flow on) in imaginary units

### `lake_id`
**database field type**: Optional ID Reference

if the tile is in a lake, this is the id of the lake in the lakes layer

### `flow_to`
**database field type**: List of Neighbor

id of neighboring tile which water flows to

### `shore_distance`
**database field type**: Signed Integer

shortest distance in number of tiles to an ocean or lake shoreline. This will be positive on land and negative inside a water body.

### `harbor_tile_id`
**database field type**: Optional Neighbor

If this is a land tile neighboring a water body, this is the id of the closest tile

### `water_count`
**database field type**: Optional Signed Integer

if this is a land tile neighboring a water body, this is the number of neighbor tiles that are water

### `biome`
**database field type**: String

The biome for this tile

### `habitability`
**database field type**: Real

the factor used to generate population numbers, along with the area of the tile

### `population`
**database field type**: Signed Integer

base population of the cell outside of the towns.

### `culture`
**database field type**: Optional String

The name of the culture assigned to this tile, unless wild

### `town_id`
**database field type**: Optional ID Reference

if the tile has a town, this is the id of the town in the towns layer

### `nation_id`
**database field type**: Optional ID Reference

if the tile is part of a nation, this is the id of the nation which controls it

### `subnation_id`
**database field type**: Optional ID Reference

if the tile is part of a subnation, this is the id of the nation which controls it

### `outlet_from`
**database field type**: Optional Neighbor

If this tile is an outlet from a lake, this is the neighbor from which the water is flowing.

### `neighbors`
**database field type**: NeighborAndDirection

A list of all tile neighbors and their angular directions (tile_id:direction)

### `edge`
**database field type**: Optional Edge

A value indicating whether the tile is on the edge of the map


## Layer `biomes`
**geometry**: MultiPolygon



### `name`
**database field type**: String



### `habitability`
**database field type**: Signed Integer



### `criteria`
**database field type**: BiomeCriteria



### `movement_cost`
**database field type**: Signed Integer



### `supports_nomadic`
**database field type**: Boolean



### `supports_hunting`
**database field type**: Boolean



### `color`
**database field type**: Color




## Layer `coastlines`
**geometry**: Polygon




## Layer `cultures`
**geometry**: MultiPolygon



### `name`
**database field type**: String



### `namer`
**database field type**: String



### `type_`
**database field type**: CultureType



### `expansionism`
**database field type**: Real



### `center_tile_id`
**database field type**: ID Reference



### `color`
**database field type**: Color




## Layer `lakes`
**geometry**: MultiPolygon



### `elevation`
**database field type**: Real



### `type_`
**database field type**: LakeType



### `flow`
**database field type**: Real



### `size`
**database field type**: Signed Integer



### `temperature`
**database field type**: Real



### `evaporation`
**database field type**: Real




## Layer `nations`
**geometry**: MultiPolygon



### `name`
**database field type**: String



### `culture`
**database field type**: Optional String



### `center_tile_id`
**database field type**: ID Reference



### `type_`
**database field type**: CultureType



### `expansionism`
**database field type**: Real



### `capital_town_id`
**database field type**: ID Reference



### `color`
**database field type**: Color




## Layer `oceans`
**geometry**: Polygon




## Layer `properties`
**geometry**: NoGeometry



### `name`
**database field type**: String



### `value`
**database field type**: String




## Layer `rivers`
**geometry**: MultiLineString



### `from_tile_id`
**database field type**: ID Reference



### `from_type`
**database field type**: RiverSegmentFrom



### `from_flow`
**database field type**: Real



### `to_tile_id`
**database field type**: Neighbor



### `to_type`
**database field type**: RiverSegmentTo



### `to_flow`
**database field type**: Real




## Layer `subnations`
**geometry**: MultiPolygon



### `name`
**database field type**: String



### `culture`
**database field type**: Optional String



### `center_tile_id`
**database field type**: ID Reference



### `type_`
**database field type**: CultureType



### `seat_town_id`
**database field type**: Optional ID Reference



### `nation_id`
**database field type**: ID Reference



### `color`
**database field type**: Color




## Layer `towns`
**geometry**: Point



### `name`
**database field type**: String



### `culture`
**database field type**: Optional String



### `is_capital`
**database field type**: Boolean



### `tile_id`
**database field type**: ID Reference



### `grouping_id`
**database field type**: ID Reference



### `population`
**database field type**: Signed Integer



### `is_port`
**database field type**: Boolean




## Field Types
### Angle
**storage type**: Real
**syntax**: `<real>`

A real number from 0 to 360.

### BiomeCriteria
**storage type**: String
**syntax**: `"Glacier" | "Matrix" (List of Unsigned Integer Pair) | "Ocean" | "Wetland"`

Criteria for how the biome is to be mapped to the world based on generated climate data.
* Glacier: This biome should be used for glacier -- only one is allowed
* Matrix: The biome should be placed in the following locations in the moisture and temperature matrix -- coordinates must not be used for another biome
* Ocean: The biome should be used for ocean -- only one is allowed
* Wetland: The biome should be used for wetland -- only one is allowed


### Boolean
**storage type**: Integer
**syntax**: `<bool>`

An value of 1 or 0

### Color
**storage type**: String
**syntax**: `<color>`

A color in #RRGGBB syntax.

### CultureType
**storage type**: String
**syntax**: `"Generic" | "Highland" | "Hunting" | "Lake" | "Naval" | "Nomadic" | "River"`

The name for the type of culture, which specifies how the culture behaves during generation
* Generic: A culture with no landscape preferences, created when no other culture type is suggested
* Highland: A culture that prefers higher elevations
* Hunting: A culture that prefers forested landscapes
* Lake: A culture that prefers to live on the shore of lakes
* Naval: A culture that prefers ocean shores
* Nomadic: A culture that prevers drier elevations
* River: A culture that prefers to live along rivers


### Edge
**storage type**: String
**syntax**: `"North" | "Northeast" | "East" | "Southeast" | "South" | "Southwest" | "West" | "Northwest"`

The name of a side or corner of the map.
* North: The north edge of the map
* Northeast: The northeast corner of the map
* East: The east edge of the map
* Southeast: The southeast corner of the map
* South: The south edge of the map
* Southwest: The southwest corner of the map
* West: The west edge of the map
* Northwest: The northwest corner of the map


### Grouping
**storage type**: String
**syntax**: `"Continent" | "Island" | "Islet" | "Lake" | "LakeIsland" | "Ocean"`

A type of land or water feature.
* Continent: A large land mass surrounded by ocean or the edge of the map if no ocean
* Island: A small land mass surrounded by ocean
* Islet: A smaller land mass surrounded by ocean
* Lake: A body of water created from rainfall, usually not at elevation 0.
* LakeIsland: A land mass surrounded by a lake
* Ocean: A body of water created by flooding the terrain to elevation 0.


### ID Reference
**storage type**: String
**syntax**: `<integer>`

A reference to the 'fid' field in another table. This is stored as a String field because an unsigned integer field is not available.

### LakeType
**storage type**: String
**syntax**: `"Fresh" | "Salt" | "Frozen" | "Pluvial" | "Dry" | "Marsh"`

A name for a type of lake.
* Fresh: Lake is freshwater
* Salt: Lake is saltwater
* Frozen: Lake is frozen
* Pluvial: Lake is intermittent
* Dry: Lakebed is dry
* Marsh: Lake is shallow


### List of Neighbor
**storage type**: String
**syntax**: `[<Neighbor>, ..]`

A list of comma-separated Neighbor values in brackets.

### List of Unsigned Integer Pair
**storage type**: String
**syntax**: `[<Unsigned Integer Pair>, ..]`

A list of comma-separated Unsigned Integer Pair values in brackets.

### Neighbor
**storage type**: String
**syntax**: `<integer> | (<integer>,<Edge>) | <Edge>`

Specifies a type of neighbor for a tile. There are three possibilities. They are described by their contents, not their name, in order to simplify the NeighborDirection fields.
* Tile: a regular contiguous tile, which is specified by it's id.
* CrossMap: a tile that sits on the opposite side of the map, specified by it's id and direction as an 'Edge'.
* OffMap: unknown content that is off the edges of the map, specified merely by a direction as an 'Edge'

### NeighborAndDirection
**storage type**: String
**syntax**: `(<Neighbor>,<real>)`

A pair of Neighbor and angular direction (in degrees, clockwise from north) surrounded by parentheses.

### Optional Edge
**storage type**: String
**syntax**: `"North" | "Northeast" | "East" | "Southeast" | "South" | "Southwest" | "West" | "Northwest"?`

The name of a side or corner of the map.
* North: The north edge of the map
* Northeast: The northeast corner of the map
* East: The east edge of the map
* Southeast: The southeast corner of the map
* South: The south edge of the map
* Southwest: The southwest corner of the map
* West: The west edge of the map
* Northwest: The northwest corner of the map


### Optional ID Reference
**storage type**: String
**syntax**: `<integer>?`

A reference to the 'fid' field in another table. This is stored as a String field because an unsigned integer field is not available.

### Optional Neighbor
**storage type**: String
**syntax**: `<integer> | (<integer>,<Edge>) | <Edge>?`

Specifies a type of neighbor for a tile. There are three possibilities. They are described by their contents, not their name, in order to simplify the NeighborDirection fields.
* Tile: a regular contiguous tile, which is specified by it's id.
* CrossMap: a tile that sits on the opposite side of the map, specified by it's id and direction as an 'Edge'.
* OffMap: unknown content that is off the edges of the map, specified merely by a direction as an 'Edge'

### Optional Signed Integer
**storage type**: Integer
**syntax**: `<integer>?`

A signed integer.

### Optional String
**storage type**: String
**syntax**: `<string>?`

A string of text

### Real
**storage type**: Real
**syntax**: `<real>`

A real number.

### RiverSegmentFrom
**storage type**: String
**syntax**: `"Branch" | "BranchingConfluence" | "BranchingLake" | "Confluence" | "Continuing" | "Lake" | "Source"`

A name for how the river segment begins
* Branch: The segment begins with the splitting of another river
* BranchingConfluence: The segment begins with a split that coincides with a confluence in the same tile
* BranchingLake: The segment begins with a split coming out of a lake
* Confluence: The segment begins with the joining of two rivers
* Continuing: The segment begins at the end of a single other segment
* Lake: The segment begins at the outflow of a lake
* Source: The segment begins where no other segment ends, with enough waterflow to make a river


### RiverSegmentTo
**storage type**: String
**syntax**: `"Branch" | "BranchingConfluence" | "Confluence" | "Continuing" | "Mouth"`

A name for how a river segment ends
* Branch: The segment ends by branching into multiple segments
* BranchingConfluence: The segment ends by branching where other segments also join
* Confluence: The segment ends by joining with another segment
* Continuing: The segment ends with the beginning of a single other segment
* Mouth: The segment ends by emptying into an ocean or lake


### Signed Integer
**storage type**: Integer
**syntax**: `<integer>`

A signed integer.

### String
**storage type**: String
**syntax**: `<string>`

A string of text

### Unsigned Integer Pair
**storage type**: String
**syntax**: `(<integer>,<integer>)`

A pair of unsigned integers in parentheses.

