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

#[test]
fn uri_regex() {
    {
        let captures = RFC3986_APPENDIX_B
            .captures("http://www.ics.uci.edu/pub/ietf/uri/#Related")
            .expect("Should have matched regex");
        assert_eq!("http:", &captures[1]);
        assert_eq!("http", &captures[2]);
        assert_eq!("//www.ics.uci.edu", &captures[3]);
        assert_eq!("www.ics.uci.edu", &captures[4]);
        assert_eq!("/pub/ietf/uri/", &captures[5]);
        assert_eq!(None, captures.get(6));
        assert_eq!(None, captures.get(7));
        assert_eq!("#Related", &captures[8]);
        assert_eq!("Related", &captures[9]);
    }
    {
        let captures = RFC3986_APPENDIX_B
            .captures("coap+sms://username:password@example.com:1234?query&d=3#frag")
            .expect("Should have matched regex");
        assert_eq!("coap+sms:", &captures[1]);
        assert_eq!("coap+sms", &captures[2]);
        assert_eq!("//username:password@example.com:1234", &captures[3]);
        assert_eq!("username:password@example.com:1234", &captures[4]);
        assert_eq!("", &captures[5]);
        assert_eq!("?query&d=3", &captures[6]);
        assert_eq!("query&d=3", &captures[7]);
        assert_eq!("#frag", &captures[8]);
        assert_eq!("frag", &captures[9]);
    }
    {
        let captures = RFC3986_APPENDIX_B
            .captures("uid:a-strange-id?q#f")
            .expect("Should have matched regex");
        assert_eq!("uid:", &captures[1]);
        assert_eq!("uid", &captures[2]);
        assert_eq!(None, captures.get(3));
        assert_eq!(None, captures.get(4));
        assert_eq!("a-strange-id", &captures[5]);
        assert_eq!("?q", &captures[6]);
        assert_eq!("q", &captures[7]);
        assert_eq!("#f", &captures[8]);
        assert_eq!("f", &captures[9]);
    }
    {
        let captures = RFC3986_APPENDIX_B
            .captures("path?q#f?b#")
            .expect("Should have matched regex");
        assert_eq!(None, captures.get(1));
        assert_eq!(None, captures.get(2));
        assert_eq!(None, captures.get(3));
        assert_eq!(None, captures.get(4));
        assert_eq!("path", &captures[5]);
        assert_eq!("?q", &captures[6]);
        assert_eq!("q", &captures[7]);
        assert_eq!("#f?b#", &captures[8]);
        assert_eq!("f?b#", &captures[9]);
    }

    {
        let captures = URI_AUTHORITY
            .captures("username:password@example.com:1234")
            .expect("Should have matched regex");
        assert_eq!("username:password@", &captures[1]);
        assert_eq!("username:password", &captures[2]);
        assert_eq!("example.com", &captures[3]);
        assert_eq!(":1234", &captures[4]);
        assert_eq!("1234", &captures[5]);
    }
    {
        let captures = URI_AUTHORITY
            .captures("username@[2000::1]:1234")
            .expect("Should have matched regex");
        assert_eq!("username@", &captures[1]);
        assert_eq!("username", &captures[2]);
        assert_eq!("[2000::1]", &captures[3]);
        assert_eq!(":1234", &captures[4]);
        assert_eq!("1234", &captures[5]);
    }
    {
        let captures = URI_AUTHORITY
            .captures("example.com")
            .expect("Should have matched regex");
        assert_eq!(None, captures.get(1));
        assert_eq!(None, captures.get(2));
        assert_eq!("example.com", &captures[3]);
        assert_eq!(None, captures.get(4));
        assert_eq!(None, captures.get(5));
    }
}
