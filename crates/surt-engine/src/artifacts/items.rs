use serde_json::Value;

pub(crate) use super::delivery::ListedItem;

const ITEM_FILE: &str = "item.json";

type Files = [(String, Vec<u8>)];

pub(crate) fn listed_items(files: &Files) -> Option<Vec<ListedItem>> {
    index(files).map(|(_, items)| items)
}

pub(crate) fn item_view(files: &Files, key: &str) -> Option<Vec<(String, Vec<u8>)>> {
    let (index, items) = index(files)?;
    let elements: Vec<Value> = serde_json::from_slice(index).ok()?;
    let position = items.iter().position(|item| item.key == key)?;
    let entry = serde_json::to_vec_pretty(elements.get(position)?).ok()?;
    let mut view = vec![(ITEM_FILE.to_owned(), entry)];
    for path in items[position].files.values() {
        let (_, bytes) = files.iter().find(|(rel_path, _)| rel_path == path)?;
        view.push((path.clone(), bytes.clone()));
    }
    Some(view)
}

fn index(files: &Files) -> Option<(&[u8], Vec<ListedItem>)> {
    files.iter().find_map(|(rel_path, bytes)| {
        let items: Vec<ListedItem> = serde_json::from_slice(bytes).ok()?;
        references_the_rest(rel_path, &items, files).then_some((bytes.as_slice(), items))
    })
}

fn references_the_rest(index: &str, items: &[ListedItem], files: &Files) -> bool {
    files.iter().all(|(rel_path, _)| {
        rel_path == index
            || items
                .iter()
                .any(|item| item.files.values().any(|path| path == rel_path))
    })
}
