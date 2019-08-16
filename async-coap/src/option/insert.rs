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

use super::*;
use core::convert::Into;

/// Trait for types that allow you to insert CoAP options into them.
pub trait OptionInsert {
    /// Inserts an option into the message with the given bytes as the value.
    /// Calling this method with out-of-order keys will incur a significant performance penalty.
    fn insert_option_with_bytes(&mut self, key: OptionNumber, value: &[u8]) -> Result<(), Error>;

    /// Inserts an option into the message with no value.
    /// Calling this method with out-of-order keys will incur a significant performance penalty.
    fn insert_option_empty(&mut self, key: OptionNumber) -> Result<(), Error> {
        self.insert_option_with_bytes(key, &[])
    }

    /// Inserts an option into the message with a string value.
    /// Calling this method with out-of-order keys will incur a significant performance penalty.
    fn insert_option_with_str(&mut self, key: OptionNumber, value: &str) -> Result<(), Error> {
        self.insert_option_with_bytes(key, value.as_bytes())
    }

    /// Inserts an option into the message with an integer value.
    /// Calling this method with out-of-order keys will incur a significant performance penalty.
    fn insert_option_with_u32(&mut self, key: OptionNumber, value: u32) -> Result<(), Error> {
        self.insert_option_with_bytes(key, encode_u32(value, &mut [0; 4]))
    }
}

/// Extension class for additional helper methods for `OptionInsertExt`.
pub trait OptionInsertExt {
    /// Inserts an option into the message with a value of the appropriate type.
    /// Calling this method with out-of-order keys will incur a significant performance penalty.
    fn insert_option<'a, T>(&mut self, key: OptionKey<T>, value: T) -> Result<(), Error>
    where
        T: Into<OptionValue<'a>>;
}

impl<O> OptionInsertExt for O
where
    O: OptionInsert + ?Sized,
{
    fn insert_option<'a, T>(&mut self, key: OptionKey<T>, value: T) -> Result<(), Error>
    where
        T: Into<OptionValue<'a>>,
    {
        match value.into() {
            OptionValue::Integer(x) => self.insert_option_with_u32(key.0, x),
            OptionValue::Bytes(x) => self.insert_option_with_bytes(key.0, x),
            OptionValue::ETag(x) => self.insert_option_with_bytes(key.0, x.as_bytes()),
        }
    }
}
