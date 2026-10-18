use super::requested;
use crate::message::Mark;
use crate::symbols::Symbols;

#[test]
fn the_request_names_the_driver_and_counts_its_executions_in_ascii_too() {
    assert_eq!(
        requested(48213, 1).plain(Mark::Waiting),
        "⏳ stop requested; driver pid 48213 is stopping 1 execution…\n"
    );
    assert_eq!(
        Symbols::Ascii.render(&requested(48213, 2).plain(Mark::Waiting)),
        "[waiting] stop requested; driver pid 48213 is stopping 2 executions...\n"
    );
}
