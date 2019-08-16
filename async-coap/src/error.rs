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

use std::fmt::{Debug, Display, Formatter};

/// Type for errors encountered while sending or receiving CoAP requests and responses.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum Error {
    /// One or more of the supplied arguments are not valid for the given operation.
    InvalidArgument,

    /// There is not enough space in the given buffer to complete the operation.
    OutOfSpace,

    /// An error was encountered while attempting to parse the data.
    ParseFailure,

    /// Operation timed out waiting for a response.
    ResponseTimeout,

    /// The response was well-formed, but not appropriate for the given request.
    BadResponse,

    /// The [message code][async-coap::message::MsgCode] was not recognized by this
    /// version of rust-async-coap.
    UnknownMessageCode,

    /// A critical option present in the message was not supported.
    UnhandledCriticalOption,

    /// An I/O error occurred while performing this operation.
    IOError,

    /// This operation has been cancelled.
    Cancelled,

    /// Unable to look up the given host because it was not found.
    HostNotFound,

    /// Unable to look up the given host for an unspecified reason.
    HostLookupFailure,

    /// The response indicated that the given resource was not found.
    ResourceNotFound,

    /// The response indicated that the request was unauthorized.
    Unauthorized,

    /// The response indicated that the request was forbidden.
    Forbidden,

    /// The response indicated an unspecified client error.
    ClientRequestError,

    /// The response indicated an unspecified server error.
    ServerError,

    /// The transaction was reset.
    Reset,

    /// More than one instance of an option marked as non-repeatable was encountered.
    OptionNotRepeatable,

    /// The given URI scheme is not supported by the associated local endpoint.
    UnsupportedUriScheme,

    /// An unspecified error has occurred.
    Unspecified,
}

#[cfg(feature = "std")]
impl std::convert::From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::IOError
    }
}

impl std::convert::From<Error> for core::fmt::Error {
    fn from(_: Error) -> Self {
        core::fmt::Error
    }
}

impl From<std::fmt::Error> for crate::Error {
    fn from(_err: std::fmt::Error) -> Self {
        Error::OutOfSpace
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        <Self as Debug>::fmt(self, f)
    }
}

impl Default for Error {
    fn default() -> Self {
        Error::Unspecified
    }
}

impl Extend<Result<(), Error>> for Error {
    fn extend<T: IntoIterator<Item = Result<(), Error>>>(&mut self, iter: T) {
        if let Some(Err(err)) = iter.into_iter().next() {
            *self = err;
        }
    }
}
