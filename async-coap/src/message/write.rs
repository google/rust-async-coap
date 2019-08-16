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

/// Trait for writing/serializing a CoAP message.
pub trait MessageWrite: OptionInsert {
    /// Sets the CoAP message type. This may be called at any time during message writing
    /// without disrupting the operation. It may be called multiple times if necessary.
    /// The written value is that of the last call.
    fn set_msg_type(&mut self, tt: MsgType);

    /// Sets the CoAP message id. This may be called at any time during message writing
    /// without disrupting the operation. It may be called multiple times if necessary.
    /// The written value is that of the last call.
    fn set_msg_id(&mut self, msg_id: MsgId);

    /// Sets the CoAP message code. This may be called at any time during message writing
    /// without disrupting the operation. It may be called multiple times if necessary.
    /// The written value is that of the last call.
    fn set_msg_code(&mut self, code: MsgCode);

    /// Sets the CoAP message token. Calling this method out-of-order will cause any previously
    /// written options or payload to be lost. It may be called multiple times if necessary.
    /// The written value is that of the last call.
    fn set_msg_token(&mut self, token: MsgToken);

    /// Appends bytes from the given slice `body` to the payload of the message.
    /// This method should only be called after the token and all options have been set.
    /// This method may be called multiple times, each time appending data to the payload.
    fn append_payload_bytes(&mut self, body: &[u8]) -> Result<(), Error>;

    /// Appends bytes from the UTF8 representation of the given string slice `body` to the payload
    /// of the message.
    /// This method should only be called after the token and all options have been set.
    /// This method may be called multiple times, each time appending data to the payload.
    fn append_payload_string(&mut self, body: &str) -> Result<(), Error> {
        self.append_payload_bytes(body.as_bytes())
    }

    /// Appends a single byte to the payload of the message.
    /// This method should only be called after the token and all options have been set.
    /// This method may be called multiple times, each time appending data to the payload.
    fn append_payload_u8(&mut self, b: u8) -> Result<(), Error> {
        self.append_payload_bytes(&[b])
    }

    /// Appends the UTF8 representation for a single unicode character to the payload of the
    /// message.
    /// This method should only be called after the token and all options have been set.
    /// This method may be called multiple times, each time appending data to the payload.
    fn append_payload_char(&mut self, c: char) -> Result<(), Error> {
        self.append_payload_string(c.encode_utf8(&mut [0; 4]))
    }

    /// Removes the message payload along with all options.
    fn clear(&mut self);
}

impl<'a> core::fmt::Write for dyn MessageWrite + 'a {
    fn write_str(&mut self, s: &str) -> Result<(), core::fmt::Error> {
        self.append_payload_string(s)?;
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), core::fmt::Error> {
        self.append_payload_char(c)?;
        Ok(())
    }
}

impl<'a> std::io::Write for dyn MessageWrite + 'a {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.append_payload_bytes(buf)
            .map(|_| buf.len())
            .map_err(|_| std::io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.append_payload_bytes(buf)
            .map_err(|_| std::io::ErrorKind::Other.into())
    }
}

impl<'a> std::io::Write for BufferMessageEncoder<'a> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.append_payload_bytes(buf)
            .map(|_| buf.len())
            .map_err(|_| std::io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.append_payload_bytes(buf)
            .map_err(|_| std::io::ErrorKind::Other.into())
    }
}

impl std::io::Write for VecMessageEncoder {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.append_payload_bytes(buf)
            .map(|_| buf.len())
            .map_err(|_| std::io::ErrorKind::Other.into())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
        self.append_payload_bytes(buf)
            .map_err(|_| std::io::ErrorKind::Other.into())
    }
}
