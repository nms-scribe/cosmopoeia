# Array_of_CultureSetItemSource

## Items

  * **Items**:
    * *([CultureSetItemSource](#definitions/CultureSetItemSource))*
## Definitions
  * <a id="definitions/CultureSetItemSource"></a>**`CultureSetItemSource`** *(Object)*
    * **`count`** *(Integer | Null, Format: uint)*: Minimum: `0`
    * **`name`** *(String | Null)*
    * **`namer`** *(String | Null)*
    * **`preferences`**
      * **Any of**
        * *([TilePreference](#definitions/TilePreference))*
        * *(Null)*
    * **`probability`** *(Number | Null, Format: double)*
  * <a id="definitions/TilePreference"></a>**`TilePreference`**
    * **One of**
      * *(String)*: Must be one of: ["Habitability","ShoreDistance","Elevation","NormalizedHabitability"]
      * *(Object)*: Can not contain additional properties.
        * **`Temperature`** *(Number, Format: double, Required)*
      * *(Object)*: Can not contain additional properties.
        * **`Biomes`** *(Array, Required)*: Minimum Items: `2`, Maximum Items: `2`
          * **Items**:
            * *(Array)*
              * **Items**:
                * *(String)*
            * *(Number, Format: double)*
      * *(Object)*: Can not contain additional properties.
        * **`OceanCoast`** *(Number, Format: double, Required)*
      * *(Object)*: Can not contain additional properties.
        * **`Negate`** *([TilePreference](#definitions/TilePreference), Required)*
      * *(Object)*: Can not contain additional properties.
        * **`Multiply`** *(Array, Required)*
          * **Items**:
            * *([TilePreference](#definitions/TilePreference))*
      * *(Object)*: Can not contain additional properties.
        * **`Divide`** *(Array, Required)*
          * **Items**:
            * *([TilePreference](#definitions/TilePreference))*
      * *(Object)*: Can not contain additional properties.
        * **`Add`** *(Array, Required)*
          * **Items**:
            * *([TilePreference](#definitions/TilePreference))*
      * *(Object)*: Can not contain additional properties.
        * **`Pow`** *(Array, Required)*: Minimum Items: `2`, Maximum Items: `2`
          * **Items**:
            * *([TilePreference](#definitions/TilePreference))*
            * *(Number, Format: double)*
