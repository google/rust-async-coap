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
use std::borrow::Borrow;

/// A class for parsing a stand-alone UDP CoAP message from a given buffer.
#[derive(Debug)]
pub struct StandardMessageParser<'buf> {
    buffer: &'buf [u8],
    msg_code: MsgCode,
    msg_type: MsgType,
    msg_id: u16,
    token: MsgToken,
    content_format: Option<ContentFormat>,
    accept: Option<ContentFormat>,
    block2: Option<BlockInfo>,
    block1: Option<BlockInfo>,
    option_start: usize,
    payload_start: usize,
}

impl<'buf> std::fmt::Display for StandardMessageParser<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MessageDisplay(self).fmt(f)
    }
}

impl<'buf> StandardMessageParser<'buf> {
    /// The minimum buffer size that can be passed into `new()`.
    pub const MIN_MESSAGE_BUFFER_LEN: usize = 4;

    /// Creates a new `StandardMessageParser` instance with the given `buffer`.
    pub fn new(buffer: &'buf [u8]) -> Result<StandardMessageParser<'buf>, Error> {
        if buffer.len() < StandardMessageParser::MIN_MESSAGE_BUFFER_LEN {
            return Err(Error::ParseFailure);
        }

        let msg_code = MsgCode::try_from(buffer[1]).ok_or(Error::UnknownMessageCode)?;

        let msg_type = MsgType::from((buffer[0] & COAP_MSG_T_MASK) >> COAP_MSG_T_OFFS);
        let msg_id = buffer[3] as u16 | ((buffer[2] as u16) << 8);
        let token_len = (buffer[0] & COAP_MSG_TKL_MASK) as usize;
        if token_len > 8 {
            return Err(Error::ParseFailure);
        }
        let token = MsgToken::new(&buffer[4..4 + token_len]);

        let mut content_format = None;
        let mut accept = None;
        let mut block2 = None;
        let mut block1 = None;

        let mut iter = OptionIterator::new(&buffer[4 + token_len..]);

        for result in &mut iter {
            match result {
                Ok((OptionNumber::CONTENT_FORMAT, value)) => {
                    content_format = Some(ContentFormat(
                        try_decode_u16(value).ok_or(Error::ParseFailure)?,
                    ));
                }
                Ok((OptionNumber::ACCEPT, value)) => match try_decode_u16(value) {
                    Some(x) => accept = Some(ContentFormat(x)),
                    None => return Err(Error::ParseFailure),
                },
                Ok((OptionNumber::BLOCK2, value)) => match try_decode_u32(value) {
                    Some(x) => block2 = Some(BlockInfo(x).valid().ok_or(Error::ParseFailure)?),
                    None => return Err(Error::ParseFailure),
                },
                Ok((OptionNumber::BLOCK1, value)) => match try_decode_u32(value) {
                    Some(x) => block1 = Some(BlockInfo(x).valid().ok_or(Error::ParseFailure)?),
                    None => return Err(Error::ParseFailure),
                },
                Ok((_key, _value)) => {
                    // Skip.
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        let payload_start = iter.as_slice().as_ptr() as usize - buffer.as_ptr() as usize;

        let ret = StandardMessageParser {
            buffer,
            msg_code,
            msg_type,
            msg_id,
            token,
            content_format,
            accept,
            block2,
            block1,
            option_start: 4 + token_len,
            payload_start,
        };

        Ok(ret)
    }

    /// Returns a byte slice containing the encoded message.
    pub fn as_bytes(&self) -> &'buf [u8] {
        self.buffer
    }
}

impl<'buf> MessageRead for StandardMessageParser<'buf> {
    fn msg_code(&self) -> MsgCode {
        self.msg_code
    }

    fn msg_type(&self) -> MsgType {
        self.msg_type
    }

    fn msg_id(&self) -> u16 {
        self.msg_id
    }

    fn msg_token(&self) -> MsgToken {
        self.token
    }

    fn payload(&self) -> &[u8] {
        &self.buffer[self.payload_start..]
    }

    fn content_format(&self) -> Option<ContentFormat> {
        self.content_format
    }

    fn accept(&self) -> Option<ContentFormat> {
        self.accept
    }

    fn block2(&self) -> Option<BlockInfo> {
        self.block2
    }

    fn block1(&self) -> Option<BlockInfo> {
        self.block1
    }

    fn options(&self) -> OptionIterator<'_> {
        OptionIterator::new(&self.buffer[4 + self.token.len()..])
    }
}

/// A class representing an immutable heap-allocated UDP CoAP message.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OwnedImmutableMessage {
    buffer: Vec<u8>,
    msg_code: MsgCode,
    msg_type: MsgType,
    msg_id: u16,
    token: MsgToken,
    content_format: Option<ContentFormat>,
    accept: Option<ContentFormat>,
    block2: Option<BlockInfo>,
    block1: Option<BlockInfo>,
    option_start: usize,
    payload_start: usize,
}

impl std::fmt::Display for OwnedImmutableMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MessageDisplay(self).fmt(f)
    }
}

impl<'a> Borrow<dyn MessageRead + 'a> for OwnedImmutableMessage {
    fn borrow(&self) -> &(dyn MessageRead + 'a) {
        self
    }
}

impl OwnedImmutableMessage {
    /// The minimum size of a buffer that can be passed into `new()`.
    pub const MIN_MESSAGE_BUFFER_LEN: usize = 4;

    /// Creates a new `OwnedImmutableMessage` instance with the given `buffer`.
    pub fn new(buffer: Vec<u8>) -> Result<OwnedImmutableMessage, Error> {
        let msg_code = MsgCode::try_from(buffer[1]).ok_or(Error::UnknownMessageCode)?;

        let msg_type = MsgType::from((buffer[0] & COAP_MSG_T_MASK) >> COAP_MSG_T_OFFS);
        let msg_id = buffer[3] as u16 | ((buffer[2] as u16) << 8);
        let token_len = (buffer[0] & COAP_MSG_TKL_MASK) as usize;
        if token_len > 8 {
            return Err(Error::ParseFailure);
        }
        let token = MsgToken::new(&buffer[4..4 + token_len]);

        let mut content_format = None;
        let mut accept = None;
        let mut block2 = None;
        let mut block1 = None;

        let mut iter = OptionIterator::new(&buffer[4 + token_len..]);

        for result in &mut iter {
            match result {
                Ok((OptionNumber::CONTENT_FORMAT, value)) => {
                    content_format = Some(ContentFormat(
                        try_decode_u16(value).ok_or(Error::ParseFailure)?,
                    ));
                }
                Ok((OptionNumber::ACCEPT, value)) => match try_decode_u16(value) {
                    Some(x) => accept = Some(ContentFormat(x)),
                    None => return Err(Error::ParseFailure),
                },
                Ok((OptionNumber::BLOCK2, value)) => match try_decode_u32(value) {
                    Some(x) => block2 = Some(BlockInfo(x).valid().ok_or(Error::ParseFailure)?),
                    None => return Err(Error::ParseFailure),
                },
                Ok((OptionNumber::BLOCK1, value)) => match try_decode_u32(value) {
                    Some(x) => block1 = Some(BlockInfo(x).valid().ok_or(Error::ParseFailure)?),
                    None => return Err(Error::ParseFailure),
                },
                Ok((_key, _value)) => {
                    // Skip.
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        let payload_start = iter.as_slice().as_ptr() as usize - buffer.as_ptr() as usize;

        let ret = OwnedImmutableMessage {
            buffer,
            msg_code,
            msg_type,
            msg_id,
            token,
            content_format,
            accept,
            block2,
            block1,
            option_start: 4 + token_len,
            payload_start,
        };

        Ok(ret)
    }

    /// Returns a byte slice containing the encoded message.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}

impl MessageRead for OwnedImmutableMessage {
    fn msg_code(&self) -> MsgCode {
        self.msg_code
    }

    fn msg_type(&self) -> MsgType {
        self.msg_type
    }

    fn msg_id(&self) -> u16 {
        self.msg_id
    }

    fn msg_token(&self) -> MsgToken {
        self.token
    }

    fn payload(&self) -> &[u8] {
        &self.buffer[self.payload_start..]
    }

    fn content_format(&self) -> Option<ContentFormat> {
        self.content_format
    }

    fn accept(&self) -> Option<ContentFormat> {
        self.accept
    }

    fn block2(&self) -> Option<BlockInfo> {
        self.block2
    }

    fn block1(&self) -> Option<BlockInfo> {
        self.block1
    }

    fn options(&self) -> OptionIterator<'_> {
        OptionIterator::new(&self.buffer[4 + self.token.len()..])
    }
}
