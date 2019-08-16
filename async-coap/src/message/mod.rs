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

//! Types related to parsing and encoding CoAP messages.
//!
use super::*;

/// Type for representing a CoAP message id.
pub type MsgId = u16;

mod read;
pub use read::AckMessage;
pub use read::MessageRead;
pub use read::ResetMessage;

mod write;
pub use write::MessageWrite;

mod msg_code;
pub use msg_code::MsgCode;
pub use msg_code::MsgCodeClass;

mod msg_type;
pub use msg_type::MsgType;

mod display;
pub use display::CoapByteDisplayFormatter;
pub use display::MessageDisplay;

mod null;
pub use null::NullMessageRead;
pub use null::NullMessageWrite;

mod std_encoder;
pub use std_encoder::BufferMessageEncoder;
pub use std_encoder::VecMessageEncoder;

mod std_parser;
pub use std_parser::OwnedImmutableMessage;
pub use std_parser::StandardMessageParser;

mod token;
pub use token::*;

pub mod codec;

#[allow(dead_code)]
const COAP_MSG_VER_MASK: u8 = 0b11000000;

#[allow(dead_code)]
const COAP_MSG_VER_OFFS: u8 = 6;

#[allow(dead_code)]
const COAP_MSG_T_MASK: u8 = 0b00110000;

#[allow(dead_code)]
const COAP_MSG_T_OFFS: u8 = 4;

#[allow(dead_code)]
const COAP_MSG_TKL_MASK: u8 = 0b00001111;

#[allow(dead_code)]
const COAP_MSG_TKL_OFFS: u8 = 0;
