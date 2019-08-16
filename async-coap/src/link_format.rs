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

//! Mechanisms and constants for encoding and decoding [IETF-RFC6690 CoAP link-formats].
//!
//! [IETF-RFC6690 CoAP link-formats]: https://tools.ietf.org/html/rfc6690

use super::*;
use crate::uri::AnyUriRef;
use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::iter::FusedIterator;

/// Relation Type.
///
/// From [IETF-RFC8288], [Section 3.3]:
///
/// > The relation type of a link conveyed in the Link header field is
/// > conveyed in the "rel" parameter's value.  The rel parameter MUST be
/// > present but MUST NOT appear more than once in a given link-value;
/// > occurrences after the first MUST be ignored by parsers.
/// >
/// > The rel parameter can, however, contain multiple link relation types.
/// > When this occurs, it establishes multiple links that share the same
/// > context, target, and target attributes.
/// >
/// > The ABNF for the rel parameter values is:
/// >
/// > ```abnf
/// >     relation-type *( 1*SP relation-type )
/// > ```
/// >
/// > where:
/// >
/// > ```abnf
/// >     relation-type  = reg-rel-type / ext-rel-type
/// >     reg-rel-type   = LOALPHA *( LOALPHA / DIGIT / "." / "-" )
/// >     ext-rel-type   = URI ; Section 3 of [RFC3986]
/// > ```
/// >
/// > Note that extension relation types are REQUIRED to be absolute URIs
/// > in Link header fields and MUST be quoted when they contain characters
/// > not allowed in tokens, such as a semicolon (";") or comma (",") (as
/// > these characters are used as delimiters in the header field itself).
///
/// Optional in [IETF-RFC6690] link format resources.
///
/// [IETF-RFC8288]: https://tools.ietf.org/html/rfc8288
/// [Section 3.3]: https://tools.ietf.org/html/rfc8288#section-3.3
/// [IETF-RFC6690]: https://tools.ietf.org/html/rfc6690
pub const LINK_ATTR_REL: &'static str = "rel";

/// Anchor attribute.
///
/// Provides an override of the document context URI when parsing relative URIs
/// in the links. The value itself may be a relative URI, which is evaluated against the document
/// context URI.
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.2">RFC8288, Section 3.2</a>
pub const LINK_ATTR_ANCHOR: &'static str = "anchor";

/// A hint indicating what the language of the result of dereferencing the link should be.
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.4.1">RFC8288, Section 3.4.1</a>
pub const LINK_ATTR_HREFLANG: &'static str = "hreflang";

/// Media Attribute. Used to indicate intended destination medium or media for style information.
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.4.1">RFC8288, Section 3.4.1</a>
pub const LINK_ATTR_MEDIA: &'static str = "media";

/// Human-readable label describing the resource.
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.4.1">RFC8288, Section 3.4.1</a>
pub const LINK_ATTR_TITLE: &'static str = "title";

/// Human-readable label describing the resource, along with language information.
///
/// Is is typically formatted as `"utf-8'<LANG_CODE&>'<TITLE_TEXT>"`. For example:
///
/// * `"utf-8'en'£ rates"`</code>
///
/// Note that since <a href="https://tools.ietf.org/html/rfc6690">RFC6690</a> requires the link
/// format serialization to always be in UTF-8 format, the value of this attribute MUST ALWAYS
/// start with either the string <code>utf-8</code> or <code>UTF-8</code> and MUST NOT be
/// percent-encoded.
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.4.1">RFC8288, Section 3.4.1</a>
/// * <a href="https://tools.ietf.org/html/rfc8187">RFC8187</a>
pub const LINK_ATTR_TITLE_STAR: &'static str = "title*";

/// MIME content type attribute.
///
/// This attribute should be avoided in favor of [`LINK_ATTR_CONTENT_FORMAT`].
///
/// * <a href="https://tools.ietf.org/html/rfc8288#section-3.4.1">RFC8288, Section 3.4.1</a>
#[doc(hidden)]
pub const LINK_ATTR_TYPE: &'static str = "type";

/// Resource Type Attribute.
///
/// The Resource Type `rt` attribute is an opaque string used to assign
/// an application-specific semantic type to a resource. One can think of this as a noun
/// describing the resource.
///
/// * <a href="https://tools.ietf.org/html/rfc6690#section-3.1">RFC6690, Section 3.1</a>
pub const LINK_ATTR_RESOURCE_TYPE: &'static str = "rt";

/// Interface Description Attribute.
///
/// The Interface Description `if` attribute is an opaque string
/// used to provide a name or URI indicating a specific interface definition used to interact
/// with the target resource. One can think of this as describing verbs usable on a resource.
///
/// * <a href="https://tools.ietf.org/html/rfc6690#section-3.2">RFC6690, Section 3.2</a>
///
pub const LINK_ATTR_INTERFACE_DESCRIPTION: &'static str = "if";

/// The estimated maximum size of the fetched resource.
///
/// The maximum size estimate attribute `sz`
/// gives an indication of the maximum size of the resource representation returned by performing
/// a GET on the target URI. For links to CoAP resources, this attribute is not expected to be
/// included for small resources that can comfortably be carried in a single Maximum Transmission
/// Unit (MTU) but SHOULD be included for resources larger than that. The maximum size estimate
/// attribute MUST NOT appear more than once in a link.
///
/// * <a href="https://tools.ietf.org/html/rfc6690#section-3.3">RFC6690, Section 3.3</a>
pub const LINK_ATTR_MAXIMUM_SIZE_ESTIMATE: &'static str = "sz";

/// The value of this resource expressed as a human-readable string. Must be less than 63 bytes.
pub const LINK_ATTR_VALUE: &'static str = "v";

/// Content-Format Code(s).
///
/// Space-separated list of content type integers appropriate for being
/// specified in an Accept option.
///
/// * <a href="https://tools.ietf.org/html/rfc7252#section-7.2.1">RFC7252, Section 7.2.1</a>
pub const LINK_ATTR_CONTENT_FORMAT: &'static str = "ct";

/// Identifies this resource as observable if present.
///
/// * <a href="https://tools.ietf.org/html/rfc7641#section-6">RFC7641, Section 6</a>
pub const LINK_ATTR_OBSERVABLE: &'static str = "obs";

/// Name of the endpoint, max 63 bytes.
///
/// * <a href="https://goo.gl/6e2s7C#section-5.3">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_ENDPOINT_NAME: &'static str = "ep";

/// Lifetime of the registration in seconds. Valid values are between 60-4294967295, inclusive.
///
/// * <a href="https://goo.gl/6e2s7C#section-5.3">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_REGISTRATION_LIFETIME: &'static str = "lt";

/// Sector to which this endpoint belongs. Must be less than 63 bytes.
///
/// * <a href="https://goo.gl/6e2s7C#section-5.3">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_SECTOR: &'static str = "d";

/// The scheme, address and point and path at which this server is available.
///
/// MUST be a valid URI.
///
/// * <a href="https://goo.gl/6e2s7C#section-5.3">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_REGISTRATION_BASE_URI: &'static str = "base";

/// Name of a group in this RD. Must be less than 63 bytes.
///
/// * <a href="https://goo.gl/6e2s7C#section-6.1">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_GROUP_NAME: &'static str = "gp";

/// Semantic name of the endpoint. Must be less than 63 bytes.
///
/// * <a href="https://goo.gl/6e2s7C#section-10.3.1">draft-ietf-core-resource-directory-14</a>
pub const LINK_ATTR_ENDPOINT_TYPE: &'static str = "et";

/// Error type for parsing a link format.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorLinkFormat {
    /// An error was encountered while parsing the link format.
    ParseError,
}

impl From<ErrorLinkFormat> for crate::Error {
    fn from(_: ErrorLinkFormat) -> Self {
        Error::ParseFailure
    }
}

const QUOTE_ESCAPE_CHAR: char = '\\';
const ATTR_SEPARATOR_CHAR: char = ';';
const LINK_SEPARATOR_CHAR: char = ',';

/// Parsing iterator which parses a string formatted as an [IETF-RFC6690 CoAP link-format].
///
/// As successful parsing is performed, this iterator emits a tuple inside of a `Result::Ok`.
/// The tuple contains a string slice for the link and a [`LinkAttributeParser`] to provide
/// access to the link attributes for that link.
///
/// Parsing errors are emitted as a `Result::Err` and are of the error type [`ErrorLinkFormat`].
///
/// [IETF-RFC6690 CoAP link-format]: https://tools.ietf.org/html/rfc6690
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LinkFormatParser<'a> {
    pub(super) inner: &'a str,
}

impl<'a> LinkFormatParser<'a> {
    /// Creates a new instance of `LinkFormatParser` for the given string slice.
    pub fn new(inner: &'a str) -> LinkFormatParser<'a> {
        LinkFormatParser { inner }
    }
}

impl<'a> Iterator for LinkFormatParser<'a> {
    /// (uri-ref, link-attribute-iterator)
    type Item = Result<(&'a str, LinkAttributeParser<'a>), ErrorLinkFormat>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_empty() {
            return None;
        }

        let mut iter = self.inner.chars();

        // Proceed through whitespace until we get a '<'.
        loop {
            match iter.next() {
                Some(c) if c.is_ascii_whitespace() => continue,
                Some('<') => break,
                Some(_) => {
                    self.inner = "";
                    return Some(Err(ErrorLinkFormat::ParseError));
                }
                None => {
                    self.inner = "";
                    return None;
                }
            }
        }

        let link_ref = iter.as_str();

        // Proceed through characters until we get a '>'.
        while let Some(c) = iter.next() {
            if c == '>' {
                break;
            }
        }

        let link_len = iter.as_str().as_ptr() as usize - link_ref.as_ptr() as usize;

        let link_ref = (&link_ref[..link_len]).trim_end_matches('>');

        let mut attr_keys = iter.as_str();

        // Skip to the end of the attributes. We leave the
        // actual attribute parsing to `LinkAttributeParser`.
        loop {
            match iter.next() {
                Some(LINK_SEPARATOR_CHAR) | None => {
                    break;
                }
                Some(c) if c == '"' => {
                    // Handle quotes.
                    loop {
                        match iter.next() {
                            Some('"') | None => break,
                            Some(QUOTE_ESCAPE_CHAR) => {
                                // Slashes always escape the next character,
                                // since we are scanning and not parsing we
                                // just skip it.
                                iter.next();
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }

        let attr_len = iter.as_str().as_ptr() as usize - attr_keys.as_ptr() as usize;
        attr_keys = (&attr_keys[..attr_len]).trim_end_matches(LINK_SEPARATOR_CHAR);

        self.inner = iter.as_str();
        return Some(Ok((
            link_ref,
            LinkAttributeParser {
                inner: attr_keys.trim_matches(ATTR_SEPARATOR_CHAR),
            },
        )));
    }
}

/// Parsing iterator which parses link attributes for [IETF-RFC6690 CoAP link-format] processing.
///
/// This iterator is emitted by [`LinkFormatParser`] while parsing a CoAP link-format. It emits
/// a tuple for each attribute, with the first item being a string slice for the attribute key
/// and the second item being an [`Unquote`] iterator for obtaining the value. A `String` or
/// `Cow<str>` version of the value can be easily obtained by calling `to_string()` or `to_cow()`
/// on the [`Unquote`] instance.
///
/// This iterator is permissive and makes a best-effort to parse the link attributes and does not
/// emit errors while parsing.
///
/// [IETF-RFC6690 CoAP link-format]: https://tools.ietf.org/html/rfc6690
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LinkAttributeParser<'a> {
    pub(super) inner: &'a str,
}

impl<'a> Iterator for LinkAttributeParser<'a> {
    /// (key_ref: &str, value-ref: Unquote)
    type Item = (&'a str, Unquote<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_empty() {
            return None;
        }

        let mut iter = self.inner.chars();

        // Skip to the end of the attribute.
        loop {
            match iter.next() {
                Some(ATTR_SEPARATOR_CHAR) | None => {
                    break;
                }
                Some(c) if c == '"' => {
                    // Handle quotes.
                    loop {
                        match iter.next() {
                            Some('"') | None => {
                                break;
                            }
                            Some(QUOTE_ESCAPE_CHAR) => {
                                iter.next();
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }

        let attr_len = iter.as_str().as_ptr() as usize - self.inner.as_ptr() as usize;
        let attr_str = &self.inner[..attr_len];

        self.inner = iter.as_str();

        let attr_str = attr_str.trim_end_matches(ATTR_SEPARATOR_CHAR);

        let (key, value) = if let Some(i) = attr_str.find('=') {
            let (key, value) = attr_str.split_at(i);

            (key, &value[1..])
        } else {
            (attr_str, "")
        };

        return Some((key.trim(), Unquote::new(value.trim())));
    }
}

/// Character iterator which decodes a [IETF-RFC2616] [`quoted-string`].
/// Used by [`LinkAttributeParser`].
///
/// From [IETF-RFC2616] Section 2.2:
///
/// > A string of text is parsed as a single word if it is quoted using
/// > double-quote marks.
/// >
/// > ```abnf
/// >     quoted-string  = ( <"> *(qdtext | quoted-pair ) <"> )
/// >     qdtext         = <any TEXT except <">>
/// > ```
/// >
/// > The backslash character ('\\') MAY be used as a single-character
/// > quoting mechanism only within quoted-string and comment constructs.
/// >
/// > ```abnf
/// >     quoted-pair    = "\" CHAR
/// > ```
///
/// [IETF-RFC2616]: https://tools.ietf.org/html/rfc2616
/// [`quoted-string`]: https://tools.ietf.org/html/rfc2616#section-2.2
#[derive(Clone, Debug)]
pub struct Unquote<'a> {
    inner: std::str::Chars<'a>,
    state: UnquoteState,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum UnquoteState {
    NotStarted,
    NotQuoted,
    Quoted,
}

impl<'a> Eq for Unquote<'a> {}

impl<'a> PartialEq for Unquote<'a> {
    fn eq(&self, other: &Self) -> bool {
        let self_s = self.inner.as_str();
        let other_s = other.inner.as_str();
        self.state == other.state
            && self_s.as_ptr() == other_s.as_ptr()
            && self_s.len() == other_s.len()
    }
}

impl<'a> From<Unquote<'a>> for Cow<'a, str> {
    fn from(iter: Unquote<'a>) -> Self {
        iter.to_cow()
    }
}

impl<'a> FusedIterator for Unquote<'a> {}

impl<'a> Display for Unquote<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

impl<'a> Unquote<'a> {
    /// Creates a new instance of the `Unquote` iterator from `quoted_str`.
    pub fn new(quoted_str: &'a str) -> Unquote<'a> {
        Unquote {
            inner: quoted_str.chars(),
            state: UnquoteState::NotStarted,
        }
    }

    /// Converts a fresh, unused instance of `Unquote` into the underlying raw string slice.
    ///
    /// Calling this method will panic if `next()` has been called.
    pub fn into_raw_str(self) -> &'a str {
        assert_eq!(self.state, UnquoteState::NotStarted);
        self.inner.as_str()
    }

    /// Returns the unquoted version of this string as a copy-on-write string.
    pub fn to_cow(&self) -> Cow<'a, str> {
        let str_ref = self.inner.as_str();
        if self.is_quoted() {
            if str_ref.find('\\').is_some() {
                Cow::from(self.to_string())
            } else {
                // String is quoted but has no escapes.
                Cow::from(&str_ref[1..str_ref.len() - 1])
            }
        } else {
            Cow::from(str_ref)
        }
    }

    /// Returns true if the underlying string is quoted, false otherwise.
    pub fn is_quoted(&self) -> bool {
        match self.state {
            UnquoteState::NotStarted => self.inner.as_str().starts_with('"'),
            UnquoteState::NotQuoted => false,
            UnquoteState::Quoted => true,
        }
    }
}

impl<'a> Iterator for Unquote<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match self.state {
                UnquoteState::NotStarted => match self.inner.next() {
                    Some('"') => {
                        self.state = UnquoteState::Quoted;
                        // Go back to the start of the loop so we can hit
                        // our "UnquoteState::Quoted" section below.
                        continue;
                    }
                    c => {
                        self.state = UnquoteState::NotQuoted;
                        c
                    }
                },
                UnquoteState::NotQuoted => self.inner.next(),
                UnquoteState::Quoted => match self.inner.next() {
                    Some('"') => {
                        // We are finished.
                        // Make ourselves empty so we can call ourselves "Fused"
                        self.inner = "".chars();
                        None
                    }
                    Some(QUOTE_ESCAPE_CHAR) => self.inner.next(),
                    c => c,
                },
            };
        }
    }
}

/// Helper for writing [IETF-RFC6690 CoAP link-formats] to anything implementing
/// [`core::fmt::Write`].
///
/// ## Example
///
/// ```
/// use async_coap::prelude::*;
/// use async_coap::LinkFormatWrite;
/// use async_coap::LINK_ATTR_INTERFACE_DESCRIPTION;
///
/// // String implements core::fmt::Write
/// let mut buffer = String::new();
///
/// let mut write = LinkFormatWrite::new(&mut buffer);
///
/// write.link(uri_ref!("/sensor/light"))
///     .attr_quoted(LINK_ATTR_INTERFACE_DESCRIPTION,"sensor")
///     .finish()
///     .expect("Error writing link");
///
/// assert_eq!(&buffer, r#"</sensor/light>;if="sensor""#);
/// ```
///
/// [IETF-RFC6690 CoAP link-formats]: https://tools.ietf.org/html/rfc6690
#[derive(Debug)]
pub struct LinkFormatWrite<'a, T: ?Sized> {
    write: &'a mut T,
    is_first: bool,
    add_newlines: bool,
    error: Option<core::fmt::Error>,
}

impl<'a, T: Write + ?Sized> LinkFormatWrite<'a, T> {
    /// Creates a new instance of `LinkFormatWriter` for a given instance that implements
    /// [`core::fmt::Write`].
    pub fn new(write: &'a mut T) -> LinkFormatWrite<'a, T> {
        LinkFormatWrite {
            write,
            is_first: true,
            add_newlines: false,
            error: None,
        }
    }

    /// Sets whether newlines should be added or not between links, possibly improving
    /// human readability an the expense of a few extra bytes.
    pub fn set_add_newlines(&mut self, add_newlines: bool) {
        self.add_newlines = add_newlines;
    }

    /// Adds a link to the link format and returns [`LinkAttributeWrite`].
    ///
    /// The returned [`LinkAttributeWrite`] instance can then be used to associate
    /// attributes to the link.
    pub fn link<'b, U: AnyUriRef + ?Sized>(
        &'b mut self,
        link: &U,
    ) -> LinkAttributeWrite<'a, 'b, T> {
        if self.is_first {
            self.is_first = false;
        } else if self.error.is_none() {
            self.error = self.write.write_char(LINK_SEPARATOR_CHAR).err();
            if self.add_newlines {
                self.error = self.write.write_str("\n\r").err();
            }
        }

        if self.error.is_none() {
            self.error = self.write.write_char('<').err();
        }

        if self.error.is_none() {
            self.error = write!(self.write, "{}", link.display()).err();
        }

        if self.error.is_none() {
            self.error = self.write.write_char('>').err();
        }

        LinkAttributeWrite(self)
    }

    /// Consumes this [`LinkFormatWrite`] instance, returning any error that
    /// might have occurred during writing.
    pub fn finish(self) -> Result<(), core::fmt::Error> {
        if let Some(e) = self.error {
            Err(e)
        } else {
            Ok(())
        }
    }
}

/// Helper for writing link format attributes; created by calling [`LinkFormatWrite::link`].
#[derive(Debug)]
pub struct LinkAttributeWrite<'a, 'b, T: ?Sized>(&'b mut LinkFormatWrite<'a, T>);

impl<'a, 'b, T: Write + ?Sized> LinkAttributeWrite<'a, 'b, T> {
    /// Prints just the key and an equals sign, prefixed with ';'
    fn internal_attr_key_eq(&mut self, key: &'static str) {
        debug_assert!(key
            .find(|c: char| c.is_ascii_whitespace() || c == '=')
            .is_none());

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_char(ATTR_SEPARATOR_CHAR).err();
        }

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_str(key).err();
        }

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_char('=').err();
        }
    }

    /// Adds an attribute to the link, only quoting the value if it contains
    /// non-ascii-alphanumeric characters.
    pub fn attr(mut self, key: &'static str, value: &str) -> Self {
        if value.find(|c: char| !c.is_ascii_alphanumeric()).is_some() {
            return self.attr_quoted(key, value);
        }

        self.internal_attr_key_eq(key);

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_str(value).err();
        }

        self
    }

    /// Adds an attribute to the link that has u32 value.
    pub fn attr_u32(mut self, key: &'static str, value: u32) -> Self {
        self.internal_attr_key_eq(key);

        if self.0.error.is_none() {
            self.0.error = write!(self.0.write, "{}", value).err();
        }

        self
    }

    /// Adds an attribute to the link that has u16 value.
    pub fn attr_u16(self, key: &'static str, value: u16) -> Self {
        self.attr_u32(key, value as u32)
    }

    /// Adds an attribute to the link, unconditionally quoting the value.
    pub fn attr_quoted(mut self, key: &'static str, value: &str) -> Self {
        self.internal_attr_key_eq(key);

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_char('"').err();
        }

        for c in value.chars() {
            if (c == '"' || c == '\\') && self.0.error.is_none() {
                self.0.error = self.0.write.write_char(QUOTE_ESCAPE_CHAR).err();
            }

            if self.0.error.is_none() {
                self.0.error = self.0.write.write_char(c).err();
            }
        }

        if self.0.error.is_none() {
            self.0.error = self.0.write.write_char('"').err();
        }

        self
    }

    /// Consumes this [`LinkAttributeWrite`] instance, returning any error that
    /// might have occurred during writing.
    pub fn finish(self) -> Result<(), core::fmt::Error> {
        if let Some(e) = self.0.error {
            Err(e)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn link_format_write_1() {
        let mut buffer = String::new();

        let mut write = LinkFormatWrite::new(&mut buffer);

        write
            .link(uri_ref!("/sensor/light"))
            .attr_quoted(LINK_ATTR_INTERFACE_DESCRIPTION, "sensor")
            .finish()
            .expect("Write link failed");

        assert_eq!(write.finish(), Ok(()));

        assert_eq!(&buffer, r#"</sensor/light>;if="sensor""#);
    }

    #[test]
    fn link_format_write_2() {
        let mut buffer = String::new();

        let mut write = LinkFormatWrite::new(&mut buffer);

        write
            .link(uri_ref!("/sensor/light"))
            .attr_quoted(LINK_ATTR_INTERFACE_DESCRIPTION, "sensor")
            .attr(LINK_ATTR_TITLE, "My Light")
            .finish()
            .expect("Write link failed");

        write
            .link(uri_ref!("/sensor/temp"))
            .attr_quoted(LINK_ATTR_INTERFACE_DESCRIPTION, "sensor")
            .attr(LINK_ATTR_TITLE, "My Thermostat")
            .attr_u32(LINK_ATTR_VALUE, 20)
            .finish()
            .expect("Write link failed");

        assert_eq!(write.finish(), Ok(()));

        assert_eq!(&buffer, r#"</sensor/light>;if="sensor";title="My Light",</sensor/temp>;if="sensor";title="My Thermostat";v=20"#);
    }

    #[test]
    fn unquote_1() {
        let unquote = Unquote::new(r#""sensor""#);

        assert_eq!(&unquote.to_string(), "sensor");
    }

    #[test]
    fn unquote_2() {
        let unquote = Unquote::new("sensor");

        assert_eq!(&unquote.to_string(), "sensor");
    }

    #[test]
    fn unquote_3() {
        let unquote = Unquote::new(r#""the \"foo\" bar""#);

        assert_eq!(&unquote.to_string(), r#"the "foo" bar"#);
    }

    #[test]
    fn unquote_4() {
        let unquote = Unquote::new(r#""\"the foo bar\"""#);

        assert_eq!(&unquote.to_string(), r#""the foo bar""#);
    }

    #[test]
    fn unquote_5() {
        let unquote = Unquote::new(r#""the \\\"foo\\\" bar""#);

        assert_eq!(&unquote.to_string(), r#"the \"foo\" bar"#);
    }

    #[test]
    fn link_format_parser_1() {
        let link_format = r#"</sensors>;ct=40"#;

        let mut parser = LinkFormatParser::new(link_format);

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                eprintln!("attr_iter: {:?}", attr_iter);
                assert_eq!(link, "/sensors");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("ct", r#"40"#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        assert_eq!(parser.next(), None);
    }

    #[test]
    fn link_format_parser_2() {
        let link_format = r#"
            </sensors/temp>;if="sensor",
            </sensors/light>;if="sensor""#;

        let mut parser = LinkFormatParser::new(link_format);

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                eprintln!("attr_iter: {:?}", attr_iter);
                assert_eq!(link, "/sensors/temp");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("if", r#""sensor""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                eprintln!("attr_iter: {:?}", attr_iter);
                assert_eq!(link, "/sensors/light");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("if", r#""sensor""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        assert_eq!(parser.next(), None);
    }

    #[test]
    fn link_format_parser_3() {
        let link_format = r#"</sensors>;ct=40;title="Sensor Index",
   </sensors/temp>;rt="temperature-c";if="sensor",
   </sensors/light>;rt="light-lux";if="sensor",
   <http://www.example.com/sensors/t123>;anchor="/sensors/temp"
   ;rel="describedby",
   </t>;anchor="/sensors/temp";rel="alternate""#;

        let mut parser = LinkFormatParser::new(link_format);

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                assert_eq!(link, "/sensors");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("ct", r#"40"#))
                );
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("title", r#""Sensor Index""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                assert_eq!(link, "/sensors/temp");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("rt", r#""temperature-c""#))
                );
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("if", r#""sensor""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                assert_eq!(link, "/sensors/light");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("rt", r#""light-lux""#))
                );
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("if", r#""sensor""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                assert_eq!(link, "http://www.example.com/sensors/t123");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("anchor", r#""/sensors/temp""#))
                );
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("rel", r#""describedby""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        match parser.next() {
            Some(Ok((link, mut attr_iter))) => {
                assert_eq!(link, "/t");
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("anchor", r#""/sensors/temp""#))
                );
                assert_eq!(
                    attr_iter.next().map(|attr| (attr.0, attr.1.into_raw_str())),
                    Some(("rel", r#""alternate""#))
                );
                assert_eq!(attr_iter.next(), None);
            }
            x => {
                panic!("{:?}", x);
            }
        }

        assert_eq!(parser.next(), None);
    }
}
