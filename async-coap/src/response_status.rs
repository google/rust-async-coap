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

/// Successful return type from [send descriptor handler method](send_desc/trait.SendDesc.html#tymethod.handler)
/// that indicates what should happen next.
#[derive(Debug, Copy, Eq, PartialEq, Clone)]
pub enum ResponseStatus<T = ()> {
    /// Emit the given value.
    Done(T),

    /// Allocate a new Message ID, resend a new request, and wait for the associated response.
    ///
    /// This is used when handling block requests to fetch additional blocks, among other cases.
    SendNext,

    /// Wait for additional responses to the original request without sending new requests.
    ///
    /// This is used when handling multicast requests and observing.
    Continue,
}

impl<T> ResponseStatus<T> {
    /// If the response status is `Done(value)`, returns `Some(value)`, otherwise returns `None`.
    pub fn done(self) -> Option<T> {
        match self {
            ResponseStatus::Done(x) => Some(x),
            _ => None,
        }
    }

    /// Returns true if the response status is `Done(...)`, false otherwise.
    pub fn is_done(&self) -> bool {
        match *self {
            ResponseStatus::Done(_) => true,
            _ => false,
        }
    }

    /// Returns true if the response status is `SendNext`, false otherwise.
    pub fn is_send_next(&self) -> bool {
        match *self {
            ResponseStatus::SendNext => true,
            _ => false,
        }
    }

    /// Returns true if the response status is `Continue`, false otherwise.
    pub fn is_continue(&self) -> bool {
        match *self {
            ResponseStatus::Continue => true,
            _ => false,
        }
    }

    /// Converts the contained type to be a reference, so that `Done(T)` becomes `Done(&T)`.
    pub fn as_ref(&self) -> ResponseStatus<&T> {
        match *self {
            ResponseStatus::Done(ref x) => ResponseStatus::Done(x),
            ResponseStatus::SendNext => ResponseStatus::SendNext,
            ResponseStatus::Continue => ResponseStatus::Continue,
        }
    }

    /// Converts the contained type to be a mutable reference, so that `Done(T)` becomes
    /// `Done(&mut T)`.
    pub fn as_mut(&mut self) -> ResponseStatus<&mut T> {
        match *self {
            ResponseStatus::Done(ref mut x) => ResponseStatus::Done(x),
            ResponseStatus::SendNext => ResponseStatus::SendNext,
            ResponseStatus::Continue => ResponseStatus::Continue,
        }
    }
}
