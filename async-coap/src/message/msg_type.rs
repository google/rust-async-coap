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

/// Enum representing the CoAP message type: `CON`, `NON`, `ACK`, and `RES`.
#[derive(Debug, Copy, Eq, PartialEq, Clone)]
pub enum MsgType {
    /// Variant for confirmable CoAP messages.
    Con = 0,

    /// Variant for non-confirmable CoAP messages.
    Non = 1,

    /// Variant for CoAP message acknowledgements.
    Ack = 2,

    /// Variant for CoAP reset messages.
    Res = 3,
}

impl MsgType {
    /// Creates a new `MsgType` from the given value, panicing if the value is invalid.
    pub fn from(tt: u8) -> MsgType {
        MsgType::try_from(tt).expect("Invalid message type")
    }

    /// Creates a new `MsgType` from the given value, returning `None` if the value is invalid.
    pub fn try_from(tt: u8) -> Option<MsgType> {
        match tt {
            0 => Some(MsgType::Con),
            1 => Some(MsgType::Non),
            2 => Some(MsgType::Ack),
            3 => Some(MsgType::Res),
            _ => None,
        }
    }

    /// Returns true if this message type is nonconfirmable (NON).
    pub fn is_non(self) -> bool {
        self == MsgType::Non
    }

    /// Returns true if this message type is confirmable (CON).
    pub fn is_con(self) -> bool {
        self == MsgType::Con
    }

    /// Returns true if this message type is an acknowledgement (ACK).
    pub fn is_ack(self) -> bool {
        self == MsgType::Ack
    }

    /// Returns true if this message type is a reset (RES).
    pub fn is_res(self) -> bool {
        self == MsgType::Res
    }
}

impl Default for MsgType {
    fn default() -> Self {
        MsgType::Con
    }
}
