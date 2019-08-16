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

use regex::Regex;
lazy_static! {
    /// Splits full URI string into "scheme", "heir-part", "query", and "fragment"
    ///
    /// * scheme    = $2
    /// * authority = $4
    /// * path      = $5
    /// * query     = $7
    /// * fragment  = $9
    ///
    /// One difference from the regex given in RFC3986 is that this one
    /// prohibits the '%' character from appearing in the scheme, which is illegal.
    pub(crate) static ref RFC3986_APPENDIX_B: Regex = Regex::new(r#"^(([^:/?#%]+):)?(//([^/?#]*))?([^?#]*)(\?([^#]*))?(#(.*))?$"#)
        .expect("RFC3986_APPENDIX_B");

    /// Regex for splitting the URI side (scheme + authority) from the rest of the URI.
    ///
    /// * `http://example.com/test/path?query#fragment`
    ///   * $1 = `http://example.com`
    ///   * $2 = `http:`
    ///   * $3 = `//example.com`
    ///   * $4 = `/test/path?query#fragment`
    ///   * $5 = `/test/path?query`
    ///   * $6 = `#fragment`
    pub(crate) static ref URI_AUTHORITY_VS_REST: Regex = Regex::new(r#"^(([^:/?#]+:)(//[^/?#]*)?)?(([^#]*)(#.*)?)$"#)
        .expect("URI_AUTHORITY_VS_REST");

    /// Regex for verifying that a URI scheme is well-formed.
    pub(crate) static ref URI_CHECK_SCHEME: Regex = Regex::new(r#"^[A-Za-z][-+.A-Za-z0-9]*$"#)
        .expect("URI_CHECK_SCHEME");

    /// Splits the authority into "userinfo", "host", and "port"
    pub(crate) static ref URI_AUTHORITY: Regex = Regex::new(r#"^(([^@/?#]+)@)?([^\[\]:]+|\[[^\]]+\])(:([0-9]+))?$"#)
        .expect("URI_AUTHORITY");
}
