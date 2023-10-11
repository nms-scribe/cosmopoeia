# Array_of_NamerSource

## Items

  * **Items**:
    * *([NamerSource](#definitions/NamerSource))*
## Definitions
  * <a id="definitions/NamerSource"></a>**`NamerSource`** *(Object)*
    * **One of**
      * *(Object)*
        * **`duplicatable_letters`** *(Array, Required)*
          * **Items**:
            * *(String)*: Minimum Length: `1`, Maximum Length: `1`
        * **`method`** *(String, Required)*: Must be: "Markov"
        * **`seed_words`** *(Array, Required)*
          * **Items**:
            * *(String)*
      * *(Object | Array)*
        * **Items**:
          * *(String)*
        * **`method`** *(String, Required)*: Must be: "ListPicker"
    * **`name`** *(String, Required)*
    * **`state_name`** *(Array, Required)*
      * **Items**:
        * *([StateNameBehavior](#definitions/StateNameBehavior))*
    * **`state_suffix`** *([StateSuffixBehavior](#definitions/StateSuffixBehavior), Required)*
  * <a id="definitions/StateNameBehavior"></a>**`StateNameBehavior`**
    * **One of**
      * *(Object)*: Can not contain additional properties.
        * **`TrimSuffixes`** *(Array, Required)*
          * **Items**:
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`TrimSuffixesIfLonger`** *(Array, Required)*: Minimum Items: `2`, Maximum Items: `2`
          * **Items**:
            * *(Array)*
              * **Items**:
                * *(String)*
            * *(Integer, Format: uint)*: Minimum: `0`
      * *(Object)*: Can not contain additional properties.
        * **`ForceVowel`** *(String, Required)*
      * *(Object)*: Can not contain additional properties.
        * **`ForcePrefix`** *(String, Required)*
      * *(Object)*: Can not contain additional properties.
        * **`ForcePrefixByLetterClass`** *(Array, Required)*: Minimum Items: `2`, Maximum Items: `2`
          * **Items**:
            * *(String)*
            * *(String)*
  * <a id="definitions/StateSuffixBehavior"></a>**`StateSuffixBehavior`**
    * **One of**
      * *(String)*: Must be one of: ["NoSuffix","Default"]
      * *(Object)*: Can not contain additional properties.
        * **`Suffix`** *(String, Required)*
      * *(Object)*: Can not contain additional properties.
        * **`ProbableSuffix`** *(Array, Required)*: Minimum Items: `2`, Maximum Items: `2`
          * **Items**:
            * *(Number, Format: double)*
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`ProbableSuffixIfShorter`** *(Array, Required)*: Minimum Items: `3`, Maximum Items: `3`
          * **Items**:
            * *(Integer, Format: uint)*: Minimum: `0`
            * *(Number, Format: double)*
            * *(String)*
      * *(Object)*: Can not contain additional properties.
        * **`Choice`** *(Array, Required)*
          * **Items**:
            * *([StateSuffixBehavior](#definitions/StateSuffixBehavior))*
