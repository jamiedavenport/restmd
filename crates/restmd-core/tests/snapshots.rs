//! Snapshot tests: parse every fixture in `tests/fixtures/` and snapshot the
//! full `Parsed` tree (document + errors). These lock down the *shape* of the
//! parse — including spans — so any accidental change is caught on review.
//!
//! Run `cargo insta review` to accept intentional changes.

use insta::{assert_debug_snapshot, glob};

#[test]
fn fixtures() {
    glob!("fixtures/*.md", |path| {
        let src = std::fs::read_to_string(path).unwrap();
        let parsed = restmd_core::parse(&src);
        assert_debug_snapshot!(parsed);
    });
}
