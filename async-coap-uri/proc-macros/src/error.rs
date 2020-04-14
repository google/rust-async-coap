#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum Error {
    #[allow(unused)]
    EncodingError,
    MalformedStructure,
    MalformedScheme,
    Degenerate,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::EncodingError => f.write_str("Encoding Error"),
            Error::MalformedStructure => f.write_str("The structure of the URI is not recognized."),
            Error::MalformedScheme => f.write_str("The scheme of the URI is malformed."),
            Error::Degenerate => {
                f.write_str("This relative reference could be confused with a URI.")
            }
        }
    }
}
