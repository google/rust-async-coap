use async_coap_uri::{rel_ref, uri};

#[test]
fn test_uri() {
    let _ = uri!("https://www.example.com/");
}

#[test]
fn test_rel_ref() {
    let _ = rel_ref!("a/b/c?q=foobar#frag");
}
