#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the tick loop of drive executes decide from ticket 08 on"
    )
)]

mod decide;
mod graph;
