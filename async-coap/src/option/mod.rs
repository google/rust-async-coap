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

//! Types related to interpreting and handling CoAP options.
//!

use super::message::codec::*;
use super::*;

/// The maximum size of a CoAP option allowed by this library.
pub const MAX_OPTION_VALUE_SIZE: usize = 1034;

mod num;
pub use super::option::num::*;

mod insert;
pub use insert::*;

mod iter;
pub use iter::*;

mod key;
pub use key::*;

mod value;
pub use value::*;

#[cfg(test)]
mod encoder;
