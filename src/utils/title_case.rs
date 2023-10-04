use std::fmt;

pub(crate) struct AsTitleCase<StringType: AsRef<str>>(StringType);

impl<T: AsRef<str>> fmt::Display for AsTitleCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let source: &str = self.0.as_ref();
        
        let mut first = true;
        for word in source.split(' ') {
            if first {
                first = false;
            } else {
                write!(f," ")?;
            }
            let mut chars = word.chars();
            if let Some(first_char) = chars.next() {
                write!(f,"{}",first_char.to_uppercase())?;
                for char in chars {
                    write!(f,"{}",char.to_lowercase())?
                }
            }

        }

        Ok(())
    }
}    

pub(crate) trait ToTitleCase: ToOwned {
    /// Convert this type to title case.
    fn to_title_case(&self) -> Self::Owned;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        AsTitleCase(self).to_string()
    }
}
