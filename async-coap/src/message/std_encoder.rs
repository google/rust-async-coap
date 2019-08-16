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

use super::codec::*;
use super::*;

/// A class for writing stand-alone messages to a mutable byte slice.
#[derive(Debug)]
pub struct BufferMessageEncoder<'buf> {
    buffer: &'buf mut [u8],
    len: usize,
    option_start: usize,
    payload_start: usize,
    last_option: OptionNumber,
}

impl<'buf> BufferMessageEncoder<'buf> {
    /// The minimum size buffer that can be passed into `new`.
    pub const MIN_MESSAGE_BUFFER_LEN: usize = 12;

    /// Creates a new `BufferMessageEncoder` using the given buffer.
    pub fn new(buffer: &'buf mut [u8]) -> BufferMessageEncoder<'buf> {
        if buffer.len() < BufferMessageEncoder::MIN_MESSAGE_BUFFER_LEN {
            panic!("Buffer too small");
        }

        // Set version on first byte.
        buffer[0] = 0b01000000;

        BufferMessageEncoder {
            buffer,
            len: 4,
            option_start: 4,
            payload_start: 4,
            last_option: Default::default(),
        }
    }

    /// Returns a byte slice containing the encoded message.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    /// Returns the token set for this message.
    pub fn msg_token(&self) -> MsgToken {
        let token_len = (self.buffer[0] & COAP_MSG_TKL_MASK) as usize;
        MsgToken::new(&self.buffer[4..4 + token_len])
    }
}

impl<'buf> std::fmt::Display for BufferMessageEncoder<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        CoapByteDisplayFormatter(self.as_bytes()).fmt(f)
    }
}

impl<'buf> core::ops::Deref for BufferMessageEncoder<'buf> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl<'buf> MessageWrite for BufferMessageEncoder<'buf> {
    fn set_msg_type(&mut self, tt: MsgType) {
        self.buffer[0] = (self.buffer[0] & !COAP_MSG_T_MASK) | ((tt as u8) << COAP_MSG_T_OFFS);
    }

    fn set_msg_id(&mut self, msg_id: u16) {
        self.buffer[2] = (msg_id >> 8) as u8;
        self.buffer[3] = (msg_id >> 0) as u8;
    }

    fn set_msg_code(&mut self, code: MsgCode) {
        self.buffer[1] = code as u8;
    }

    fn set_msg_token(&mut self, token: MsgToken) {
        if self.option_start != 4 + token.len() {
            self.len = 4 + token.len();
            self.option_start = self.len;
            self.payload_start = self.option_start;

            self.buffer[0] = (self.buffer[0] & !COAP_MSG_TKL_MASK) | token.len() as u8;
        }

        self.buffer[4..4 + token.len()].copy_from_slice(token.as_bytes());
    }

    fn append_payload_bytes(&mut self, body: &[u8]) -> Result<(), Error> {
        if self.len == self.payload_start {
            if self.payload_start >= self.buffer.len() {
                return Err(Error::OutOfSpace);
            }
            // Append an end-of-options marker.
            self.buffer[self.payload_start] = 0xFF;
            self.len += 1;
        }

        let new_body_end = self.len + body.len();

        if new_body_end > self.buffer.len() {
            return Err(Error::OutOfSpace);
        }

        self.buffer[self.len..new_body_end].copy_from_slice(body);
        self.len = new_body_end;

        Ok(())
    }

    fn clear(&mut self) {
        self.buffer[0] = 0b01000000;
        self.len = 4;
        self.option_start = 4;
        self.payload_start = 4;
        self.last_option = Default::default();
    }
}

impl<'buf> OptionInsert for BufferMessageEncoder<'buf> {
    fn insert_option_with_bytes(&mut self, key: OptionNumber, value: &[u8]) -> Result<(), Error> {
        if self.last_option == key && !key.is_repeatable() {
            panic!("Option {} is not repeatable", key);
            //return Err(Error::OptionNotRepeatable);
        }
        let option_start = self.option_start;
        let (mut len, last_option) = insert_option(
            &mut self.buffer[option_start..],
            self.len - option_start,
            self.last_option,
            key,
            value,
        )?;

        len += option_start;
        self.last_option = last_option;
        self.len = len;
        self.payload_start = len;

        Ok(())
    }
}

/// A class for writing stand-alone messages to a heap-allocated [`Vec`].
#[derive(Debug)]
pub struct VecMessageEncoder {
    buffer: Vec<u8>,
    option_start: usize,
    payload_start: usize,
    last_option: OptionNumber,
}

impl VecMessageEncoder {
    /// Creates a new `VecMessageEncoder` instance.
    pub fn new() -> VecMessageEncoder {
        Self::with_payload_capacity(16)
    }

    /// Creates a new `VecMessageEncoder` instance with a specific capacity.
    pub fn with_payload_capacity(capacity: usize) -> VecMessageEncoder {
        let mut buffer = Vec::with_capacity(16 + capacity);

        // Set version on first byte.
        buffer.push(0b01000000);
        buffer.resize(4, 0);

        VecMessageEncoder {
            buffer,
            option_start: 4,
            payload_start: 4,
            last_option: Default::default(),
        }
    }

    /// Returns a byte slice containing the encoded message.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns the token set for this message.
    pub fn msg_token(&self) -> MsgToken {
        let token_len = (self.buffer[0] & COAP_MSG_TKL_MASK) as usize;
        MsgToken::new(&self.buffer[4..4 + token_len])
    }
}

impl std::convert::From<VecMessageEncoder> for Vec<u8> {
    fn from(x: VecMessageEncoder) -> Self {
        x.buffer
    }
}

impl std::convert::From<VecMessageEncoder> for OwnedImmutableMessage {
    fn from(x: VecMessageEncoder) -> Self {
        OwnedImmutableMessage::new(x.buffer).expect("Encoding corrupt")
    }
}

impl Default for VecMessageEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VecMessageEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        CoapByteDisplayFormatter(self.as_bytes()).fmt(f)
    }
}

impl core::ops::Deref for VecMessageEncoder {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl MessageWrite for VecMessageEncoder {
    fn set_msg_type(&mut self, tt: MsgType) {
        self.buffer[0] = (self.buffer[0] & !COAP_MSG_T_MASK) | ((tt as u8) << COAP_MSG_T_OFFS);
    }

    fn set_msg_id(&mut self, msg_id: u16) {
        self.buffer[2] = (msg_id >> 8) as u8;
        self.buffer[3] = (msg_id >> 0) as u8;
    }

    fn set_msg_code(&mut self, code: MsgCode) {
        self.buffer[1] = code as u8;
    }

    fn set_msg_token(&mut self, token: MsgToken) {
        if self.option_start != 4 + token.len() {
            self.buffer.resize(4 + token.len(), 0);
            self.option_start = self.buffer.len();
            self.payload_start = self.option_start;

            self.buffer[0] = (self.buffer[0] & !COAP_MSG_TKL_MASK) | token.len() as u8;
        }
        self.buffer[4..4 + token.len()].copy_from_slice(token.as_bytes());
    }

    fn append_payload_bytes(&mut self, body: &[u8]) -> Result<(), Error> {
        if self.buffer.len() == self.payload_start {
            // Append an end-of-options marker.
            self.buffer.push(0xFF);
        }
        self.buffer.extend_from_slice(body);
        Ok(())
    }

    fn clear(&mut self) {
        self.buffer[0] = 0b01000000;
        self.buffer.resize(4, 0);
        self.option_start = 4;
        self.payload_start = 4;
        self.last_option = Default::default();
    }
}

impl OptionInsert for VecMessageEncoder {
    fn insert_option_with_bytes(&mut self, key: OptionNumber, value: &[u8]) -> Result<(), Error> {
        if self.last_option == key && !key.is_repeatable() {
            return Err(Error::OptionNotRepeatable);
        }

        let option_start = self.option_start;

        let workspace = value.len() + 5;

        let len = self.buffer.len();
        self.buffer.resize(self.buffer.len() + workspace, 0);

        let (mut len, last_option) = insert_option(
            &mut self.buffer[option_start..],
            len - option_start,
            self.last_option,
            key,
            value,
        )?;

        len += option_start;
        self.buffer.truncate(len);
        self.last_option = last_option;
        self.payload_start = len;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::option::*;

    #[test]
    fn message_builder_rfc7252_fig_16() {
        let buffer = &mut [0u8; 200];

        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Con);
        builder.set_msg_code(MsgCode::MethodGet);
        builder.set_msg_id(0x7d34);
        assert_eq!(Ok(()), builder.insert_option(URI_PATH, "temperature"));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01000000, 1, 0x7d, 0x34, 0xbb, b't', b'e', b'm', b'p', b'e', b'r', b'a', b't', b'u',
            b'r', b'e',
        ];
        println!("request: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);

        let parser = StandardMessageParser::new(packet_real).unwrap();
        assert_eq!(MsgType::Con, parser.msg_type());
        assert_eq!(MsgCode::MethodGet, parser.msg_code());
        assert_eq!(0x7d34, parser.msg_id());
        assert_eq!(MsgToken::EMPTY, parser.msg_token());
        assert_eq!(None, parser.content_format());
        assert_eq!(None, parser.accept());
        assert!(parser.payload().is_empty());
        let mut iter = parser.options();
        assert_eq!(
            Some(Ok((OptionNumber::URI_PATH, &b"temperature"[..]))),
            iter.next()
        );
        assert_eq!(None, iter.next());

        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Ack);
        builder.set_msg_code(MsgCode::SuccessContent);
        builder.set_msg_id(0x7d34);
        assert_eq!(Ok(()), builder.append_payload_string(r#"22.3 C"#));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01100000, 69, 0x7d, 0x34, 0xff, b'2', b'2', b'.', b'3', b' ', b'C',
        ];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);
    }

    #[test]
    fn message_builder_rfc7252_fig_17() {
        let buffer = &mut [0u8; 200];

        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Con);
        builder.set_msg_code(MsgCode::MethodGet);
        builder.set_msg_id(0x7d34);
        builder.set_msg_token(MsgToken::from(0x20));
        assert_eq!(Ok(()), builder.insert_option(URI_PATH, "temperature"));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01000001, 1, 0x7d, 0x34, 0x20, 0xbb, b't', b'e', b'm', b'p', b'e', b'r', b'a', b't',
            b'u', b'r', b'e',
        ];
        println!("request: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);

        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Ack);
        builder.set_msg_code(MsgCode::SuccessContent);
        builder.set_msg_id(0x7d34);
        builder.set_msg_token(MsgToken::from(0x20));
        assert_eq!(Ok(()), builder.append_payload_string(r#"22.3 C"#));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01100001, 69, 0x7d, 0x34, 0x20, 0xff, b'2', b'2', b'.', b'3', b' ', b'C',
        ];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);
    }

    #[test]
    fn message_builder_append_body() {
        let buffer = &mut [0u8; 200];

        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Ack);
        builder.set_msg_code(MsgCode::SuccessContent);
        builder.set_msg_id(0x7d34);
        builder.set_msg_token(MsgToken::from(0x20));
        assert_eq!(Ok(()), builder.append_payload_string(r#"22."#));
        assert_eq!(Ok(()), builder.append_payload_string(r#"3 C"#));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01100001, 69, 0x7d, 0x34, 0x20, 0xff, b'2', b'2', b'.', b'3', b' ', b'C',
        ];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);
    }

    #[test]
    fn message_builder_misc() {
        let buffer = &mut [0u8; 200];
        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Con);
        builder.set_msg_code(MsgCode::MethodPost);
        builder.set_msg_id(0x7d34);
        builder.set_msg_token(MsgToken::from(0x2021));
        assert_eq!(
            Ok(()),
            builder.insert_option(CONTENT_FORMAT, ContentFormat::TEXT_PLAIN_UTF8)
        );
        assert_eq!(Ok(()), builder.insert_option(URI_PATH, "temp"));
        assert_eq!(Ok(()), builder.append_payload_string(r#"22."#));
        assert_eq!(Ok(()), builder.append_payload_string(r#"3 C"#));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01000010, 2, 0x7d, 0x34, 0x20, 0x21, 0xb4, b't', b'e', b'm', b'p', 0x10, 0xff, b'2',
            b'2', b'.', b'3', b' ', b'C',
        ];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);

        let parser = StandardMessageParser::new(packet_real).unwrap();

        assert_eq!(MsgType::Con, parser.msg_type());
        assert_eq!(MsgCode::MethodPost, parser.msg_code());
        assert_eq!(0x7d34, parser.msg_id());
        assert_eq!(MsgToken::from(0x2021), parser.msg_token());

        assert_eq!(
            Some(ContentFormat::TEXT_PLAIN_UTF8),
            parser.content_format()
        );
        assert_eq!(None, parser.accept());
        assert_eq!(b"22.3 C", parser.payload());

        let mut iter = parser.options();
        assert_eq!(
            Some(Ok((OptionNumber::URI_PATH, &b"temp"[..]))),
            iter.next()
        );
        assert_eq!(
            Some(Ok((OptionNumber::CONTENT_FORMAT, &b""[..]))),
            iter.next()
        );
        assert_eq!(None, iter.next());

        let mut iter = parser.options();
        assert_eq!(Some(Ok("temp")), iter.find_next_of(URI_PATH));
        assert_eq!(None, iter.find_next_of(URI_PATH));
        assert_eq!(
            Some(Ok(ContentFormat::TEXT_PLAIN_UTF8)),
            iter.find_next_of(CONTENT_FORMAT)
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn vec_message_builder_misc() {
        let mut builder = VecMessageEncoder::new();
        builder.set_msg_type(MsgType::Con);
        builder.set_msg_code(MsgCode::MethodPost);
        builder.set_msg_id(0x7d34);
        builder.set_msg_token(MsgToken::from(0x2021));
        assert_eq!(
            Ok(()),
            builder.insert_option(CONTENT_FORMAT, ContentFormat::TEXT_PLAIN_UTF8)
        );
        assert_eq!(Ok(()), builder.insert_option(URI_PATH, "temp"));
        assert_eq!(Ok(()), builder.append_payload_string(r#"22."#));
        assert_eq!(Ok(()), builder.append_payload_string(r#"3 C"#));
        let packet_calc: &[u8] = &builder;
        let packet_real = &[
            0b01000010, 2, 0x7d, 0x34, 0x20, 0x21, 0xb4, b't', b'e', b'm', b'p', 0x10, 0xff, b'2',
            b'2', b'.', b'3', b' ', b'C',
        ];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);

        let parser = StandardMessageParser::new(packet_real).unwrap();

        assert_eq!(MsgType::Con, parser.msg_type());
        assert_eq!(MsgCode::MethodPost, parser.msg_code());
        assert_eq!(0x7d34, parser.msg_id());
        assert_eq!(MsgToken::from(0x2021), parser.msg_token());

        assert_eq!(
            Some(ContentFormat::TEXT_PLAIN_UTF8),
            parser.content_format()
        );
        assert_eq!(None, parser.accept());
        assert_eq!(b"22.3 C", parser.payload());

        let mut iter = parser.options();
        assert_eq!(
            Some(Ok((OptionNumber::URI_PATH, &b"temp"[..]))),
            iter.next()
        );
        assert_eq!(
            Some(Ok((OptionNumber::CONTENT_FORMAT, &b""[..]))),
            iter.next()
        );
        assert_eq!(None, iter.next());

        let mut iter = parser.options();
        assert_eq!(Some(Ok("temp")), iter.find_next_of(URI_PATH));
        assert_eq!(None, iter.find_next_of(URI_PATH));
        assert_eq!(
            Some(Ok(ContentFormat::TEXT_PLAIN_UTF8)),
            iter.find_next_of(CONTENT_FORMAT)
        );
        assert_eq!(None, iter.next());
    }

    #[test]
    fn message_builder_reset() {
        let buffer = &mut [0u8; 200];
        let mut builder = BufferMessageEncoder::new(buffer);
        builder.set_msg_type(MsgType::Con);
        builder.set_msg_code(MsgCode::Empty);
        builder.set_msg_id(0x0000);
        builder.set_msg_token(MsgToken::EMPTY);
        let packet_calc: &[u8] = &builder;
        let packet_real = &[0b01000000, 0, 0x00, 0x00];
        println!("response: {:#x?}", packet_calc);
        assert_eq!(packet_real, packet_calc);

        let parser = StandardMessageParser::new(packet_real).unwrap();

        assert_eq!(MsgType::Con, parser.msg_type());
        assert_eq!(MsgCode::Empty, parser.msg_code());
        assert_eq!(0x0, parser.msg_id());
        assert_eq!(MsgToken::EMPTY, parser.msg_token());

        assert_eq!(None, parser.content_format());
        assert_eq!(None, parser.accept());
        assert_eq!(b"", parser.payload());
    }
}
