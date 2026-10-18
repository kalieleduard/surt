mod delivery;
mod items;

use std::fmt::Write;

use sha2::{Digest, Sha256};

pub(crate) use delivery::{AcceptedDelivery, DeliveryContract, DeliveryRejection};
#[cfg_attr(
    not(all(test, feature = "test-support")),
    expect(
        unused_imports,
        reason = "consistency checks and human Item List edits are wired from tickets 10 and 13 on"
    )
)]
pub(crate) use delivery::{ConsistencyCheck, ItemListProblem};
pub(crate) use items::{ListedItem, item_view, listed_items};

const SHA256_HEX_LEN: usize = 64;

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(SHA256_HEX_LEN), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

pub(crate) fn identity<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut pairs: Vec<(&str, &str)> = files.into_iter().collect();
    pairs.sort_unstable();
    let listing: String = pairs
        .into_iter()
        .map(|(rel_path, sha256)| format!("{rel_path}\0{sha256}\n"))
        .collect();
    sha256_hex(listing.as_bytes())
}
