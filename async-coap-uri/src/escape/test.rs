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

use crate::*;

#[test]
fn escaped_path_starts_with_1() {
    let s = "bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    assert_eq!(
        s.unescape_uri()
            .skip_slashes()
            .starts_with("blåbær/%2F/syltetøy"),
        Some("bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y".len())
    );
}

#[test]
fn escaped_path_starts_with_2() {
    let s = "/bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    assert_eq!(
        s.unescape_uri()
            .skip_slashes()
            .starts_with("blåbær/%2F/syltetøy"),
        None
    );
}

#[test]
fn escaped_path_starts_with_3() {
    let s = "/1/2/3/";
    assert_eq!(s.unescape_uri().skip_slashes().starts_with("/1/"), Some(3));
}

#[test]
fn escaped_path_starts_with_4() {
    let s = "/1/";
    assert_eq!(s.unescape_uri().skip_slashes().starts_with("/1/"), Some(3));
}

#[test]
fn escaped_starts_with_0() {
    let s = "bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    assert_eq!(s.unescape_uri().starts_with("blåbær/%2F/syltetøy"), None);
}

#[test]
fn escaped_starts_with_1() {
    let s = "bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    assert_eq!(
        s.unescape_uri().starts_with("blåbær///syltetøy"),
        Some("bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y".len())
    );
}

#[test]
fn escaped_starts_with_2() {
    let s = "/bl%C3%A5b%C3%A6r/%2F/syltet%C3%B8y/and/on/and/on";
    assert_eq!(s.unescape_uri().starts_with("blåbær///syltetøy"), None);
}

#[test]
fn escaped_starts_with_3() {
    let s = "/1/2/3/";
    assert_eq!(s.unescape_uri().starts_with("/1/"), Some(3));
}

#[test]
fn escaped_starts_with_4() {
    let s = "/1/";
    assert_eq!(s.unescape_uri().starts_with("/1/"), Some(3));
}

#[test]
fn escape_uri_cow_1() {
    let s = "needs-no-escaping";
    let cow = s.escape_uri().to_cow();

    assert_eq!(cow, s);
}

#[test]
fn escape_uri_cow_2() {
    let s = "needs escaping";
    let cow = s.escape_uri().to_cow();

    assert_ne!(cow, s);
    assert_eq!(cow, "needs%20escaping");
}

#[test]
fn unescape_uri_cow_1() {
    let s = "needs-no-unescaping";
    let cow = s.unescape_uri().to_cow();

    assert_eq!(cow, s);
}

#[test]
fn unescape_uri_cow_2() {
    let s = "needs%20unescaping";
    let cow = s.unescape_uri().to_cow();

    assert_ne!(cow, s);
    assert_eq!(cow, "needs unescaping");
}

#[test]
fn unescape_uri_path_cow_1() {
    let s = "needs/no/unescaping";
    let cow = s.unescape_uri().skip_slashes().to_cow();

    assert_eq!(cow, s);
}

#[test]
fn unescape_uri_path_cow_2() {
    let s = "this/%20does%20/need%2Funescaping";
    let cow = s.unescape_uri().skip_slashes().to_cow();

    assert_ne!(cow, s);
    assert_eq!(cow, "this/ does /need%2Funescaping");
}

#[test]
fn try_unescape_uri_cow_1() {
    let s = "needs-no-unescaping";
    let cow = s.unescape_uri().try_to_cow();

    assert_eq!(cow, Ok(Cow::from(s)));
}

#[test]
fn try_unescape_uri_cow_2() {
    let s = "needs%20unescaping";
    let cow = s.unescape_uri().try_to_cow();

    assert_ne!(cow, Ok(Cow::from(s)));
    assert_eq!(cow, Ok(Cow::from("needs unescaping")));
}

#[test]
fn try_unescape_uri_cow_3() {
    let s = "bad%10escaping";
    let cow = s.unescape_uri().try_to_cow();

    assert_eq!(cow.unwrap_err().index, 6);
}

macro_rules! test_escape_unescape {
    ( $NAME:ident, $UNESCAPED:expr, $ESCAPED:expr ) => {
        #[test]
        fn $NAME() {
            assert_eq!(
                &$UNESCAPED.escape_uri().to_string(),
                $ESCAPED,
                "Failed on escape_uri().to_string()"
            );
            assert_eq!(
                &$ESCAPED.unescape_uri().to_string(),
                $UNESCAPED,
                "Failed on unescape_uri().to_string()"
            );
        }
    };
}

macro_rules! test_unescape_garbage {
    ( $NAME:ident, $UNESCAPED:expr, $ESCAPED:expr ) => {
        #[test]
        fn $NAME() {
            let escaped = $ESCAPED;
            assert_eq!(
                &escaped.unescape_uri().to_string(),
                $UNESCAPED,
                "Failed on uri_unescape_to_string_lossy({:?})",
                escaped
            );
        }
    };
}

test_escape_unescape!(test_ascii_1, "a-simple-test", "a-simple-test");
test_escape_unescape!(test_ascii_2, "a?simple?test", "a%3Fsimple%3Ftest");
test_escape_unescape!(test_ascii_3, "\u{20AC}", "%E2%82%AC");
test_escape_unescape!(
    test_ascii_4,
    "blåbærsyltetøy",
    "bl%C3%A5b%C3%A6rsyltet%C3%B8y"
);
test_escape_unescape!(test_ascii_5, "f/scen?create", "f%2Fscen%3Fcreate");

test_unescape_garbage!(ascii_control_percent_escape, "␀␁␂␃␄", "%00%01%02%03%04");
test_unescape_garbage!(bad_utf8_spaces, "� � �", "%E2 %82 %AC");
test_unescape_garbage!(bad_utf8_3b, "�", "%E2%F2%AC");
test_unescape_garbage!(truncated_utf8_1, "fan�say", "fan%E2%8say");
test_unescape_garbage!(truncated_utf8_2, "fan�say", "fan%E2%82say");
test_unescape_garbage!(truncated_utf8_3, "fan�say", "fan%E2%82%say");
test_unescape_garbage!(bad_percent_escape, "bloat%1zface", "bloat%1zface");
