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

/// Transparent conversions from [`core::fmt::Error`] to [`ResolveError`].
impl From<core::fmt::Error> for ResolveError {
    fn from(_: core::fmt::Error) -> Self {
        ResolveError::WriteFailure
    }
}

/// URI parse error type.
///
/// This type indicates the details of an error that occurs while parsing a URI.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ParseError {
    desc: &'static str,
    span: Option<Range<usize>>,
}

impl ParseError {
    /// Constructor for URI parse errors.
    pub fn new(desc: &'static str, span: Option<Range<usize>>) -> ParseError {
        ParseError { desc, span }
    }

    /// The location in the input string of the error. Optional.
    pub fn span(&self) -> Option<Range<usize>> {
        self.span.clone()
    }

    /// A debugging description of the error.
    pub fn desc(&self) -> &'static str {
        self.desc
    }
}
