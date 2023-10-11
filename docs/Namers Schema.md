# Array_of_NamerSource

## Items

  * **Items**:
    * See *[NamerSource](#definitions/NamerSource)*
## Definitions
  * <a id="definitions/NamerSource"></a>**`NamerSource`** *(Object)*
    * **One of**
      * *(Object)*
        * **`duplicatable_letters`** *(Array, required)*
          * **Items**:
            * *(String)*: Length must be equal to 1
        * **`method`** *(String, required)*: Must be: "Markov"
        * **`seed_words`** *(Array, required)*
          * **Items**:
            * *(String)*
      * *(Object | Array)*
        * **Items**:
          * *(String)*
        * **`method`** *(String, required)*: Must be: "ListPicker"
    * **`name`** *(String, required)*
    * **`state_name`** *(Array, required)*
      * **Items**:
        * See *[StateNameBehavior](#definitions/StateNameBehavior)*
    * **`state_suffix`** : See *[StateSuffixBehavior](#definitions/StateSuffixBehavior)*
  * <a id="definitions/StateNameBehavior"></a>**`StateNameBehavior`** 
    * **One of**
      * *(Object)*: Can not contain additional properties.
        * **`TrimSuffixes`** *(Array, required)*
          * **Items**:
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`TrimSuffixesIfLonger`** *(Array, required)*: Length must be equal to 2
          * **Items**:
            * *(Array)*
              * **Items**:
                * *(String)*
            * *(Integer, format: uint)*: Minimum: `0`
      * *(Object)*: Can not contain additional properties.
        * **`ForceVowel`** *(String, required)*
      * *(Object)*: Can not contain additional properties.
        * **`ForcePrefix`** *(String, required)*
      * *(Object)*: Can not contain additional properties.
        * **`ForcePrefixByLetterClass`** *(Array, required)*: Length must be equal to 2
          * **Items**:
            * *(String)*
            * *(String)*
  * <a id="definitions/StateSuffixBehavior"></a>**`StateSuffixBehavior`** 
    * **One of**
      * *(String)*: Must be one of: ["NoSuffix","Default"]
      * *(Object)*: Can not contain additional properties.
        * **`Suffix`** *(String, required)*
      * *(Object)*: Can not contain additional properties.
        * **`ProbableSuffix`** *(Array, required)*: Length must be equal to 2
          * **Items**:
            * *(Number, format: double)*
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`ProbableSuffixIfShorter`** *(Array, required)*: Length must be equal to 3
          * **Items**:
            * *(Integer, format: uint)*: Minimum: `0`
            * *(Number, format: double)*
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`Choice`** *(Array, required)*
          * **Items**:
            * See *[StateSuffixBehavior](#definitions/StateSuffixBehavior)*
