use super::{Moment, Slot, command, offer_into};

const SUBJECT: &str = "20261017-add-login";

#[test]
fn gates_suggest_approving_the_subject() {
    assert_eq!(
        command(Moment::AtGates, SUBJECT),
        "surt approve 20261017-add-login"
    );
}

#[test]
fn approved_suggests_continuing() {
    assert_eq!(
        command(Moment::Approved, SUBJECT),
        "surt continue 20261017-add-login"
    );
}

#[test]
fn rejected_suggests_continuing() {
    assert_eq!(
        command(Moment::Rejected, SUBJECT),
        "surt continue 20261017-add-login"
    );
}

#[test]
fn blocked_suggests_continuing() {
    assert_eq!(
        command(Moment::Blocked, SUBJECT),
        "surt continue 20261017-add-login"
    );
}

#[test]
fn interrupted_suggests_continuing() {
    assert_eq!(
        command(Moment::Interrupted, SUBJECT),
        "surt continue 20261017-add-login"
    );
}

#[test]
fn a_new_slot_is_empty() {
    assert_eq!(Slot::new().take(), None);
}

#[test]
fn take_returns_the_command_once() {
    let slot = Slot::new();
    slot.set("surt continue s".to_owned());
    assert_eq!(slot.take().as_deref(), Some("surt continue s"));
    assert_eq!(slot.take(), None);
}

#[test]
fn a_later_command_overwrites_an_earlier_one() {
    let slot = Slot::new();
    slot.set("surt approve s".to_owned());
    slot.set("surt continue s".to_owned());
    assert_eq!(slot.take().as_deref(), Some("surt continue s"));
}

#[test]
fn offering_outside_a_terminal_leaves_the_slot_empty() {
    let slot = Slot::new();
    offer_into(&slot, false, Moment::Blocked, SUBJECT);
    assert_eq!(slot.take(), None);
}

#[test]
fn offering_in_a_terminal_fills_the_slot() {
    let slot = Slot::new();
    offer_into(&slot, true, Moment::AtGates, SUBJECT);
    assert_eq!(
        slot.take().as_deref(),
        Some("surt approve 20261017-add-login")
    );
}
