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

//! Crate providing supporting proc-macros for the `async-coap-uri` crate.
//!
//! **Don't use this crate directly**, use `async-coap-uri`.

#![doc(html_no_source)]

extern crate proc_macro;

#[macro_use]
extern crate lazy_static;

use crate::proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;
use quote::quote;
use regex::Regex;
use syn;

mod unescape_uri;
use unescape_uri::UnescapeUri;

lazy_static! {
    // Splits full URI string into "scheme", "heir-part", "query", and "fragment"
    //      scheme    = $2
    //      authority = $4
    //      path      = $5
    //      query     = $7
    //      fragment  = $9
    //
    // One difference from the regex given in RFC3986 is that this one
    // prohibits the '%' character from appearing in the scheme, which is illegal.
    pub(crate) static ref RFC3986_APPENDIX_B: Regex = Regex::new(r#"^(([^:/?#%]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?$"#)
        .expect("RFC3986_APPENDIX_B");

    //  * `http://example.com/test/path?query#fragment`
    //      * $1 = `http://example.com`
    //      * $2 = `http:`
    //      * $3 = `//example.com`
    //      * $4 = `/test/path?query#fragment`
    //      * $5 = `/test/path?query`
    //      * $6 = `#fragment`
    pub(crate) static ref URI_AUTHORITY_VS_REST: Regex = Regex::new(r#"^(([^:/?#]+:)(//[^/?#]*)?)?(([^#]*)(#.*)?)$"#)
        .expect("URI_AUTHORITY_VS_REST");

    pub(crate) static ref URI_CHECK_SCHEME: Regex = Regex::new(r#"^[A-Za-z][-+.A-Za-z0-9]*$"#)
        .expect("URI_CHECK_SCHEME");

    // Splits the authority into "userinfo", "host", and "port"
    pub(crate) static ref URI_AUTHORITY: Regex = Regex::new(r#"^(([^@/?#]+)@)?([^\[\]:]+|\[[^\]]+\])(:([0-9]+))?$"#)
        .expect("URI_AUTHORITY");
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum Error {
    #[allow(unused)]
    EncodingError,
    MalformedStructure,
    MalformedScheme,
    Degenerate,
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::EncodingError => f.write_str("Encoding Error"),
            Error::MalformedStructure => f.write_str("The structure of the URI is not recognized."),
            Error::MalformedScheme => f.write_str("The scheme of the URI is malformed."),
            Error::Degenerate => {
                f.write_str("This relative reference could be confused with a URI.")
            }
        }
    }
}

fn assert_uri_str(uri_str: &str) -> Result<(), Error> {
    let captures = RFC3986_APPENDIX_B
        .captures(uri_str)
        .ok_or(Error::MalformedStructure)?;

    let has_scheme = captures.get(2).is_some();
    let has_authority = captures.get(4).is_some();

    if !has_scheme && !has_authority {
        return Err(Error::MalformedStructure);
    }

    if let Some(scheme) = captures.get(2) {
        // Do an additional syntax check on the scheme to make sure it is valid.
        URI_CHECK_SCHEME
            .captures(scheme.as_str())
            .ok_or(Error::MalformedScheme)?;
    }

    Ok(())
}

fn assert_rel_ref_str(uri_str: &str) -> Result<(), Error> {
    // We should not be able to parse as a URI.
    assert_uri_str(uri_str)
        .err()
        .map(|_| ())
        .ok_or(Error::Degenerate)?;

    // We should be able to parse as a URI-Reference
    assert_uri_ref_str(uri_str)
}

fn assert_uri_ref_str(uri_str: &str) -> Result<(), Error> {
    // Not sure what additional checks to do in this case.
    RFC3986_APPENDIX_B
        .captures(uri_str)
        .ok_or(Error::MalformedStructure)?;

    Ok(())
}

fn string_literal_from_token_stream(input: TokenStream) -> String {
    use syn::LitStr;

    if let Some(nom) = syn::parse::<LitStr>(input.clone()).ok() {
        return nom.value();
    }

    panic!("Expected string literal, got {:?}", input);
}

#[proc_macro_hack]
pub fn assert_uri_literal(input: TokenStream) -> TokenStream {
    let uri_str = string_literal_from_token_stream(input);

    if let Some(err_pos) = UnescapeUri::new(&uri_str).first_error() {
        panic!("Malformed percent encoding at index {}", err_pos);
    }

    if let Err(err) = assert_uri_str(&uri_str) {
        panic!("Malformed uri literal; {:?}", err);
    }

    let gen = quote! { () };
    gen.into()
}

#[proc_macro_hack]
pub fn assert_rel_ref_literal(input: TokenStream) -> TokenStream {
    let uri_str = string_literal_from_token_stream(input);

    if let Some(err_pos) = UnescapeUri::new(&uri_str).first_error() {
        panic!("Malformed percent encoding at index {}", err_pos);
    }

    if let Err(err) = assert_rel_ref_str(&uri_str) {
        panic!("Malformed rel_ref literal; {:?}", err);
    }

    let gen = quote! { () };
    gen.into()
}

#[proc_macro_hack]
pub fn assert_uri_ref_literal(input: TokenStream) -> TokenStream {
    let uri_str = string_literal_from_token_stream(input);

    if let Some(err_pos) = UnescapeUri::new(&uri_str).first_error() {
        panic!("Malformed percent encoding at index {}", err_pos);
    }

    if let Err(err) = assert_uri_ref_str(&uri_str) {
        panic!("Malformed uri_ref literal; {:?}", err);
    }

    let gen = quote! { () };
    gen.into()
}

#[cfg(test)]
mod test {
    use super::*;

    fn check_uri_str(uri_str: &str) -> Result<(), Error> {
        if let Some(_) = UnescapeUri::new(uri_str).first_error() {
            return Err(Error::EncodingError);
        }
        assert_uri_str(uri_str)
    }

    fn check_rel_ref_str(uri_str: &str) -> Result<(), Error> {
        if let Some(_) = UnescapeUri::new(uri_str).first_error() {
            return Err(Error::EncodingError);
        }
        assert_rel_ref_str(uri_str)
    }

    fn check_uri_ref_str(uri_str: &str) -> Result<(), Error> {
        if let Some(_) = UnescapeUri::new(uri_str).first_error() {
            return Err(Error::EncodingError);
        }
        assert_uri_ref_str(uri_str)
    }

    #[test]
    fn test_uri() {
        assert_eq!(check_uri_str("g:a/b/c"), Ok(()));
        assert_eq!(check_uri_str("g+z://a/b/c"), Ok(()));
        assert_eq!(check_uri_str("//a/b/c"), Ok(()));
        assert_eq!(check_uri_str("a/b/c"), Err(Error::MalformedStructure));
        assert_eq!(check_uri_str("g$:a/b/c"), Err(Error::MalformedScheme));
        assert_eq!(check_uri_str("g%:a/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_str("g:%aa/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_str("g:%00/b/c"), Err(Error::EncodingError));
    }

    #[test]
    fn test_rel_ref() {
        assert_eq!(check_rel_ref_str("/a/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("a/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("g:a/b/c"), Err(Error::Degenerate));
        assert_eq!(check_rel_ref_str("g%3Aa/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("./g:a/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("//a/b/c"), Err(Error::Degenerate));
        assert_eq!(check_rel_ref_str("/.//a/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("g$:a/b/c"), Ok(()));
        assert_eq!(check_rel_ref_str("g%:a/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("g:%aa/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("g:%00/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("%a/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("%aa/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("%00/b/c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("a/ /c"), Err(Error::EncodingError));
        assert_eq!(check_rel_ref_str("a/\n/c"), Err(Error::EncodingError));
    }

    #[test]
    fn test_uri_ref() {
        assert_eq!(check_uri_ref_str("/a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("g:a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("g%3Aa/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("./g:a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("//a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("/.//a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("g$:a/b/c"), Ok(()));
        assert_eq!(check_uri_ref_str("g%:a/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("g:%aa/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("g:%00/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("%a/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("%aa/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("%00/b/c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("a/ /c"), Err(Error::EncodingError));
        assert_eq!(check_uri_ref_str("a/\n/c"), Err(Error::EncodingError));
    }
}
