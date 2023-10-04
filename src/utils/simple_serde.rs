/*
    Yes, this is reinventing the wheel, however...

    1) It took me less time to write this (5-6 hours, 8 hours with editing), than it would to try to wrangle serde into something I could use.
    2) I wanted a serialization format that was text-based. I wanted simple enums to be output as bare identifiers (not strings), so that other applications could use their values as labels. I wanted to be able to output the Neighbor type as an untagged, I specifically designed it so the types were incompatible, so an untagged enum was possible.
    3) I didn't want json (serde_json), because all identifiers are strings.
    4) Rusty Object Notation (ron) was much closer to what I wanted, except that it could not handle the untagged enums for the Neighbor enum. Well, it could for serializing, but not deserializing. This is where a custom deserializer might have worked, but the serde API for these is so arcane it would have taken me at least 5-6 hours to figure that out before implementing it, possibly discovering that it was still impossible.
    5) YAML was right out, as it kept inserting linefeeds into my values. I didn't stick around long enough to decide if there were any other problems.

    Simple Serde is easy to use. You serialize by writing tokens and values out. You deserialize it by standard parsing techniques: expectingand matching tokens. You don't have to deal with creating visitors and implementing matchers.

    Deserialization is strictly typed. There is no deserialize_any. If you don't know what you're expecting, this isn't the right tool for you.

    To read and write types, you call 'read_from_str' or 'write_to_string' directly on the type. Buffers aren't yet supported, but who knows, maybe someday. You might be able to call 'Deserialize::read_from_str' if rust can figure out the type of the result.

    To make a type readable or writable, there are several options:
    * If it's an enum, use `impl_simple_serde_tagged_enum`.
    * If it's a tuple struct use `impl_simple_serde_tuple_struct`
    * If it's a keyed struct, use `impl_simple_serde_keyed_struct`
    * If it doesn't fit those, or you want to control serialization, implement Serialize and Deserialize. There's only one function on each that you must implement, and its straightforward. Then make sure you have tests to confirm that the value serialized will also be deserialized.

    Note that in all of the macro cases above, you almost have to repeat the entire structure of the type, because I'm too lazy to create a proc macro. Only tuple structs and tuple enum variants can get by with bare identifiers, but they still must match the count. However, you don't have to worry about whether you have them correct, because the compiler will warn you if you've got names or counts wrong.

    If you want to use a different format, such as json, you might be able to implement Serializer and Deserializer, and pass that to `read_from` and `write_to` methods on the objects. But more than likely you'll find yourself struggling, and decide that it's just better to use another library. That's why I don't use this for more file-based input data in this crate.
     
    */

use core::str::Chars;
use core::iter::Peekable;

use paste::paste;

use crate::errors::CommandError;

#[derive(Debug,Clone)]
pub enum Token {
    OpenBracket,
    CloseBracket,
    OpenParenthesis,
    CloseParenthesis,
    OpenBrace,
    CloseBrace,
    Colon,
    Comma,
    Whitespace,
    Integer(u64),
    SignedInteger(i64),
    Float(f64),
    String(String),
    Identifier(String)
}

pub(crate) struct Tokenizer<'string> {
    text: Peekable<Chars<'string>>,
}

impl Iterator for Tokenizer<'_> {

    type Item = Result<Token,CommandError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(char) = self.text.peek() {
            match char {
                '[' => {
                    _ = self.text.next();
                    Some(Ok(Token::OpenBracket))
                },
                ']' => {
                    _ = self.text.next();
                    Some(Ok(Token::CloseBracket))
                },
                '(' => {
                    _ = self.text.next();
                    Some(Ok(Token::OpenParenthesis))
                },
                ')' => {
                    _ = self.text.next();
                    Some(Ok(Token::CloseParenthesis))
                },
                '{' => {
                    _ = self.text.next();
                    Some(Ok(Token::OpenBrace))
                },
                '}' => {
                    _ = self.text.next();
                    Some(Ok(Token::CloseBrace))
                },
                ':' => {
                    _ = self.text.next();
                    Some(Ok(Token::Colon))
                },
                ',' => {
                    _ = self.text.next();
                    Some(Ok(Token::Comma))
                },
                ' ' => {
                    _ = self.text.next();
                    while let Some(' ') = self.text.peek() {
                        _ = self.text.next();
                    }
                    Some(Ok(Token::OpenBracket))
                },
                '-' | '+' | '0'..='9' => {
                    let char = *char;
                    let signed = matches!(char,'-' | '+');
                    let mut number = String::from(char);
                    _ = self.text.next();
                    while let Some(char @ '0'..='9') = self.text.peek() {
                        number.push(*char);
                        _ = self.text.next();
                    }

                    if let Some('.') = self.text.peek() {
                        number.push('.');
                        _ = self.text.next();
                        while let Some(char @ '0'..='9') = self.text.peek() {
                            number.push(*char);
                            _ = self.text.next();
                        }

                        match number.parse() {
                            Ok(value) => Some(Ok(Token::Float(value))),
                            Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                        }                    

                    } else if signed {
                        match number.parse() {
                            Ok(value) => Some(Ok(Token::SignedInteger(value))),
                            Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                        }                    
                    } else {
                        match number.parse() {
                            Ok(value) => Some(Ok(Token::Integer(value))),
                            Err(_) => Some(Err(CommandError::InvalidNumberInSerializedValue(number))),
                        }                    

                    }

                },
                '"' => {
                    let mut value = String::new();
                    _ = self.text.next();
                    let mut found_quote = false;
                    while let Some(char) = self.text.next() {
                        match char {
                            '"' => {
                                found_quote = true;
                                break
                            },
                            '\\' => if let Some(char) = self.text.next() {
                                value.push(char);
                            } else {
                                value.push('\\');
                                break;
                            },
                            c => value.push(c)
                        }
                    }

                    if found_quote {
                        Some(Ok(Token::String(value)))
                    } else {
                        Some(Err(CommandError::InvalidStringInSerializedValue(value)))
                    }
                },
                'A'..='Z' | 'a'..='z' => {
                    let mut value = String::from(*char);
                    _ = self.text.next();
                    while let Some(char @ 'A'..='Z' | char @ 'a'..='z' | char @ '_' | char @ '0'..='9') = self.text.peek() {
                        value.push(*char);
                        _ = self.text.next();
                    }
                    Some(Ok(Token::Identifier(value)))
                },
                _ => {
                    Some(Err(CommandError::InvalidCharacterInSerializedValue(*char)))
                }

            }

        } else {
            None
        }
        
    }

}

pub(crate) trait Deserializer {

    fn expect(&mut self, expected: &Token) -> Result<(),CommandError>;

    fn matches(&mut self, desired: &Token) -> Result<bool,CommandError>;

    fn expect_identifier(&mut self) -> Result<String,CommandError>;

    fn skip_whitespace(&mut self) -> Result<(),CommandError>;

    fn expect_float(&mut self) -> Result<f64,CommandError>;

    fn expect_integer(&mut self, size: u32) -> Result<u64,CommandError>;

    fn matches_integer(&mut self) -> Result<Option<u64>,CommandError>;

    fn expect_signed_integer(&mut self, size: u32) -> Result<i64,CommandError>;

    fn peek_token(&mut self) -> Result<Option<&Token>,CommandError>;

}

impl Deserializer for Peekable<Tokenizer<'_>> {

    fn expect(&mut self, expected: &Token) -> Result<(),CommandError>  {
        self.skip_whitespace()?;
        match self.next().transpose()? {
            Some(found) => match (expected,&found) {
                (Token::OpenBracket, Token::OpenBracket) |
                (Token::CloseBracket, Token::CloseBracket) |
                (Token::OpenParenthesis, Token::OpenParenthesis) |
                (Token::CloseParenthesis, Token::CloseParenthesis) |
                (Token::Comma, Token::Comma) |
                (Token::Whitespace, Token::Whitespace) => Ok(()),
                (Token::Integer(a), Token::Integer(b)) => if a == b {
                    Ok(())
                } else {
                    Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                },
                (Token::SignedInteger(a), Token::SignedInteger(b)) => if a == b {
                    Ok(())
                } else {
                    Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                },
                (Token::Float(a), Token::Float(b)) => if a == b {
                    Ok(())
                } else {
                    Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                },
                (Token::String(a), Token::String(b)) |
                (Token::Identifier(a), Token::Identifier(b)) => if a == b {
                    Ok(())
                } else {
                    Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
                },
                (_,_) => Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),Some(found.clone())))
            },
            None => Err(CommandError::ExpectedTokenInSerializedValue(expected.clone(),None))
        }
    }


    fn matches(&mut self, desired: &Token) -> Result<bool,CommandError> {
        self.skip_whitespace()?;
        let result = match self.peek() {
            Some(Ok(found)) => match (desired,found) {
                (Token::OpenBracket, Token::OpenBracket) |
                (Token::CloseBracket, Token::CloseBracket) |
                (Token::OpenParenthesis, Token::OpenParenthesis) |
                (Token::CloseParenthesis, Token::CloseParenthesis) |
                (Token::Comma, Token::Comma) |
                (Token::Whitespace, Token::Whitespace) => true,
                (Token::Integer(a), Token::Integer(b)) => if a == b {
                    true
                } else {
                    false
                },
                (Token::SignedInteger(a), Token::SignedInteger(b)) => if a == b {
                    true
                } else {
                    false
                },
                (Token::Float(a), Token::Float(b)) => if a == b {
                    true
                } else {
                    false 
                },
                (Token::String(a), Token::String(b)) |
                (Token::Identifier(a), Token::Identifier(b)) => if a == b {
                    true
                } else {
                    false
                },
                (_,_) => false
            },
            Some(Err(err)) => return Err(err.clone()),
            None => false
        };
        if result {
            _ = self.next().transpose()?;
        }
        Ok(result)
    }

    fn expect_identifier(&mut self) -> Result<String,CommandError> {
        self.skip_whitespace()?;
        match self.next().transpose()? {
            Some(Token::Identifier(value)) => Ok(value),
            Some(token) => Err(CommandError::ExpectedIdentifierInSerializedValue(Some(token))),
            None => Err(CommandError::ExpectedIdentifierInSerializedValue(None)),
        }
    }

    fn skip_whitespace(&mut self) -> Result<(),CommandError> {
        while let Some(Ok(Token::Whitespace)) = self.peek() {
            _ = self.next().transpose()?;
        }
        Ok(())
    }

    fn expect_float(&mut self) -> Result<f64,CommandError> {
        self.skip_whitespace()?;
        match self.next().transpose()? {
            Some(Token::Float(value)) => Ok(value),
            Some(Token::Integer(value)) => Ok(value as f64),
            Some(Token::SignedInteger(value)) => Ok(value as f64),
            Some(token) => Err(CommandError::ExpectedFloatInSerializedValue(Some(token))),
            None => Err(CommandError::ExpectedFloatInSerializedValue(None)),
        }
    }

    fn expect_integer(&mut self, size: u32) -> Result<u64,CommandError> {
        self.skip_whitespace()?;
        match self.next().transpose()? {
            Some(Token::Integer(value)) => Ok(value),
            Some(token) => Err(CommandError::ExpectedIntegerInSerializedValue(size,false,Some(token))),
            None => Err(CommandError::ExpectedIntegerInSerializedValue(size,false,None)),
        }
    }

    fn expect_signed_integer(&mut self, size: u32) -> Result<i64,CommandError> {
        self.skip_whitespace()?;
        match self.next().transpose()? {
            Some(Token::SignedInteger(value)) => Ok(value),
            Some(Token::Integer(value)) => Ok(value as i64),
            Some(token) => Err(CommandError::ExpectedIntegerInSerializedValue(size,true,Some(token))),
            None => Err(CommandError::ExpectedIntegerInSerializedValue(size,true,None)),
        }
    }

    fn matches_integer(&mut self) -> Result<Option<u64>,CommandError> {
        self.skip_whitespace()?;
        match self.peek() {
            Some(Ok(Token::Integer(value))) => {
                let value = *value;
                _ = self.next().transpose()?;
                Ok(Some(value))
            },
            Some(Ok(_)) => Ok(None),
            Some(Err(err)) => Err(err.clone()),
            None => Ok(None),
        }
    }

    fn peek_token(&mut self) -> Result<Option<&Token>,CommandError> {
        match self.peek() {
            Some(value) => match value {
                Ok(value) => Ok(Some(value)),
                Err(err) => Err(err.clone()),
            },
            None => Ok(None),
        }
    }

}

pub(crate) trait Serializer: Sized {

    fn write_token(&mut self, token: Token);

    fn serialize_value<Value: Serialize>(&mut self, value: Value) {
        value.write_value(self)
    }

}

impl Serializer for String {
    fn write_token(&mut self, token: Token) {
        match token {
            Token::OpenBracket => self.push('['),
            Token::CloseBracket => self.push(']'),
            Token::OpenParenthesis => self.push('('),
            Token::CloseParenthesis => self.push(')'),
            Token::OpenBrace => self.push('{'),
            Token::CloseBrace => self.push('}'),
            Token::Colon => self.push(':'),
            Token::Comma => self.push(','),
            Token::Whitespace => self.push(' '),
            Token::Float(number) => self.push_str(&number.to_string()),
            Token::Integer(number) => self.push_str(&number.to_string()),
            Token::SignedInteger(number) => self.push_str(&number.to_string()),
            Token::String(string) => {
                self.push('"');
                self.push_str(&string.replace('"', "\\\""));
                self.push('"');
            },
            Token::Identifier(identifier) => self.push_str(&identifier),
        }
    }
}

pub(crate) trait Serialize {

    fn write_value<Target: Serializer>(&self, serializer: &mut Target);

    fn write_to_string(&self) -> String {
        let mut string = String::new();
        self.write_value(&mut string);
        string
    }

}

impl<Borrowed: Serialize> Serialize for &Borrowed {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        (*self).write_value(serializer)
    }
}

pub(crate) trait Deserialize: Sized {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError>;

    fn read_from_str(string: &str) -> Result<Self,CommandError> {
        let tokenizer = Tokenizer {
            text: string.chars().peekable()
        };

        Deserialize::read_value(&mut tokenizer.peekable())        
    }
}

impl<ItemType: Serialize> Serialize for Vec<ItemType> {
    
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        serializer.write_token(Token::OpenBracket);
        let mut first = true;
        for item in self {
            if first {
                first = false;
            } else {
                serializer.write_token(Token::Comma);
            }
            item.write_value(serializer);
        }
        serializer.write_token(Token::CloseBracket)
    }
}

impl<ItemType: Deserialize> Deserialize for Vec<ItemType> {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        deserializer.expect(&Token::OpenBracket)?;
        let mut result = Vec::new();
        if !deserializer.matches(&Token::CloseBracket)? {
            result.push(Deserialize::read_value(deserializer)?);
            while deserializer.matches(&Token::Comma)? {
                result.push(Deserialize::read_value(deserializer)?);    
            }
            deserializer.expect(&Token::CloseBracket)?;
        }
        Ok(result)
    }

}

macro_rules! impl_simple_serde_tuple {
    ($($first_name: ident: $first_gen_param: ident $(, $name: ident: $gen_param: ident)* $(,)?)?) => {

        impl$(<$first_gen_param: Serialize $(,$gen_param: Serialize)*>)? Serialize for ($($first_gen_param, $($gen_param),*)?) {

            fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
                serializer.write_token(Token::OpenParenthesis);
                $(
                    let ($first_name, $($name,)*) = self;
                    $first_name.write_value(serializer);
                    $(
                        serializer.write_token(Token::Comma);
                        $name.write_value(serializer);
                    )*
                )?
                serializer.write_token(Token::CloseParenthesis)
            }
                    
        }
        
        impl$(<$first_gen_param: Deserialize $(,$gen_param: Deserialize)*>)? Deserialize for ($($first_gen_param, $($gen_param),*)?) {

            fn read_value<Source: Deserializer>(source: &mut Source) -> Result<Self,CommandError> {
                source.expect(&Token::OpenParenthesis)?;
                $(
                    let $first_name = Deserialize::read_value(source)?;
                    $(
                        source.expect(&Token::Comma)?;
                        let $name = Deserialize::read_value(source)?;
                    )*
                )?
                source.expect(&Token::CloseParenthesis)?;
                Ok(($($first_name,$($name,)*)?))
            }
    
        }            
    };
    ($($first_gen_param: ident $(, $gen_param: ident)* $(,)?)?) => {
        paste!{
            impl_simple_serde_tuple!($([<$first_gen_param:snake>]: $first_gen_param $(, [<$gen_param:snake>]: $gen_param)*)?);
        }
    }
}

impl_simple_serde_tuple!();

impl_simple_serde_tuple!(Item1);

impl_simple_serde_tuple!(Item1,Item2);

impl Serialize for f64 {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        serializer.write_token(Token::Float(*self))
    }
}

impl Deserialize for f64 {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        deserializer.expect_float()
    }
}

impl Serialize for u64 {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        serializer.write_token(Token::Integer(*self))
    }
}

impl Deserialize for u64 {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        deserializer.expect_integer(64)
    }
}

impl Serialize for usize {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        serializer.write_token(Token::Integer(*self as u64))
    }
}

impl Deserialize for usize {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        Ok(deserializer.expect_integer(usize::BITS)? as usize)
    }
}

impl Serialize for i32 {
    fn write_value<Target: Serializer>(&self, serializer: &mut Target) {
        serializer.write_token(Token::SignedInteger(*self as i64))
    }

}

impl Deserialize for i32 {

    fn read_value<Source: Deserializer>(deserializer: &mut Source) -> Result<Self,CommandError> {
        Ok(deserializer.expect_signed_integer(32)? as i32)
    }
}

#[macro_export]
macro_rules! impl_simple_serde_tagged_enum {

    ($enum: ty {$($variant: ident $(($($name: ident),*$(,)?))?),*$(,)?}) => {
        impl $crate::utils::simple_serde::Serialize for $enum {
    
            fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                match self {
                    $(
                        Self::$variant$(($($name,)*))? => {
                            serializer.write_token($crate::utils::simple_serde::Token::Identifier(stringify!($variant).to_owned()));
                            $(
                                // use tuple serialization to do it. Note that I need the comma even on the one-element to convert it into a tuple
                                ($( $name, )*).write_value(serializer)
                            )?
                        },
                    )*
                }
            }
        }

        impl $crate::utils::simple_serde::Deserialize for $enum {
        
            fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                let identifier = deserializer.expect_identifier()?;
                match identifier.as_str() {
                    $(
                        stringify!($variant) => {
                            // use tuple deserialization. Note that I need the comma even on the one-element to convert it into a tuple
                            $( let ($($name,)*) = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;)?
                            Ok(Self::$variant$(($($name,)*))?)
                        }
                    ),*
                    invalid => Err($crate::errors::CommandError::InvalidEnumValueInInSerializedValue(invalid.to_owned())),
                }
            }
        
        }
        
        
    };
}

#[macro_export]
macro_rules! impl_simple_serde_tuple_struct {

    ($struct: ty {$($name: ident),*$(,)?}) => {
        impl $crate::utils::simple_serde::Serialize for $struct {
    
            fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                let Self($($name,)*) = self;
                // use tuple serialization to handle it. Note that I need the comma even on the one-element to convert it into a tuple
                ($($name,)*).write_value(serializer);
            }
        }

        impl $crate::utils::simple_serde::Deserialize for $struct {
        
            fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                // use tuple deserialization. Note that I need the comma even on the one-element to convert it into a tuple
                let ($($name,)*) = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                Ok(Self($($name,)*))
            }
        
        }
        
        
    };
}

#[allow(unused_macros)] // This is just a hint at what you could do. I don't have a need for it right now though.
macro_rules! impl_simple_serde_keyed_struct {

    ($struct: ty {$first_name: ident $(,$name: ident)*$(,)?}) => {
        impl $crate::utils::simple_serde::Serialize for $struct {
    
            fn write_value<Target: $crate::utils::simple_serde::Serializer>(&self, serializer: &mut Target) {
                let Self{$first_name $(,$name)*} = self;
                serializer.write_token(Token::OpenBrace);
                $first_name.write_value(serializer);
                $(
                    serializer.write_token(Token::Comma);
                    $name.write_value(serializer);
                )*
                serializer.write_token(Token::CloseBrace);
            }
        }

        impl $crate::utils::simple_serde::Deserialize for $struct {
        
            fn read_value<Source: $crate::utils::simple_serde::Deserializer>(deserializer: &mut Source) -> Result<Self,$crate::errors::CommandError> {
                source.expect(Token::OpenBrace)?;
                let $first_name = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                $(
                    source.expect(Token::Comma)?;
                    let $name = $crate::utils::simple_serde::Deserialize::read_value(deserializer)?;
                )*
                Ok(Self{
                    $first_name,
                    $(,$name)*
                })
            }
        
        }
        
        
    };
}


#[cfg(test)]
mod test {

    use angular_units::Deg;

    use crate::utils::simple_serde::Serialize as SimpleSerialize;
    use crate::utils::simple_serde::Deserialize as SimpleDeserialize;    

    use crate::utils::edge::Edge;
    use crate::world_map::Neighbor; // and vec
    use crate::world_map::NeighborAndDirection; // and vec
    use crate::world_map::Grouping;
    use crate::world_map::RiverSegmentFrom;
    use crate::world_map::RiverSegmentTo;
    use crate::world_map::LakeType;
    use crate::world_map::BiomeCriteria;
    use crate::world_map::CultureType;


    fn test_serializing<Value: SimpleSerialize + SimpleDeserialize + PartialEq + core::fmt::Debug>(value: Value, text: &str) {
        let serialized = value.write_to_string();
        assert_eq!(serialized,text);
        let deserialized = Value::read_from_str(&serialized).unwrap();
        assert_eq!(value,deserialized)
    }


    #[test]
    fn test_serde_edge() {
        test_serializing(Edge::North, "North");
        test_serializing(Edge::Southwest, "Southwest");
    }

    #[test]
    fn test_serde_neighbor() {
        test_serializing(Neighbor::Tile(36), "36");
        test_serializing(Neighbor::CrossMap(42, Edge::East), "(42,East)");
        test_serializing(Neighbor::OffMap(Edge::West), "West");
    }

    #[test]
    fn test_serde_neighbor_vec() {
        test_serializing(vec![Neighbor::Tile(36), Neighbor::CrossMap(42, Edge::East), Neighbor::OffMap(Edge::West)], "[36,(42,East),West]");
        test_serializing::<Vec<Neighbor>>(vec![], "[]");
    }

    #[test]
    fn test_serde_neighbor_and_direction() {
        test_serializing(NeighborAndDirection(Neighbor::Tile(72),Deg(45.6)), "(72,45.6)")
    }

    #[test]
    fn test_serde_neighbor_and_direction_vec() {
        test_serializing(vec![NeighborAndDirection(Neighbor::Tile(72),Deg(45.6)),NeighborAndDirection(Neighbor::CrossMap(49,Edge::Southeast),Deg(0.1))], "[(72,45.6),((49,Southeast),0.1)]")
    }

    #[test]
    fn test_serde_grouping() {
        test_serializing(Grouping::LakeIsland, "LakeIsland")
    }

    #[test]
    fn test_serde_river_segment_from() {
        test_serializing(RiverSegmentFrom::Confluence, "Confluence")
    }

    #[test]
    fn test_serde_river_segment_to() {
        test_serializing(RiverSegmentTo::Mouth, "Mouth")
    }

    #[test]
    fn test_serde_lake_type() {
        test_serializing(LakeType::Fresh, "Fresh")
    }

    #[test]
    fn test_serde_biome_criteria() {
        test_serializing(BiomeCriteria::Glacier, "Glacier");
        test_serializing(BiomeCriteria::Matrix(vec![(23,24),(12,20),(13,4)]), "Matrix([(23,24),(12,20),(13,4)])")
    }

    #[test]
    fn test_serde_culture_type() {
        test_serializing(CultureType::Hunting, "Hunting")
    }


}
