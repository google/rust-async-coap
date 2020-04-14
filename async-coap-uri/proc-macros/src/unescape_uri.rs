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

use std::str;

use crate::error::{Error, UnescapeError};

/// Used for determining how long a given UTF8 sequence is.
/// Table values come from <https://tools.ietf.org/html/rfc3629>
static UTF8_CHAR_WIDTH: [u8; 256] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x1F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x3F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x5F
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, // 0x7F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0x9F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, // 0xBF
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
    2, // 0xDF
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // 0xEF
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 0xFF
];

trait CharExt {
    fn to_hex_digit(self) -> Option<u8>;
}

impl CharExt for char {
    fn to_hex_digit(self) -> Option<u8> {
        match self {
            '0'..='9' => Some(self as u8 - b'0'),
            'a'..='f' => Some(self as u8 - b'a' + 10),
            'A'..='F' => Some(self as u8 - b'A' + 10),
            _ => None,
        }
    }
}

/// This is a stripped-down version of `async-coap-uri::escape::UnescapeUri` that is only used for
/// verifying the percent encoding of a string.
pub fn verify_uri(input: &str) -> Result<(), Error> {
    let mut chars = input.char_indices();

    let mut utf8_buf = [0_u8; 4];
    let mut utf8_len = 0_usize;

    while let Some((i, c)) = chars.next() {
        // unescaped ascii control codes are not allowed
        if c.is_ascii_control() {
            return Err(Error::decode_error(i, UnescapeError::UnescapedAsciiControl));
        }

        match c {
            ' ' => {
                // unescaped spaces are not allowed
                return Err(Error::decode_error(i, UnescapeError::Space));
            }
            '%' => {
                let decoded = match (chars.next(), chars.next()) {
                    (Some((_, _)), None) => {
                        return Err(Error::decode_error(i, UnescapeError::MissingChar(1)));
                    }
                    (Some((_, a)), Some((_, b))) => {
                        match (a.to_hex_digit(), b.to_hex_digit()) {
                            // both a and b are valid valid hex digits
                            //
                            // largest single hex digit = 0xF (0b00001111)
                            // -> no bits discarded by shifting to the left.
                            (Some(a), Some(b)) => b | a << 4,
                            // a or b is an invalid hex digit:
                            _ => return Err(Error::decode_error(i, UnescapeError::InvalidEscape)),
                        }
                    }
                    _ => {
                        // two chars are missing
                        return Err(Error::decode_error(i, UnescapeError::MissingChar(2)));
                    }
                };

                if (decoded as char).is_ascii_control() {
                    // We don't allow escaped ascii control codes for security reasons.
                    return Err(Error::decode_error(i, UnescapeError::AsciiControl));
                } else if decoded & 0x80 == 0x80 {
                    // This is a UTF8 byte.
                    utf8_buf[utf8_len] = decoded;
                    utf8_len += 1;

                    if utf8_len >= UTF8_CHAR_WIDTH[utf8_buf[0] as usize] as usize {
                        if str::from_utf8(&utf8_buf[..utf8_len]).is_err() {
                            // Invalid Utf8
                            return Err(Error::decode_error(
                                i,
                                UnescapeError::InvalidUtf8 {
                                    len: utf8_len as u8 + 1,
                                },
                            ));
                        } else {
                            // reset buffer + len
                            utf8_buf = [0; 4];
                            utf8_len = 0;
                        }
                    }
                } else if utf8_len != 0 {
                    return Err(Error::decode_error(
                        i - (utf8_len * 2 + utf8_len),
                        UnescapeError::UnfinishedUtf8 {
                            len: (utf8_len * 2 + utf8_len) as u8,
                        },
                    ));
                }
            }
            _ => {
                if utf8_len != 0 {
                    return Err(Error::decode_error(
                        i - (utf8_len * 2 + utf8_len),
                        UnescapeError::UnfinishedUtf8 {
                            len: (utf8_len * 2 + utf8_len) as u8,
                        },
                    ));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_verify_uri_valid() {
        assert_eq!(verify_uri("g:a/b/c"), Ok(()));
        assert_eq!(verify_uri("g+z://a/b/c"), Ok(()));
        assert_eq!(verify_uri("//a/b/c"), Ok(()));
        assert_eq!(verify_uri("a/b/c"), Ok(()));
        assert_eq!(verify_uri("g$:a/b/c"), Ok(()));

        assert_eq!(verify_uri("/a/b/c"), Ok(()));
        assert_eq!(verify_uri("a/b/c"), Ok(()));
        assert_eq!(verify_uri("g%3Aa/b/c"), Ok(()));
        assert_eq!(verify_uri("./g:a/b/c"), Ok(()));
        assert_eq!(verify_uri("//a/b/c"), Ok(()));
        assert_eq!(verify_uri("/.//a/b/c"), Ok(()));

        assert_eq!(verify_uri(""), Ok(()));
        assert_eq!(verify_uri("%3A"), Ok(()));
    }

    #[test]
    fn test_verify_uri_invalid() {
        assert_eq!(
            verify_uri("a/\n/c"),
            Err(Error::decode_error(2, UnescapeError::UnescapedAsciiControl))
        );

        assert_eq!(
            verify_uri("a/ /c"),
            Err(Error::decode_error(2, UnescapeError::Space))
        );

        assert_eq!(
            verify_uri("a/%a"),
            Err(Error::decode_error(2, UnescapeError::MissingChar(1)))
        );

        assert_eq!(
            verify_uri("a/%"),
            Err(Error::decode_error(2, UnescapeError::MissingChar(2)))
        );

        assert_eq!(
            verify_uri("g%:a/b/c"),
            Err(Error::decode_error(1, UnescapeError::InvalidEscape))
        );

        assert_eq!(
            verify_uri("%a/b/c"),
            Err(Error::decode_error(0, UnescapeError::InvalidEscape))
        );

        assert_eq!(
            verify_uri("g:%00/b/c"),
            Err(Error::decode_error(2, UnescapeError::AsciiControl))
        );

        assert_eq!(
            verify_uri("%02/b/c"),
            Err(Error::decode_error(0, UnescapeError::AsciiControl))
        );

        assert_eq!(
            verify_uri("g:%aa%FF/b/c"),
            Err(Error::decode_error(
                2,
                UnescapeError::InvalidUtf8 { len: 2 }
            ))
        );

        assert_eq!(
            verify_uri("//%F0%90%40"),
            Err(Error::decode_error(
                2,
                UnescapeError::UnfinishedUtf8 { len: 6 }
            ))
        );

        assert_eq!(
            verify_uri("%F0%90%40"),
            Err(Error::decode_error(
                0,
                UnescapeError::UnfinishedUtf8 { len: 6 }
            ))
        );

        assert_eq!(
            verify_uri("%F0%90a"),
            Err(Error::decode_error(
                0,
                UnescapeError::UnfinishedUtf8 { len: 6 }
            ))
        );

        assert_eq!(
            verify_uri("//%F0%90a"),
            Err(Error::decode_error(
                2,
                UnescapeError::UnfinishedUtf8 { len: 6 }
            ))
        );
    }
}
