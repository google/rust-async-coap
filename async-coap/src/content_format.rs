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

use std::borrow::Cow;

/// A type for representing a CoAP Content Format value.
#[derive(Debug, Copy, Eq, PartialEq, Hash, Clone, Ord, PartialOrd)]
pub struct ContentFormat(pub u16);

impl ContentFormat {
    /// From IETF-RFC7252.
    pub const TEXT_PLAIN_UTF8: ContentFormat = ContentFormat(0);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_ENCRYPT0: ContentFormat = ContentFormat(16);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_MAC0: ContentFormat = ContentFormat(17);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_SIGN1: ContentFormat = ContentFormat(18);

    /// From IETF-RFC7252.
    pub const APPLICATION_LINK_FORMAT: ContentFormat = ContentFormat(40);

    /// From IETF-RFC7252.
    pub const APPLICATION_XML: ContentFormat = ContentFormat(41);

    /// From IETF-RFC7252.
    pub const APPLICATION_OCTET_STREAM: ContentFormat = ContentFormat(42);

    /// From IETF-RFC7252.
    pub const APPLICATION_EXI: ContentFormat = ContentFormat(47);

    /// From IETF-RFC7252.
    pub const APPLICATION_JSON: ContentFormat = ContentFormat(50);

    /// From IETF-RFC6902 JavaScript Object Notation (JSON) Patch
    pub const APPLICATION_JSON_PATCH_JSON: ContentFormat = ContentFormat(51);

    /// From IETF-RFC7396 JSON Merge Patch
    pub const APPLICATION_MERGE_PATCH_JSON: ContentFormat = ContentFormat(52);

    /// From IETF-RFC7049 Concise Binary Object Representation (CBOR)
    pub const APPLICATION_CBOR: ContentFormat = ContentFormat(60);

    /// From IETF-RFC8392 CBOR Web Token
    pub const APPLICATION_CWT: ContentFormat = ContentFormat(61);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_ENCRYPT: ContentFormat = ContentFormat(96);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_MAC: ContentFormat = ContentFormat(97);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_COSE_SIGN: ContentFormat = ContentFormat(98);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_KEY: ContentFormat = ContentFormat(101);

    /// From IETF-RFC8152
    pub const APPLICATION_COSE_KEY_SET: ContentFormat = ContentFormat(102);

    /// JSON-formatted RFC8428 Sensor Measurement Lists (SenML)
    pub const APPLICATION_SENML_JSON: ContentFormat = ContentFormat(110);

    /// JSON-formatted RFC8428 Sensor Streaming Measurement List (SenSML)
    pub const APPLICATION_SENSML_JSON: ContentFormat = ContentFormat(111);

    /// CBOR-formatted RFC8428 Sensor Measurement Lists (SenML)
    pub const APPLICATION_SENML_CBOR: ContentFormat = ContentFormat(112);

    /// CBOR-formatted RFC8428 Sensor Streaming Measurement List (SenSML)
    pub const APPLICATION_SENSML_CBOR: ContentFormat = ContentFormat(113);

    /// EXI-formatted RFC8428 Sensor Measurement Lists (SenML)
    pub const APPLICATION_SENML_EXI: ContentFormat = ContentFormat(114);

    /// EXI-formatted RFC8428 Sensor Streaming Measurement List (SenSML)
    pub const APPLICATION_SENSML_EXI: ContentFormat = ContentFormat(115);

    /// XML-formatted RFC8428 Sensor Measurement Lists (SenML)
    pub const APPLICATION_SENML_XML: ContentFormat = ContentFormat(310);

    /// XML-formatted RFC8428 Sensor Streaming Measurement List (SenSML)
    pub const APPLICATION_SENSML_XML: ContentFormat = ContentFormat(311);

    /// [IETF-RFC7389] Group Communication for the Constrained Application Protocol
    ///
    /// [IETF-RFC7389]: https://tools.ietf.org/html/rfc7390#section-6.2
    pub const APPLICATION_COAP_GROUP_JSON: ContentFormat = ContentFormat(256);

    /// From RFC-ietf-core-object-security-16
    pub const APPLICATION_OSCORE: ContentFormat = ContentFormat(10001);

    /// Same as `application/json`, but with *deflate* compression.
    pub const APPLICATION_JSON_DEFLATE: ContentFormat = ContentFormat(11050);

    /// Same as `application/cbor`, but with *deflate* compression.
    pub const APPLICATION_CBOR_DEFLATE: ContentFormat = ContentFormat(11060);

    /// Returns the MIME name of this content format as a `&'static str`, if possible.
    pub fn static_name(self) -> Option<&'static str> {
        Some(match self {
            Self::TEXT_PLAIN_UTF8 => "text/plain;charset=utf-8",
            Self::APPLICATION_LINK_FORMAT => "application/link-format",
            Self::APPLICATION_XML => "application/xml",
            Self::APPLICATION_OCTET_STREAM => "application/octet-stream",
            Self::APPLICATION_EXI => "application/exi",
            Self::APPLICATION_JSON => "application/json",
            Self::APPLICATION_CBOR => "application/cbor",
            Self::APPLICATION_COSE_COSE_ENCRYPT0 => "application/cose;cose-type=\"cose-encrypt0\"",
            Self::APPLICATION_COSE_COSE_MAC0 => "application/cose;cose-type=\"cose-mac0\"",
            Self::APPLICATION_COSE_COSE_SIGN1 => "application/cose;cose-type=\"cose-sign1\"",
            Self::APPLICATION_COSE_COSE_ENCRYPT => "application/cose;cose-type=\"cose-encrypt\"",
            Self::APPLICATION_COSE_COSE_MAC => "application/cose;cose-type=\"cose-mac\"",
            Self::APPLICATION_COSE_COSE_SIGN => "application/cose;cose-type=\"cose-sign\"",
            Self::APPLICATION_COSE_KEY => "application/cose-key",
            Self::APPLICATION_COSE_KEY_SET => "application/cose-key-set",

            Self::APPLICATION_JSON_PATCH_JSON => "application/json-patch+json",
            Self::APPLICATION_MERGE_PATCH_JSON => "application/merge-patch+json",
            Self::APPLICATION_CWT => "application/cwt",

            Self::APPLICATION_SENML_JSON => "application/senml+json",
            Self::APPLICATION_SENSML_JSON => "application/sensml+json",
            Self::APPLICATION_SENML_CBOR => "application/senml+cbor",
            Self::APPLICATION_SENSML_CBOR => "application/sensml+cbor",
            Self::APPLICATION_SENML_EXI => "application/senml+exi",
            Self::APPLICATION_SENSML_EXI => "application/sensml+exi",
            Self::APPLICATION_SENML_XML => "application/senml+xml",
            Self::APPLICATION_SENSML_XML => "application/sensml+xml",

            Self::APPLICATION_COAP_GROUP_JSON => "application/coap-group+json",

            Self::APPLICATION_OSCORE => "application/oscore",

            Self::APPLICATION_JSON_DEFLATE => "application/json;deflate",
            Self::APPLICATION_CBOR_DEFLATE => "application/cbor;deflate",
            _ => return None,
        })
    }

    /// Returns a MIME name for this content format.
    pub fn name(&self) -> Cow<'static, str> {
        if let Some(name) = self.static_name() {
            Cow::from(name)
        } else {
            Cow::from(self.to_string())
        }
    }

    /// Experimental.
    #[doc(hidden)]
    pub fn is_deflated(self) -> Option<ContentFormat> {
        if self.0 >= 11000 && self.0 <= 11500 {
            Some(ContentFormat(self.0 - 11000))
        } else {
            None
        }
    }

    /// Returns true if this content format is known to contain UTF8.
    pub fn is_utf8(self) -> bool {
        match self {
            Self::TEXT_PLAIN_UTF8 => true,
            Self::APPLICATION_LINK_FORMAT => true,
            _ => self.is_xml() || self.is_json(),
        }
    }

    /// Returns true if this content format is known to contain JSON.
    pub fn is_json(self) -> bool {
        match self {
            Self::APPLICATION_JSON => true,
            Self::APPLICATION_JSON_PATCH_JSON => true,
            Self::APPLICATION_MERGE_PATCH_JSON => true,
            Self::APPLICATION_SENML_JSON => true,
            Self::APPLICATION_SENSML_JSON => true,
            Self::APPLICATION_COAP_GROUP_JSON => true,

            _ => false,
        }
    }

    /// Returns true if this content format is known to contain XML.
    pub fn is_xml(self) -> bool {
        match self {
            Self::APPLICATION_XML => true,
            Self::APPLICATION_SENML_XML => true,
            Self::APPLICATION_SENSML_XML => true,
            _ => false,
        }
    }

    /// Returns true if this content format is known to contain EXI.
    pub fn is_exi(self) -> bool {
        match self {
            Self::APPLICATION_EXI => true,
            Self::APPLICATION_SENML_EXI => true,
            Self::APPLICATION_SENSML_EXI => true,
            _ => false,
        }
    }

    /// Returns true if this content format is known to contain CBOR.
    pub fn is_cbor(self) -> bool {
        match self {
            Self::APPLICATION_CBOR => true,
            Self::APPLICATION_CWT => true,
            Self::APPLICATION_SENML_CBOR => true,
            Self::APPLICATION_SENSML_CBOR => true,
            Self::APPLICATION_OSCORE => true,
            _ => false,
        }
    }
}

impl core::fmt::Display for ContentFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(n) = self.static_name() {
            f.write_str(n)
        } else {
            write!(f, "application/x-coap-{}", self.0)
        }
    }
}
