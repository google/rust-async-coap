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

/// Null message writer. Anything written to this instance will be ignored.
#[derive(Debug)]
pub struct NullMessageWrite;
impl MessageWrite for NullMessageWrite {
    fn set_msg_type(&mut self, _: MsgType) {}

    fn set_msg_id(&mut self, _: u16) {}

    fn set_msg_code(&mut self, _: MsgCode) {}

    fn set_msg_token(&mut self, _: MsgToken) {}

    fn append_payload_bytes(&mut self, _: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn clear(&mut self) {}
}

impl OptionInsert for NullMessageWrite {
    fn insert_option_with_bytes(&mut self, _key: OptionNumber, _value: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// Null message reader. Always reads as an empty CoAP reset.
#[derive(Debug)]
pub struct NullMessageRead;
impl MessageRead for NullMessageRead {
    fn msg_code(&self) -> MsgCode {
        MsgCode::Empty
    }

    fn msg_type(&self) -> MsgType {
        MsgType::Res
    }

    fn msg_id(&self) -> u16 {
        0x0000
    }

    fn msg_token(&self) -> MsgToken {
        MsgToken::EMPTY
    }

    fn payload(&self) -> &[u8] {
        &[]
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
        OptionIterator::new(&[])
    }
}

impl std::fmt::Display for NullMessageRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        MessageDisplay(self).fmt(f)
    }
}
