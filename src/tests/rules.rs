//! The Window Rules page: lazily loaded rule details.

use super::*;

#[test]
fn opening_a_rule_loads_its_details_once() {
    let mut rt = on(Page::Windowrules, fixtures::rules_data);
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    assert!(
        rt.state
            .windowrules
            .open
            .contains(&"float-pavucontrol".to_owned())
    );
    assert_eq!(
        rt.state
            .jobs
            .skipped
            .iter()
            .filter(|j| **j == "rule-detail")
            .count(),
        1
    );
    // Closing and reopening a loading rule doesn't ask again.
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    assert_eq!(
        rt.state
            .jobs
            .skipped
            .iter()
            .filter(|j| **j == "rule-detail")
            .count(),
        1
    );
}
