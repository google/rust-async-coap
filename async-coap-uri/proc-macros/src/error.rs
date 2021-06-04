use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::Lit;

#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    span: Option<Span>,
}

#[derive(Debug, Clone)]
enum ErrorKind {
    SynError(syn::Error),
    UnexpectedLit {
        found: &'static str,
        expected: &'static str,
    },
    DecodeError {
        index: usize,
        kind: UnescapeError,
    },
    MalformedStructure,
    MalformedScheme,
    Degenerate,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

// This manual implementation is required, because syn::Error does not implement PartialEq
impl PartialEq for ErrorKind {
    fn eq(&self, other: &Self) -> bool {
        match (&self, other) {
            (Self::SynError(e1), Self::SynError(e2)) => format!("{:?}", e1) == format!("{:?}", e2),
            (
                Self::UnexpectedLit {
                    found: f1,
                    expected: e1,
                },
                Self::UnexpectedLit {
                    found: f2,
                    expected: e2,
                },
            ) => f1 == f2 && e1 == e2,
            (
                Self::DecodeError {
                    index: i1,
                    kind: k1,
                },
                Self::DecodeError {
                    index: i2,
                    kind: k2,
                },
            ) => i1 == i2 && k1 == k2,
            (Self::Degenerate, Self::Degenerate)
            | (Self::MalformedStructure, Self::MalformedStructure)
            | (Self::MalformedScheme, Self::MalformedScheme) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnescapeError {
    /// found unescaped ascii control char
    UnescapedAsciiControl,
    /// found an unescaped space (' ')
    Space,
    /// there is no char after the `%`
    MissingChar(u8),
    /// the char following a `%` must be a valid ascii hex digit
    InvalidEscape,
    /// ascii control chars are forbidden for security reasons.
    AsciiControl,
    InvalidUtf8 {
        len: u8,
    },
    UnfinishedUtf8 {
        len: u8,
    },
}

impl fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnescapedAsciiControl => write!(f, "unescaped ascii control character"),
            Self::Space => write!(f, "unescaped space"),
            Self::MissingChar(n) => write!(f, "missing {} char after '%'", n),
            Self::InvalidEscape => write!(f, "the 2 char after '%' must be valid hex character"),
            Self::AsciiControl => {
                write!(f, "ascii control chars are forbidden for security reasons")
            }
            Self::InvalidUtf8 { .. } => write!(f, "invalid utf8"),
            Self::UnfinishedUtf8 { .. } => write!(f, "unfinished utf8"),
        }
    }
}

impl Error {
    #[must_use]
    const fn new(kind: ErrorKind) -> Self {
        Self { kind, span: None }
    }

    #[must_use]
    pub fn syn(value: syn::Error) -> Self {
        let span = value.span();

        Self::new(ErrorKind::SynError(value)).with_span(&span)
    }

    #[must_use]
    pub fn unexpected_lit(lit: &syn::Lit, expected: &'static str) -> Self {
        let found = {
            match lit {
                Lit::Str(_) => "string",
                Lit::ByteStr(_) => "byte string",
                Lit::Byte(_) => "byte",
                Lit::Char(_) => "char",
                Lit::Int(_) => "int",
                Lit::Float(_) => "float",
                Lit::Bool(_) => "bool",
                Lit::Verbatim(_) => "verbatim",
            }
        };

        Self::new(ErrorKind::UnexpectedLit { found, expected }).with_span(lit)
    }

    #[must_use]
    pub const fn decode_error(index: usize, kind: UnescapeError) -> Self {
        Self::new(ErrorKind::DecodeError { index, kind })
    }

    #[must_use]
    pub const fn malformed_structure() -> Self {
        Self::new(ErrorKind::MalformedStructure)
    }

    #[must_use]
    pub const fn malformed_scheme() -> Self {
        Self::new(ErrorKind::MalformedScheme)
    }

    #[must_use]
    pub const fn degenerate() -> Self {
        Self::new(ErrorKind::Degenerate)
    }
}

impl Error {
    #[must_use]
    pub fn with_span<T: Spanned>(mut self, node: &T) -> Self {
        if self.span.is_none() {
            self.span = Some(node.span());
        }

        self
    }

    #[must_use]
    pub fn into_token_stream(self) -> TokenStream {
        TokenStream::from(self.into_syn_error().to_compile_error())
    }

    #[must_use]
    pub fn span(&self) -> Span {
        self.span.unwrap_or_else(Span::call_site)
    }

    #[must_use]
    fn into_syn_error(self) -> syn::Error {
        syn::Error::new(self.span(), self)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::SynError(value) => value.fmt(f),
            Self::UnexpectedLit { found, expected } => {
                write!(f, "expected {} literal, found {}", expected, found)
            }
            Self::DecodeError { kind, .. } => write!(f, "{}", kind),
            Self::MalformedStructure => f.write_str("unrecognized uri structure"),
            Self::MalformedScheme => f.write_str("malformed uri scheme"),
            Self::Degenerate => f.write_str("relative reference could be confused with a uri"),
        }
    }
}

impl Into<syn::Error> for Error {
    fn into(self) -> syn::Error {
        self.into_syn_error()
    }
}

impl From<syn::Error> for Error {
    fn from(value: syn::Error) -> Self {
        Self::syn(value)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}
