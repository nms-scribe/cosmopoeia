# Array_of_CultureSetItemSource

## Items

  * **Items**:
    * : : See *[CultureSetItemSource](#definitions/CultureSetItemSource)*
## Definitions
  * <a id="definitions/CultureSetItemSource"></a>**`CultureSetItemSource`** *(Object)*: 
    * **`count`** *(Integer | Null, format: uint)*: : Minimum: `0`
    * **`name`** *(String | Null)*: 
    * **`namer`** *(String | Null)*: 
    * **`preferences`** : 
      * **Any of**
        * : : See *[TilePreference](#definitions/TilePreference)*
        * *(Null)*: 
    * **`probability`** *(Number | Null, format: double)*: 
  * <a id="definitions/TilePreference"></a>**`TilePreference`** : 
    * **One of**
      * *(String)*: : Must be one of: ["Habitability","ShoreDistance","Elevation","NormalizedHabitability"]
      * *(Object)*: : Can not contain additional properties.
        * **`Temperature`** *(Number, format: double, required)*: 
      * *(Object)*: : Can not contain additional properties.
        * **`Biomes`** *(Array, required)*: : Length must be equal to 2
          * **Items**:
            * *(Array)*: 
              * **Items**:
                * *(String)*: 
            * *(Number, format: double)*: 
      * *(Object)*: : Can not contain additional properties.
        * **`OceanCoast`** *(Number, format: double, required)*: 
      * *(Object)*: : Can not contain additional properties.
        * **`Negate`** : : See *[TilePreference](#definitions/TilePreference)*
      * *(Object)*: : Can not contain additional properties.
        * **`Multiply`** *(Array, required)*: 
          * **Items**:
            * : : See *[TilePreference](#definitions/TilePreference)*
      * *(Object)*: : Can not contain additional properties.
        * **`Divide`** *(Array, required)*: 
          * **Items**:
            * : : See *[TilePreference](#definitions/TilePreference)*
      * *(Object)*: : Can not contain additional properties.
        * **`Add`** *(Array, required)*: 
          * **Items**:
            * : : See *[TilePreference](#definitions/TilePreference)*
      * *(Object)*: : Can not contain additional properties.
        * **`Pow`** *(Array, required)*: : Length must be equal to 2
          * **Items**:
            * : : See *[TilePreference](#definitions/TilePreference)*
            * *(Number, format: double)*: 
