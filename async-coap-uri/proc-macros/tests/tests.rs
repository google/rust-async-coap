use trybuild::TestCases;

#[test]
fn tests() {
    let t = TestCases::new();

    // tests for `assert_uri_literal` macro:
    t.compile_fail("tests/ui/assert_uri_literal/escaped_ascii_control.rs");
    t.compile_fail("tests/ui/assert_uri_literal/invalid_escape.rs");
    t.compile_fail("tests/ui/assert_uri_literal/invalid_literal.rs");
    t.compile_fail("tests/ui/assert_uri_literal/invalid_utf8.rs");
    t.compile_fail("tests/ui/assert_uri_literal/missing_char_1.rs");
    t.compile_fail("tests/ui/assert_uri_literal/missing_char_2.rs");
    t.compile_fail("tests/ui/assert_uri_literal/space.rs");
    t.compile_fail("tests/ui/assert_uri_literal/unescaped_ascii_control.rs");
    t.compile_fail("tests/ui/assert_uri_literal/unfinished_utf8.rs");

    t.compile_fail("tests/ui/assert_uri_literal/malformed_structure_1.rs");
    t.compile_fail("tests/ui/assert_uri_literal/malformed_structure_2.rs");
    t.compile_fail("tests/ui/assert_uri_literal/malformed_scheme.rs");

    // tests for `assert_rel_ref_literal` macro:
    t.compile_fail("tests/ui/assert_rel_ref_literal/escaped_ascii_control.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/invalid_escape.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/invalid_literal.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/invalid_utf8.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/missing_char_1.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/missing_char_2.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/space.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/unescaped_ascii_control.rs");
    t.compile_fail("tests/ui/assert_rel_ref_literal/unfinished_utf8.rs");
}
