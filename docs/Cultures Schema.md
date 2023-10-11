# Array_of_CultureSetItemSource

## Items

  - **Items**:
    - : Refer to *[#/definitions/CultureSetItemSource](#definitions/CultureSetItemSource)*
## Definitions
  - **`CultureSetItemSource`***(Object)*
    - **`count`***(Integer | Null, format: uint)*: Minimum: `0`
    - **`name`***(String | Null)*
    - **`namer`***(String | Null)*
    - **`preferences`**
      - **Any of**
        - : Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
        - *(Null)*
    - **`probability`***(Number | Null, format: double)*
  - **`TilePreference`**
    - **One of**
      - *(String)*: Must be on of: ["Habitability","ShoreDistance","Elevation","NormalizedHabitability"]
      - *(Object)*: Can not contain additional properties.
        - **`Temperature`***(Number, format: double, required)*
      - *(Object)*: Can not contain additional properties.
        - **`Biomes`***(Array, required)*: Length must be equal to 2
          - **Items**:
            - *(Array)*
              - **Items**:
                - *(String)*
            - *(Number, format: double)*
      - *(Object)*: Can not contain additional properties.
        - **`OceanCoast`***(Number, format: double, required)*
      - *(Object)*: Can not contain additional properties.
        - **`Negate`**: Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
      - *(Object)*: Can not contain additional properties.
        - **`Multiply`***(Array, required)*
          - **Items**:
            - : Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
      - *(Object)*: Can not contain additional properties.
        - **`Divide`***(Array, required)*
          - **Items**:
            - : Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
      - *(Object)*: Can not contain additional properties.
        - **`Add`***(Array, required)*
          - **Items**:
            - : Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
      - *(Object)*: Can not contain additional properties.
        - **`Pow`***(Array, required)*: Length must be equal to 2
          - **Items**:
            - : Refer to *[#/definitions/TilePreference](#definitions/TilePreference)*
            - *(Number, format: double)*
