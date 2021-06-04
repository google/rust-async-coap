use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
use async_coap_uri_macros::assert_uri_literal;

fn call_macro() {
    assert_uri_literal!("https://www.example.com/a/%");
}

fn main() {}
