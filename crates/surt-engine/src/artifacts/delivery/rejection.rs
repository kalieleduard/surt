use super::item_list::ItemListProblem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DeliveryRejection {
    MissingEnvelope,
    InvalidEnvelope,
    UnknownOutcome {
        outcome: String,
    },
    MissingOutput {
        name: String,
    },
    UndeclaredOutput {
        name: String,
    },
    UnsafePath {
        path: String,
    },
    Link {
        path: String,
    },
    MissingFile {
        path: String,
    },
    NotRegularFile {
        path: String,
    },
    EmptyFile {
        path: String,
    },
    NotUtf8 {
        path: String,
    },
    InvalidJson {
        path: String,
        detail: String,
    },
    SchemaViolation {
        path: String,
        detail: String,
    },
    InvalidItemList {
        path: String,
        problem: ItemListProblem,
    },
    Inconsistent {
        check: String,
        detail: String,
    },
}
