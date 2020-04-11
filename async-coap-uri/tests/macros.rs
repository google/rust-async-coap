use async_coap_uri::{rel_ref, uri, uri_ref};

#[test]
fn test_uri() {
    let _ = uri!("https://www.example.com/");
}

#[test]
fn test_rel_ref() {
    let _ = rel_ref!("a/b/c?q=foobar#frag");
}

#[test]
fn test_uri_ref() {
    let _ = uri_ref!("a/b/c?q=foobar#frag");
}
