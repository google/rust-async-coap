use async_coap_uri::uri;

#[test]
fn test_uri() {
    let _ = uri!("https://www.example.com/");
}
