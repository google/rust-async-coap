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
use core::fmt::{Display, Formatter};

/// Provides an implementation of [`core::fmt::Debug`] and [`core::fmt::Display`] for
/// any type implementing [`MessageRead`].
#[derive(Debug)]
pub struct MessageDisplay<'a, T: MessageRead + ?Sized>(pub &'a T);

impl<'a, T: MessageRead + ?Sized> Display for MessageDisplay<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "<{:?} {:?}", self.0.msg_type(), self.0.msg_code())?;
        write!(f, " MID:{:04X}", self.0.msg_id())?;

        let mut content_format: Option<u16> = None;

        let token = self.0.msg_token();
        if !token.is_empty() {
            write!(f, " TOK:{}", token)?;
        }

        for option in self.0.options() {
            match option {
                Ok((number, bytes)) => {
                    if number == OptionNumber::CONTENT_FORMAT {
                        content_format = try_decode_u16(bytes);
                    }
                    f.write_str(" ")?;
                    number.fmt_with_value(f, bytes)?;
                }
                Err(e) => return write!(f, " ERR:{:?}>", e),
            }
        }

        let payload = self.0.payload();
        if !payload.is_empty() {
            let payload_str_opt = if let Some(i) = content_format {
                if ContentFormat(i).is_utf8() {
                    std::str::from_utf8(payload).ok()
                } else {
                    None
                }
            } else {
                std::str::from_utf8(payload).ok()
            };

            if let Some(payload_str) = payload_str_opt {
                write!(f, " {:?}", payload_str)?;
            } else {
                write!(f, " {:?}", payload)?;
            }
        }

        write!(f, ">")
    }
}

/// Helper struct for formatting a CoAP buffer for display.
#[derive(Copy, Clone)]
pub struct CoapByteDisplayFormatter<'buf>(pub &'buf [u8]);

impl<'buf> std::fmt::Display for CoapByteDisplayFormatter<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(x) = StandardMessageParser::new(self.0) {
            MessageDisplay(&x).fmt(f)
        } else {
            write!(f, "<CORRUPTED {:02x?}>", self.0)
        }
    }
}

impl<'buf> std::fmt::Debug for CoapByteDisplayFormatter<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(x) = StandardMessageParser::new(self.0) {
            write!(
                f,
                "CoapByteDisplayFormatter({}, {:02x?})",
                MessageDisplay(&x),
                self.0
            )
        } else {
            write!(f, "<CORRUPTED {:02x?}>", self.0)
        }
    }
}
