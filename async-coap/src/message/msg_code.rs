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

/// Enum representing the *class* of a CoAP message code.
#[derive(Debug, Copy, Eq, PartialEq, Clone)]
pub enum MsgCodeClass {
    /// Class for methods
    Method = 0,

    /// Class for successful responses
    Success = 2,

    /// Class for client error responses
    ClientError = 4,

    /// Class for server error responses
    ServerError = 5,

    /// Class for in-band signaling
    Signal = 7,
}

impl MsgCodeClass {
    /// Tries to calculate the message code class from the given message code.
    pub fn try_from(x: u8) -> Option<MsgCodeClass> {
        match x {
            0 => Some(MsgCodeClass::Method),
            2 => Some(MsgCodeClass::Success),
            4 => Some(MsgCodeClass::ClientError),
            5 => Some(MsgCodeClass::ServerError),
            7 => Some(MsgCodeClass::Signal),
            _ => None,
        }
    }

    /// Returns true if the given message code is in this message code class.
    pub fn contains(self, code: MsgCode) -> bool {
        let code_u8 = code as u8;

        code_u8 != 0 && (code_u8 >> 5) == self as u8
    }
}

/// Helper function
const fn calc_code(class: u8, detail: u8) -> isize {
    (((class & 0x7) << 5) + detail) as isize
}

/// Enum representing a CoAP message code.
#[derive(Debug, Copy, Eq, PartialEq, Clone)]
pub enum MsgCode {
    /// Empty message code. Only used for ping requests, resets, and empty acknowledgements.
    Empty = 0x00,

    /// CoAP GET method.
    MethodGet = 0x01,

    /// CoAP POST method.
    MethodPost = 0x02,

    /// CoAP PUT method.
    MethodPut = 0x03,

    /// CoAP DELETE method.
    MethodDelete = 0x04,

    /// CoAP FETCH method.
    MethodFetch = 0x05,

    /// CoAP PATCH method.
    MethodPatch = 0x06,

    /// CoAP iPATCH method.
    MethodIPatch = 0x07,

    /// CoAP CREATED success code.
    SuccessCreated = 0x41,

    /// CoAP DELETED success code.
    SuccessDeleted = 0x42,

    /// CoAP VALID success code.
    SuccessValid = 0x43,

    /// CoAP CHANGED success code.
    SuccessChanged = 0x44,

    /// CoAP CONTENT success code.
    SuccessContent = 0x45,

    /// CoAP CONTINUE success code.
    SuccessContinue = 0x5F,

    /// CoAP BAD_REQUEST client error.
    ClientErrorBadRequest = 0x80,

    /// CoAP UNAUTHORIZED client error.
    ClientErrorUnauthorized = 0x81,

    /// CoAP BAD_OPTION client error.
    ClientErrorBadOption = 0x82,

    /// CoAP FORBIDDEN client error.
    ClientErrorForbidden = 0x83,

    /// CoAP NOT_FOUND client error.
    ClientErrorNotFound = 0x84,

    /// CoAP METHOD_NOT_ALLOWED client error.
    ClientErrorMethodNotAllowed = 0x85,

    /// CoAP NOT_ACCEPTABLE client error.
    ClientErrorNotAcceptable = 0x86,

    /// CoAP REQUEST_ENTITY_INCOMPLETE client error.
    ClientErrorRequestEntityIncomplete = 0x88,

    /// CoAP PRECONDITION_FAILED client error.
    ClientErrorPreconditionFailed = 0x8C,

    /// CoAP REQUEST_ENTITY_TOO_LARGE client error.
    ClientErrorRequestEntityTooLarge = 0x8D,

    /// CoAP UNSUPPORTED_MEDIA_TYPE client error.
    ClientErrorUnsupportedMediaType = 0x8F,

    /// RFC8516 "Too Many Requests" Response Code for the Constrained Application Protocol
    ClientErrorTooManyRequests = calc_code(4, 29),

    /// CoAP INTERNAL_SERVER_ERROR server error.
    ServerErrorInternalServerError = 0xA0,

    /// CoAP NOT_IMPLEMENTED server error.
    ServerErrorNotImplemented = 0xA1,

    /// CoAP BAD_GATEWAY server error.
    ServerErrorBadGateway = 0xA2,

    /// CoAP SERVICE_UNAVAILABLE server error.
    ServerErrorServiceUnavailable = 0xA3,

    /// CoAP GATEWAY_TIMEOUT server error.
    ServerErrorGatewayTimeout = 0xA4,

    /// CoAP PROXYING_NOT_SUPPORTED server error.
    ServerErrorProxyingNotSupported = 0xA5,

    /// CoAP CSM in-band signal.
    SignalCsm = 0xE1,

    /// CoAP PING in-band signal.
    SignalPing = 0xE2,

    /// CoAP PONG in-band signal.
    SignalPong = 0xE3,

    /// CoAP RELEASE in-band signal.
    SignalRelease = 0xE4,

    /// CoAP ABORT in-band signal.
    SignalAbort = 0xE5,
}

impl MsgCode {
    /// Tries to convert the given `u8` into a `MsgCode`. If the given code isn't recognized,
    /// this method will return `None`.
    pub fn try_from(x: u8) -> Option<MsgCode> {
        use MsgCode::*;
        match x {
            0x00 => Some(Empty),
            0x01 => Some(MethodGet),
            0x02 => Some(MethodPost),
            0x03 => Some(MethodPut),
            0x04 => Some(MethodDelete),

            0x41 => Some(SuccessCreated),
            0x42 => Some(SuccessDeleted),
            0x43 => Some(SuccessValid),
            0x44 => Some(SuccessChanged),
            0x45 => Some(SuccessContent),
            0x5F => Some(SuccessContinue),

            0x80 => Some(ClientErrorBadRequest),
            0x81 => Some(ClientErrorUnauthorized),
            0x82 => Some(ClientErrorBadOption),
            0x83 => Some(ClientErrorForbidden),
            0x84 => Some(ClientErrorNotFound),
            0x85 => Some(ClientErrorMethodNotAllowed),
            0x86 => Some(ClientErrorNotAcceptable),
            0x88 => Some(ClientErrorRequestEntityIncomplete),
            0x8C => Some(ClientErrorPreconditionFailed),
            0x8D => Some(ClientErrorRequestEntityTooLarge),
            0x8F => Some(ClientErrorUnsupportedMediaType),
            0x9D => Some(ClientErrorTooManyRequests),

            0xA0 => Some(ServerErrorInternalServerError),
            0xA1 => Some(ServerErrorNotImplemented),
            0xA2 => Some(ServerErrorBadGateway),
            0xA3 => Some(ServerErrorServiceUnavailable),
            0xA4 => Some(ServerErrorGatewayTimeout),
            0xA5 => Some(ServerErrorProxyingNotSupported),

            0xE1 => Some(SignalCsm),
            0xE2 => Some(SignalPing),
            0xE3 => Some(SignalPong),
            0xE4 => Some(SignalRelease),
            0xE5 => Some(SignalAbort),

            _ => None,
        }
    }

    /// Returns an approximation of this message code as an HTTP status code.
    pub fn to_http_code(self) -> u16 {
        ((self as u8) >> 5) as u16 * 100 + (self as u8 as u16) & 0b11111
    }

    /// Returns true if this is the empty code.
    pub fn is_empty(self) -> bool {
        self as u8 == 0
    }

    /// Returns true if message code is a method.
    pub fn is_method(self) -> bool {
        MsgCodeClass::Method.contains(self)
    }

    /// Returns true if message code is a client error.
    pub fn is_client_error(self) -> bool {
        MsgCodeClass::ClientError.contains(self)
    }

    /// Returns true if message code is a server error.
    pub fn is_server_error(self) -> bool {
        MsgCodeClass::ServerError.contains(self)
    }

    /// Returns true if message code is any sort of error.
    pub fn is_error(self) -> bool {
        self.is_client_error() || self.is_server_error()
    }

    /// Returns true if message code indicates success.
    pub fn is_success(self) -> bool {
        MsgCodeClass::Success.contains(self)
    }

    /// Returns true if message code is an in-band signal.
    pub fn is_signal(self) -> bool {
        MsgCodeClass::Signal.contains(self)
    }
}

impl Default for MsgCode {
    fn default() -> Self {
        MsgCode::Empty
    }
}

impl core::convert::From<MsgCode> for u8 {
    fn from(code: MsgCode) -> Self {
        code as u8
    }
}

impl core::convert::From<MsgCode> for u16 {
    fn from(code: MsgCode) -> Self {
        code as u16
    }
}

impl core::convert::From<MsgCode> for u32 {
    fn from(code: MsgCode) -> Self {
        code as u32
    }
}
