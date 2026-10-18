use std::collections::{HashMap, HashSet};

use super::{Item, ItemListProblem};

struct Walk<'a> {
    needs: HashMap<&'a str, &'a [String]>,
    finished: HashSet<&'a str>,
    stack: Vec<&'a str>,
}

pub(super) fn check_needs(items: &[Item]) -> Result<(), ItemListProblem> {
    let needs: HashMap<&str, &[String]> = items
        .iter()
        .map(|item| (item.key.as_str(), item.needs.as_slice()))
        .collect();
    for item in items {
        if let Some(need) = item
            .needs
            .iter()
            .find(|need| !needs.contains_key(need.as_str()))
        {
            return Err(ItemListProblem::UnknownNeed {
                key: item.key.clone(),
                need: need.clone(),
            });
        }
    }
    let mut walk = Walk {
        needs,
        finished: HashSet::new(),
        stack: Vec::new(),
    };
    match items.iter().find_map(|item| walk.cycle_from(&item.key)) {
        Some(path) => Err(ItemListProblem::Cycle { path }),
        None => Ok(()),
    }
}

impl<'a> Walk<'a> {
    fn cycle_from(&mut self, key: &'a str) -> Option<Vec<String>> {
        if self.finished.contains(key) {
            return None;
        }
        if let Some(start) = self.stack.iter().position(|open| *open == key) {
            let mut path: Vec<String> = self.stack[start..]
                .iter()
                .map(|open| (*open).to_owned())
                .collect();
            path.push(key.to_owned());
            return Some(path);
        }
        self.stack.push(key);
        let needs = self.needs[key];
        if let Some(path) = needs.iter().find_map(|need| self.cycle_from(need)) {
            return Some(path);
        }
        self.stack.pop();
        self.finished.insert(key);
        None
    }
}
