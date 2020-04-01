// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

use std::fmt;
use std::ops::Range;

/// Error type for resolving a target URI against a base URI.
///
/// Emitted by [`AnyUriRef::write_resolved`], [`AnyUriRef::resolved`],
/// and a few others.
///
/// [`AnyUriRef::write_resolved`]: async-coap-uri::AnyUriRef::write_resolved
/// [`AnyUriRef::resolved`]: async-coap-uri::AnyUriRef::resolved
#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum ResolveError {
    /// The URI-reference being given as a base cannot be used as a base for the given
    /// target URI-reference.
    CannotBeABase,

    /// Unable to write to the given [`core::fmt::Write`] instance.
    WriteFailure,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::CannotBeABase => write!(
                f,
                "given uri-ref cannot be used as a base for the target uri-ref"
            ),
            Self::WriteFailure => write!(f, "unable to write to the given `fmt::Write` instance"),
        }
    }
}

/// Transparent conversions from [`core::fmt::Error`] to [`ResolveError`].
impl From<::core::fmt::Error> for ResolveError {
    fn from(_: ::core::fmt::Error) -> Self {
        ResolveError::WriteFailure
    }
}

/// URI parse error type.
///
/// This type indicates the details of an error that occurs while parsing a URI.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ParseError {
    desc: ParseErrorKind,
    span: Option<Range<usize>>,
}

impl ParseError {
    /// Constructor for URI parse errors.
    pub fn new(desc: &'static str, span: Option<Range<usize>>) -> ParseError {
        ParseError {
            desc: desc.into(),
            span,
        }
    }

    /// The location in the input string of the error. Optional.
    pub fn span(&self) -> Option<Range<usize>> {
        self.span.clone()
    }

    /// A debugging description of the error.
    pub fn desc(&self) -> &'static str {
        self.desc.as_str()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.desc)
    }
}

impl From<crate::escape::DecodingError> for ParseError {
    fn from(error: crate::escape::DecodingError) -> Self {
        Self {
            span: Some(error.index..error.index + 1),
            desc: ParseErrorKind::EncodingError(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ParseErrorKind {
    /// Bad percent encoding or illegal characters
    EncodingError(crate::escape::DecodingError),
    /// Missing scheme or authority
    MissingSchemeOrAuthority,
    /// Cannot find URI components
    MissingUriComponents,
    /// Invalid URI scheme
    InvalidUriScheme,
    /// Not a URI
    InvalidUri,
    #[allow(dead_code)]
    Custom { desc: &'static str },
}

impl From<&'static str> for ParseErrorKind {
    fn from(desc: &'static str) -> Self {
        match desc {
            "Missing scheme or authority" => Self::MissingSchemeOrAuthority,
            "Cannot find URI components" => Self::MissingUriComponents,
            "Invalid URI scheme" => Self::InvalidUriScheme,
            "Not a URI" => Self::InvalidUri,
            _ => Self::Custom { desc },
        }
    }
}

impl ParseErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EncodingError(_) => "Bad percent encoding or illegal characters",
            Self::MissingSchemeOrAuthority => "Missing scheme or authority",
            Self::MissingUriComponents => "Cannot find URI components",
            Self::InvalidUriScheme => "Invalid URI scheme",
            Self::InvalidUri => "Not a URI",
            Self::Custom { desc } => desc,
        }
    }
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::EncodingError(e) = &self {
            write!(f, "{}", e)
        } else {
            write!(f, "{}", self.as_str())
        }
    }
}
