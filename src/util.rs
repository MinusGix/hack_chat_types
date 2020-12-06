use std::{convert::TryFrom, num::ParseIntError};

#[cfg(feature = "json_parsing")]
use json::JsonValue;

/// Utility type where you have a val, not have a val, or be unknown as to which it is.
/// Primarily for trips/hashes
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MaybeExist<T> {
    /// We have the value
    Has(T),
    /// We don't have a value, but we don't know whether they have it
    // or if they don't
    Unknown,
    /// They don't have it.
    Not,
}
impl<T> MaybeExist<T> {
    /// Converts an `Option<T>` into this type.
    /// `Some(T)` -> `Has(T)`
    /// `None` -> `Unknown`
    pub fn from_option_unknown(opt: Option<T>) -> MaybeExist<T> {
        match opt {
            Some(v) => MaybeExist::Has(v),
            None => MaybeExist::Unknown,
        }
    }

    pub fn as_ref(&self) -> MaybeExist<&T> {
        match self {
            MaybeExist::Has(v) => MaybeExist::Has(v),
            MaybeExist::Unknown => MaybeExist::Unknown,
            MaybeExist::Not => MaybeExist::Not,
        }
    }

    pub fn as_mut(&mut self) -> MaybeExist<&mut T> {
        match self {
            MaybeExist::Has(v) => MaybeExist::Has(v),
            MaybeExist::Unknown => MaybeExist::Unknown,
            MaybeExist::Not => MaybeExist::Not,
        }
    }

    pub fn expect(self, msg: &str) -> T {
        match self {
            MaybeExist::Has(v) => v,
            _ => panic!("{}", msg),
        }
    }

    pub fn as_unknown(&mut self) {
        *self = MaybeExist::Unknown;
    }

    pub fn as_not(&mut self) {
        *self = MaybeExist::Not;
    }

    pub fn map<U, F>(self, f: F) -> MaybeExist<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            MaybeExist::Has(v) => MaybeExist::Has(f(v)),
            MaybeExist::Unknown => MaybeExist::Unknown,
            MaybeExist::Not => MaybeExist::Not,
        }
    }

    pub fn and_then<U, F>(self, f: F) -> MaybeExist<U>
    where
        F: FnOnce(T) -> MaybeExist<U>,
    {
        match self {
            MaybeExist::Has(v) => f(v),
            MaybeExist::Unknown => MaybeExist::Unknown,
            MaybeExist::Not => MaybeExist::Not,
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            MaybeExist::Has(v) => v,
            MaybeExist::Unknown | MaybeExist::Not => default,
        }
    }
}

impl<T> Into<Option<T>> for MaybeExist<T> {
    fn into(self) -> Option<T> {
        match self {
            MaybeExist::Has(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ColorParseError {
    UnexpectedEOF,
    TooManyCharacters,
    ParseError(ParseIntError),
}
impl From<ParseIntError> for ColorParseError {
    fn from(err: ParseIntError) -> Self {
        ColorParseError::ParseError(err)
    }
}
/// RGB color.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl TryFrom<&str> for Color {
    type Error = ColorParseError;
    // TODO: handle if the values are unicode and the slicing partway through them is incorrect.
    fn try_from(text: &str) -> Result<Self, Self::Error> {
        use std::cmp::Ordering;
        // This shouldn't appear in the string, but we might as well handle it.
        let text = text.trim_start_matches('#');

        // Compare the length to see if its valid for parsing.
        let len = text.len();
        match len.cmp(&6) {
            // Too many characters.
            // TODO: should this support RGBA?
            Ordering::Greater => Err(ColorParseError::UnexpectedEOF),
            // Single byte RGB.
            Ordering::Less if len == 3 => {
                let red = u8::from_str_radix(&text[0..1], 16)?;
                let green = u8::from_str_radix(&text[1..2], 16)?;
                let blue = u8::from_str_radix(&text[2..3], 16)?;

                Ok(Color {
                    r: red,
                    g: green,
                    b: blue,
                })
            }
            // Not enough characters to consider.
            Ordering::Less => Err(ColorParseError::TooManyCharacters),
            // Two byte RGB
            _ => {
                // Sanity: these indices are
                let red = u8::from_str_radix(&text[0..2], 16)?;
                let green = u8::from_str_radix(&text[2..4], 16)?;
                let blue = u8::from_str_radix(&text[4..6], 16)?;

                Ok(Color {
                    r: red,
                    g: green,
                    b: blue,
                })
            }
        }
    }
}

/// Convert the thing (usually a command) into json.
#[cfg(feature = "json_parsing")]
pub trait IntoJson {
    /// The server format only applies when deciding how to format the data inside
    /// If you're using a command that's only for a specific format, then it will still be created.
    fn into_json(self, server_api: crate::ServerApi) -> JsonValue;
}
/// Mark a command, and the name of its CMD property.
pub trait Command {
    const CMD: &'static str;
}
/// Marker trait for commands sent from the client
pub trait ClientCommand: Command {}
/// Marker trait for commands sent by the server
pub trait ServerCommand: Command {}

#[cfg(feature = "json_parsing")]
#[derive(Debug, Clone, PartialEq)]
pub enum FromJsonError {
    InvalidStructure,
    InvalidField(&'static str),
    InvalidCommandField(&'static str),
}
/// For extracting a command from the json sent by the server.
#[cfg(feature = "json_parsing")]
pub trait FromJson: Sized {
    fn from_json(json: JsonValue, server_api: crate::ServerApi) -> Result<Self, FromJsonError>;
}

/// Utility function for converting to an array, as the json lib does not supply it
#[cfg(feature = "json_parsing")]
pub fn as_array(value: JsonValue) -> Option<Vec<JsonValue>> {
    match value {
        JsonValue::Array(arr) => Some(arr),
        _ => None,
    }
}
/// Utility function for converting to an object, as the json lib does not supply it
#[cfg(feature = "json_parsing")]
pub fn as_object(value: JsonValue) -> Option<json::object::Object> {
    match value {
        JsonValue::Object(obj) => Some(obj),
        _ => None,
    }
}
