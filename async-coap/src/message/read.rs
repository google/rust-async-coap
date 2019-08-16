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
use crate::option::OptionIterator;

/// Trait for reading the various parts of a CoAP message.
pub trait MessageRead {
    /// Gets the message code for this message.
    fn msg_code(&self) -> MsgCode;

    /// Gets the message type for this message.
    fn msg_type(&self) -> MsgType;

    /// Gets the message id for this message.
    fn msg_id(&self) -> MsgId;

    /// Gets the message token for this message.
    fn msg_token(&self) -> MsgToken;

    /// Gets the payload as a byte slice.
    fn payload(&self) -> &[u8];

    /// Gets an iterator for processing the options of the message.
    fn options(&self) -> OptionIterator<'_>;

    /// Writes this message to the given `target` that implements [`MessageWrite`].
    ///
    /// If this message has a message id ([`msg_id`][MessageRead::msg_id]) of zero, the message
    /// id will not be written to `target`.
    fn write_msg_to(&self, target: &mut dyn MessageWrite) -> Result<(), Error> {
        target.set_msg_type(self.msg_type());
        target.set_msg_code(self.msg_code());
        let msg_id = self.msg_id();
        if msg_id != 0 {
            target.set_msg_id(self.msg_id());
        }
        target.set_msg_token(self.msg_token());

        for opt in self.options() {
            let opt = opt?;
            target.insert_option_with_bytes(opt.0, opt.1)?;
        }

        target.append_payload_bytes(self.payload())?;
        Ok(())
    }

    /// Gets the payload as a string slice.
    fn payload_as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.payload()).ok()
    }

    /// Indicates the content format of the payload, if specified.
    fn content_format(&self) -> Option<ContentFormat>;

    /// Indicates the content format that the sender of the message will accept
    /// for the payload of the response, if specified.
    fn accept(&self) -> Option<ContentFormat>;

    /// Returns the value of the `block2` option for this message, if any.
    fn block2(&self) -> Option<BlockInfo>;

    /// Returns the value of the `block1` option for this message, if any.
    fn block1(&self) -> Option<BlockInfo>;
}

impl<'a> ToOwned for dyn MessageRead + 'a {
    type Owned = OwnedImmutableMessage;

    fn to_owned(&self) -> Self::Owned {
        let mut target = VecMessageEncoder::default();

        // UNWRAP-SAFETY: Should only happen under severe memory pressure.
        self.write_msg_to(&mut target).unwrap();

        target.into()
    }
}

/// A type representing a reset message.
///
/// This type is useful for quickly writing out reset responses via the
/// [`write_msg_to`] method.
///
/// [`write_msg_to`]: MessageRead::write_msg_to
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ResetMessage;
impl std::fmt::Display for ResetMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MessageDisplay(self).fmt(f)
    }
}
impl MessageRead for ResetMessage {
    fn msg_code(&self) -> MsgCode {
        MsgCode::Empty
    }

    fn msg_type(&self) -> MsgType {
        MsgType::Res
    }

    fn msg_id(&self) -> u16 {
        0
    }

    fn msg_token(&self) -> MsgToken {
        Default::default()
    }

    fn payload(&self) -> &[u8] {
        Default::default()
    }

    fn content_format(&self) -> Option<ContentFormat> {
        None
    }

    fn accept(&self) -> Option<ContentFormat> {
        None
    }

    fn block2(&self) -> Option<BlockInfo> {
        None
    }

    fn block1(&self) -> Option<BlockInfo> {
        None
    }

    fn options(&self) -> OptionIterator<'_> {
        Default::default()
    }

    fn write_msg_to(&self, target: &mut dyn MessageWrite) -> Result<(), Error> {
        target.set_msg_code(MsgCode::Empty);
        target.set_msg_type(MsgType::Res);
        target.set_msg_token(MsgToken::EMPTY);
        Ok(())
    }
}

/// A type representing an acknowledgement message.
///
/// This type is useful for quickly writing out reset responses via the
/// [`write_msg_to`] method.
///
/// [`write_msg_to`]: MessageRead::write_msg_to
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AckMessage;
impl std::fmt::Display for AckMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MessageDisplay(self).fmt(f)
    }
}
impl MessageRead for AckMessage {
    fn msg_code(&self) -> MsgCode {
        MsgCode::Empty
    }

    fn msg_type(&self) -> MsgType {
        MsgType::Ack
    }

    fn msg_id(&self) -> u16 {
        0
    }

    fn msg_token(&self) -> MsgToken {
        Default::default()
    }

    fn payload(&self) -> &[u8] {
        Default::default()
    }

    fn content_format(&self) -> Option<ContentFormat> {
        None
    }

    fn accept(&self) -> Option<ContentFormat> {
        None
    }

    fn block2(&self) -> Option<BlockInfo> {
        None
    }

    fn block1(&self) -> Option<BlockInfo> {
        None
    }

    fn options(&self) -> OptionIterator<'_> {
        Default::default()
    }

    fn write_msg_to(&self, target: &mut dyn MessageWrite) -> Result<(), Error> {
        target.set_msg_code(MsgCode::Empty);
        target.set_msg_type(MsgType::Ack);
        target.set_msg_token(MsgToken::EMPTY);
        Ok(())
    }
}
