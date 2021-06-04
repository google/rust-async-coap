use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
use async_coap_uri_macros::assert_rel_ref_literal;

fn call_macro() {
    assert_rel_ref_literal!("https://www.example.com/a/%:A/c");
}

fn main() {}
