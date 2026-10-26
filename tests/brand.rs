//! `lyrn brand pick`, run as a user runs it: the lessons of the line's board
//! are caught by the command, and its exit code says whether the choice holds.

mod common;

fn pick(args: &[&str]) -> (String, bool) {
    let out = common::lyrn().args(["brand", "pick"]).args(args).output().unwrap();
    (
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
        out.status.success(),
    )
}

#[test]
fn mi_on_orange_is_caught_before_the_assets_are_laid_out() {
    let (report, passed) = pick(&["midda", "--code", "mi", "--colour", "#D9704A"]);
    assert!(!passed, "{report}");
    for line in [
        "lyrn brand pick: midda, against the line's",
        "  ✓ mi  free",
        "is fit for the second colour of a pair",
        "  ✗ mi on #D9704A (16°) is Xiaomi",
    ] {
        assert!(report.contains(line), "no `{line}` in:\n{report}");
    }
}

#[test]
fn a_free_code_on_a_free_hue_passes() {
    let (report, passed) = pick(&["nova-sync", "--code", "ns", "--color", "#3EB556"]);
    assert!(passed, "{report}");
    assert!(report.contains("free for a single colour"), "{report}");
    assert!(report.contains("  ✓ no known mark is ns on #3EB556"), "{report}");
}

#[test]
fn a_pair_is_taken_one_way_and_free_the_other() {
    let (report, passed) = pick(&["nova", "--code", "nv", "--colour", "#D9704A", "--to", "#2FAF8C"]);
    assert!(!passed && report.contains("scheda's pair, in the same direction"), "{report}");
    let (report, passed) = pick(&["nova", "--code", "nv", "--colour", "#2FAF8C", "--to", "#D9704A"]);
    assert!(passed && report.contains("the reverse of scheda's pair"), "{report}");
}

#[test]
fn without_a_code_the_name_gives_candidates() {
    let (report, passed) = pick(&["kasl-server"]);
    assert!(passed, "a menu is not a verdict:\n{report}");
    assert!(report.contains("  ✓ ks  free"), "a product's own code is its own:\n{report}");
    let (report, _) = pick(&["kestrel"]);
    assert!(report.contains("  ✗ ks  kasl-server's"), "{report}");
}

#[test]
fn what_is_not_a_code_or_a_colour_is_refused() {
    let (report, passed) = pick(&["nova", "--code", "NV"]);
    assert!(!passed && report.contains("is not a code"), "{report}");
    let (report, passed) = pick(&["nova", "--colour", "green"]);
    assert!(!passed && report.contains("is not a `#rrggbb` colour"), "{report}");
    let (report, passed) = pick(&["nova", "--to", "#2FAF8C"]);
    assert!(!passed && report.contains("`--to` is the second colour of a pair"), "{report}");
}
