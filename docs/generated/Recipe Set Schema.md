# Map_of_Array_of_Command

## Additional Properties

* **Additional Properties** *(Array)*
  * **Items**:
    * *([Command](#definitions/Command))*
## Definitions
  * <a id="definitions/Command"></a>**`Command`**
    * **One of**
      * *(Object)*: Processes a series of pre-saved tasks
        * **`source`** *(String, Required)*: JSON File describing the tasks to complete
        * **`task`** *(String, Required)*: Must be: "Recipe"
      * *(Object)*: Randomly chooses a recipe from a set of named recipes and follows it
        * **`recipe`** *(String | Null)*
        * **`source`** *(String, Required)*: JSON file containing a map of potential recipes to follow
        * **`task`** *(String, Required)*: Must be: "RecipeSet"
      * *(Object)*: Clears all elevations to 0 and all groupings to "Continent". This is an alias for Multiplying all height by 0.0.
        * **`task`** *(String, Required)*: Must be: "Clear"
      * *(Object)*: Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
        * **`task`** *(String, Required)*: Must be: "ClearOcean"
      * *(Object)*: Adds a uniform amount of random noise to the map
        * **`height_delta`** *([Range_int8](#definitions/Range_int8), Required)*
        * **`height_filter`**
          * **Any of**
            * *([Range_int8](#definitions/Range_int8))*
            * *(Null)*
        * **`task`** *(String, Required)*: Must be: "RandomUniform"
      * *(Object)*: Adds hills or pits to a certain area of the map
        * **`count`** *([Range_uint](#definitions/Range_uint), Required)*
        * **`height_delta`** *([Range_int8](#definitions/Range_int8), Required)*
        * **`task`** *(String, Required)*: Must be: "AddHill"
        * **`x_filter`** *([Range_double](#definitions/Range_double), Required)*
        * **`y_filter`** *([Range_double](#definitions/Range_double), Required)*
      * *(Object)*: Adds a range of heights or a trough to a certain area of a map
        * **`count`** *([Range_uint](#definitions/Range_uint), Required)*
        * **`height_delta`** *([Range_int8](#definitions/Range_int8), Required)*
        * **`task`** *(String, Required)*: Must be: "AddRange"
        * **`x_filter`** *([Range_double](#definitions/Range_double), Required)*
        * **`y_filter`** *([Range_double](#definitions/Range_double), Required)*
      * *(Object)*: Adds a long cut somewhere on the map
        * **`direction`** *([StraitDirection](#definitions/StraitDirection), Required)*
        * **`task`** *(String, Required)*: Must be: "AddStrait"
        * **`width`** *([Range_double](#definitions/Range_double), Required)*
      * *(Object)*: Changes the heights based on their distance from the edge of the map
        * **`power`** *(Number, Format: double, Required)*
        * **`task`** *(String, Required)*: Must be: "Mask"
      * *(Object)*: Inverts the heights across the entire map
        * **`axes`** *([InvertAxes](#definitions/InvertAxes), Required)*
        * **`probability`** *(Number, Format: double, Required)*
        * **`task`** *(String, Required)*: Must be: "Invert"
      * *(Object)*: Inverts the heights across the entier map
        * **`height_delta`** *(Integer, Format: int8, Required)*
        * **`height_filter`**
          * **Any of**
            * *([Range_int8](#definitions/Range_int8))*
            * *(Null)*
        * **`task`** *(String, Required)*: Must be: "Add"
      * *(Object)*: Inverts the heights across the entier map
        * **`height_factor`** *(Number, Format: double, Required)*
        * **`height_filter`**
          * **Any of**
            * *([Range_int8](#definitions/Range_int8))*
            * *(Null)*
        * **`task`** *(String, Required)*: Must be: "Multiply"
      * *(Object)*: Smooths elevations by averaging the value against it's neighbors.
        * **`fr`** *(Number, Format: double, Required)*
        * **`task`** *(String, Required)*: Must be: "Smooth"
      * *(Object)*: Runs an erosion process on the map
        * **`iterations`** *(Integer, Format: uint, Required)*: Minimum: `0`
        * **`task`** *(String, Required)*: Must be: "Erode"
        * **`weathering_amount`** *(Number, Format: double, Required)*: Maximum amount of "soil" in meters to weather off of the elevation before erosion (Actual amount calculated based on slope)
      * *(Object)*: Sets random points in an area to ocean if they are below sea level (Use FloodOcean to complete the process)
        * **`count`** *([Range_uint](#definitions/Range_uint), Required)*
        * **`task`** *(String, Required)*: Must be: "SeedOcean"
        * **`x_filter`** *([Range_double](#definitions/Range_double), Required)*
        * **`y_filter`** *([Range_double](#definitions/Range_double), Required)*
      * *(Object)*: Marks all tiles below sea level as ocean (SeedOcean and FloodOcean might be better)
        * **`task`** *(String, Required)*: Must be: "FillOcean"
      * *(Object)*: Finds tiles that are marked as ocean and marks all neighbors that are below sea level as ocean, until no neighbors below sea level can be found.
        * **`task`** *(String, Required)*: Must be: "FloodOcean"
      * *(Object)*: Sets tiles to ocean by sampling data from a heightmap. If data in heightmap is not nodata, the tile becomes ocean.
        * **`source`** *(String, Required)*: The path to the heightmap containing the ocean data
        * **`task`** *(String, Required)*: Must be: "SampleOceanMasked"
      * *(Object)*: Sets tiles to ocean by sampling data from a heightmap. If value in heightmap is less than specified elevation, it becomes ocean.
        * **`elevation`** *(Number, Format: double, Required)*: The elevation to compare to
        * **`source`** *(String, Required)*: The path to the heightmap containing the ocean data
        * **`task`** *(String, Required)*: Must be: "SampleOceanBelow"
      * *(Object)*: Replaces elevations by sampling from a heightmap
        * **`source`** *(String, Required)*: The path to the heightmap containing the elevation data
        * **`task`** *(String, Required)*: Must be: "SampleElevation"
  * <a id="definitions/InvertAxes"></a>**`InvertAxes`** *(String)*: Must be one of: ["X","Y","Both"]
  * <a id="definitions/Range_double"></a>**`Range_double`** *(String)*: A string value representing a range of numbers. Pattern: `-?\d+(\.\d+)?(\.\.=?-?\d+(\.\d+)?)?`
  * <a id="definitions/Range_int8"></a>**`Range_int8`** *(String)*: A string value representing a range of numbers. Pattern: `-?\d+(\.\.=?-?\d+)?`
  * <a id="definitions/Range_uint"></a>**`Range_uint`** *(String)*: A string value representing a range of numbers. Pattern: `\d+(\.\.=?\d+)?`
  * <a id="definitions/StraitDirection"></a>**`StraitDirection`** *(String)*: Must be one of: ["Horizontal","Vertical"]
