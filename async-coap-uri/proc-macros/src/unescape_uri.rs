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

use std::char::REPLACEMENT_CHARACTER;
use std::str::{from_utf8, from_utf8_unchecked};

/// Used for determining how long a given UTF8 sequence is.
/// Table values come from https://tools.ietf.org/html/rfc3629
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

/// This is a stripped-down version of `async-coap-uri::escape::UnescapeUri` that is only used for
/// verifying the percent encoding of a string.
#[derive(Debug, Clone)]
pub struct UnescapeUri<'a> {
    pub(super) iter: std::str::Chars<'a>,
    pub(super) iter_index: usize,
    pub(super) next_c: Option<(char, Option<char>)>,
    pub(super) had_error: bool,
    pub(super) skip_slashes: bool,
}

impl<'a> UnescapeUri<'a> {
    pub fn new(string: &str) -> UnescapeUri<'_> {
        UnescapeUri {
            iter: string.chars(),
            iter_index: 0,
            next_c: None,
            had_error: false,
            skip_slashes: false,
        }
    }

    pub fn first_error(&self) -> Option<usize> {
        let mut iter = self.clone();
        let begin = iter.index();
        while let Some(_) = iter.next() {
            if iter.had_error {
                break;
            }
        }
        if iter.had_error {
            let end = iter.index();
            return Some(end - begin);
        }
        None
    }

    /// Indicates the number of characters that have been read by this iterator
    /// from the source string.
    pub fn index(&self) -> usize {
        self.iter_index
    }
}

impl<'a> Iterator for UnescapeUri<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        let mut utf8_buf = [0u8; 4];
        let mut utf8_len = 0usize;

        if let Some((c, next_c)) = self.next_c.take() {
            if let Some(x) = next_c {
                self.next_c = Some((x, None));
            }
            return Some(c);
        }

        while let Some(c) = self.iter.next() {
            self.iter_index += 1;

            // Immediately drop all unescaped ascii control codes
            if c.is_ascii_control() {
                self.had_error = true;
                continue;
            }
            match c {
                ' ' => {
                    // Mark unescaped spaces as an error, but pass them along.
                    self.had_error = true;
                    if utf8_len == 0 {
                        return Some(c);
                    } else {
                        self.next_c = Some((c, None));
                        return Some(REPLACEMENT_CHARACTER);
                    }
                }

                '%' => {
                    self.iter_index += 1;
                    let msn = match self.iter.next() {
                        Some(c) if c.is_ascii_hexdigit() => c as u8,
                        Some(c) => {
                            self.had_error = true;
                            if utf8_len == 0 {
                                self.next_c = Some((c, None));
                                return Some('%');
                            } else {
                                self.next_c = Some((c, None));
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                        None => {
                            self.iter_index -= 1;
                            self.had_error = true;
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    };

                    self.iter_index += 1;
                    let lsn = match self.iter.next() {
                        Some(c) if c.is_ascii_hexdigit() => c as u8,
                        Some(c) => {
                            self.had_error = true;
                            if utf8_len == 0 {
                                self.next_c = Some((msn as char, Some(c)));
                                return Some('%');
                            } else {
                                self.next_c = Some((c, None));
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                        None => {
                            self.iter_index -= 1;
                            self.had_error = true;
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    };

                    let buf = [msn, lsn];

                    // SAFETY: This is safe because we just verified that
                    // these two characters were ASCII hex digits.
                    let buf_str = unsafe { from_utf8_unchecked(&buf) };

                    let decoded = u8::from_str_radix(&buf_str, 16).unwrap();

                    if self.skip_slashes && decoded == b'/' {
                        // Skip decoding escaped slashes.
                        self.next_c = Some((msn as char, Some(lsn as char)));
                        return Some('%');
                    }

                    if (decoded as char).is_ascii_control() {
                        // We don't allow escaped ascii control codes for security reasons.
                        // We signal our error and then convert the code to it's visual
                        // equivalent in the `CONTROL_PICTURES` unicode block at 0x2400.
                        const CONTROL_PICTURES: u32 = 0x2400;
                        let c = core::char::from_u32(decoded as u32 + CONTROL_PICTURES).unwrap();
                        self.had_error = true;
                        if utf8_len == 0 {
                            return Some(c);
                        } else {
                            self.next_c = Some((c, None));
                            return Some(REPLACEMENT_CHARACTER);
                        }
                    } else if decoded & 0x80 == 0x80 {
                        // This is a UTF8 byte.
                        utf8_buf[utf8_len] = decoded;
                        utf8_len += 1;
                        if utf8_len >= UTF8_CHAR_WIDTH[utf8_buf[0] as usize] as usize {
                            if let Ok(utf8_str) = from_utf8(&utf8_buf[..utf8_len]) {
                                return Some(utf8_str.chars().next().unwrap());
                            } else {
                                self.had_error = true;
                                return Some(REPLACEMENT_CHARACTER);
                            }
                        }
                    } else if utf8_len != 0 {
                        self.had_error = true;
                        self.next_c = Some((decoded as char, None));
                        return Some(REPLACEMENT_CHARACTER);
                    } else {
                        return Some(decoded as char);
                    }
                }
                c => {
                    if utf8_len != 0 {
                        self.next_c = Some((c, None));
                        self.had_error = true;
                        return Some(REPLACEMENT_CHARACTER);
                    }
                    return Some(c);
                }
            }
        }
        None
    }
}
