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

//! Module defining various CoAP-related constants.

/// The standard default IP port number used for CoAP-over-UDP.
pub const DEFAULT_PORT_COAP_UDP: u16 = 5683;

/// The standard default IP port number used for CoAP-over-DTLS.
pub const DEFAULT_PORT_COAP_DTLS: u16 = 5684;

/// The standard default IP port number used for CoAP-over-TCP.
pub const DEFAULT_PORT_COAP_TCP: u16 = 5683;

/// The standard default IP port number used for CoAP-over-TLS.
pub const DEFAULT_PORT_COAP_TLS: u16 = 5684;

/// The standard URI scheme for vanilla CoAP-over-UDP on IP networks.
pub const URI_SCHEME_COAP: &'static str = "coap";

/// The standard URI scheme for CoAP-over-DTLS on IP networks.
pub const URI_SCHEME_COAPS: &'static str = "coaps";

/// The standard URI scheme for CoAP-over-TCP on IP networks.
pub const URI_SCHEME_COAP_TCP: &'static str = "coap+tcp";

/// The standard URI scheme for CoAP-over-TLS on IP networks.
pub const URI_SCHEME_COAPS_TCP: &'static str = "coaps+tcp";

/// Non-standard URI scheme for a [loopback interface](https://en.wikipedia.org/wiki/Loopback).
pub const URI_SCHEME_LOOPBACK: &'static str = "loop";

/// Non-standard URI scheme for a [null interface](https://en.wikipedia.org/wiki/Black_hole_(networking)).
pub const URI_SCHEME_NULL: &'static str = "null";

/// A fake hostname representing the "all CoAP devices" multicast addresses, or
/// the equivalent for a given network layer.
///
/// Note that the value of this string has been chosen somewhat arbitrarily and
/// is unlikely to be supported outside of this library. The trailing "dot" is to
/// ensure that it can never be interpreted as a partial domain name.
pub const ALL_COAP_DEVICES_HOSTNAME: &'static str = "all-coap-devices.";

/// String slice containing the "All CoAP Devices" IPv6 **Link**-Local Multicast Address: `FF02::FD`
pub const ALL_COAP_DEVICES_V6_LL: &'static str = "FF02::FD";

/// String slice containing the "All CoAP Devices" IPv6 **Realm**-Local Multicast Address: `FF03::FD`
pub const ALL_COAP_DEVICES_V6_RL: &'static str = "FF03::FD";

/// String slice containing the "All CoAP Devices" IPv4 **Link**-Local Multicast Address: `224.0.1.187`
pub const ALL_COAP_DEVICES_V4: &'static str = "224.0.1.187";

/// Value for `OptionNumber::OBSERVE` when registering an observer.
///
/// Note that this is only for requests, replies have entirely different semantics.
///
/// Defined by [IETF-RFC7641](https://tools.ietf.org/html/rfc7641).
pub const OBSERVE_REGISTER: u32 = 0;

/// Value for `OptionNumber::OBSERVE` when deregistering an observer.
///
/// Note that this is only for requests, replies have entirely different semantics.
///
/// Defined by [IETF-RFC7641](https://tools.ietf.org/html/rfc7641).
pub const OBSERVE_DEREGISTER: u32 = 1;

/// Value for `OptionNumber::NO_RESPONSE` when "Not interested in 2.xx responses".
/// From [RFC7967](https://tools.ietf.org/html/rfc7967).
pub const NO_RESPONSE_SUCCESS: u8 = 0b00000010;

/// Value for `OptionNumber::NO_RESPONSE` when "Not interested in 4.xx responses".
/// From [RFC7967](https://tools.ietf.org/html/rfc7967).
pub const NO_RESPONSE_CLIENT_ERROR: u8 = 0b00001000;

/// Value for `OptionNumber::NO_RESPONSE` when "Not interested in 5.xx responses".
/// From [RFC7967](https://tools.ietf.org/html/rfc7967).
pub const NO_RESPONSE_SERVER_ERROR: u8 = 0b00010000;

/// Value for `OptionNumber::NO_RESPONSE` when not interested in any response.
/// From [RFC7967](https://tools.ietf.org/html/rfc7967).
pub const NO_RESPONSE_ANY: u8 = 0;

/// Value for `OptionNumber::NO_RESPONSE` when not interested in any error response.
/// From [RFC7967](https://tools.ietf.org/html/rfc7967).
pub const NO_RESPONSE_ERROR: u8 = NO_RESPONSE_CLIENT_ERROR | NO_RESPONSE_SERVER_ERROR;
