use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, CoffError>;

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
