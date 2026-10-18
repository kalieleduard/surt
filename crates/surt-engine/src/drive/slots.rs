use crate::platform::{self, ExclusiveLock};
use crate::project::{Error, Layout};

pub(super) struct Slot {
    index: usize,
    _lock: ExclusiveLock,
}

impl Slot {
    pub(super) fn index(&self) -> usize {
        self.index
    }
}

pub(super) struct FreeSlots(Vec<Slot>);

impl FreeSlots {
    pub(super) fn take(layout: &Layout, limit: usize, held: &[usize]) -> Result<Self, Error> {
        let mut free = Vec::new();
        for index in (0..limit).filter(|index| !held.contains(index)) {
            let path = layout.slot(index);
            let lock = platform::try_lock_exclusive(&path)
                .map_err(|error| Error::io("lock", &path, error))?;
            if let Some(lock) = lock {
                free.push(Slot { index, _lock: lock });
            }
        }
        Ok(FreeSlots(free))
    }

    pub(super) fn indices(&self) -> Vec<usize> {
        self.0.iter().map(Slot::index).collect()
    }

    pub(super) fn claim(&mut self, index: usize) -> Option<Slot> {
        let position = self.0.iter().position(|slot| slot.index == index)?;
        Some(self.0.swap_remove(position))
    }
}
