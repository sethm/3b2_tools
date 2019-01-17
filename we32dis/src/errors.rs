use std::error;
use std::fmt;
use std::io;

pub type ReadResult<T> = std::result::Result<T, CoffError>;

#[derive(Debug, Clone)]
pub struct OffsetError;

impl fmt::Display for OffsetError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bad offset")
    }
}

impl error::Error for OffsetError {
    fn description(&self) -> &str {
        "bad offset"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}


#[derive(Debug, Clone)]
pub enum CoffError {
    BadFileHeader,
    BadOptionalHeader,
    BadSections,
    BadSymbols,
    BadStrings,
}

impl fmt::Display for CoffError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CoffError::BadFileHeader => write!(f, "bad file header"),
            CoffError::BadOptionalHeader => write!(f, "bad optional header"),
            CoffError::BadSections => write!(f, "bad section headers"),
            CoffError::BadSymbols => write!(f, "bad symbols table"),
            CoffError::BadStrings => write!(f, "bad strings table"),
        }
    }
}

impl error::Error for CoffError {
    fn description(&self) -> &str {
        match *self {
            CoffError::BadFileHeader => "bad file header",
            CoffError::BadOptionalHeader => "bad file header",
            CoffError::BadSections => "bad section headers",
            CoffError::BadSymbols => "bad symbols table",
            CoffError::BadStrings => "bad strings table",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

///
/// Error while decoding instruction stream
///
#[derive(Debug)]
pub enum DecodeError {
    IoError(io::Error),
    Parse,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeError::IoError(error) => write!(f, "io error on decode: {:?}", error),
            DecodeError::Parse => write!(f, "parse error on decode"),
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match self {
            DecodeError::IoError(_) => "io error on decode",
            DecodeError::Parse => "parse error on decode",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            DecodeError::IoError(error) => Some(error),
            DecodeError::Parse => None,
        }
    }
}

impl From<io::Error> for DecodeError {
    fn from(error: io::Error) -> Self {
        DecodeError::IoError(error)
    }
}