use std::collections::VecDeque;
use std::marker::PhantomData;

/// A generic ID allocator for stable IDs.
#[derive(Debug, Clone)]
pub struct StableIdAllocator<Id>
    where Id: From<u32>
{
    free_ids: VecDeque<u32>,
    next_id: u32,
    _phantom: PhantomData<Id>, // To tie Id to the struct
}

impl<Id> StableIdAllocator<Id>
    where Id: From<u32>
{
    /// Creates a new `StableIdAllocator`.
    ///
    /// `id_factory`: A closure that takes a `u32` (the raw ID value) and returns your custom `Id` type.
    pub fn new() -> Self {
        StableIdAllocator {
            free_ids: VecDeque::new(),
            next_id: 0,
            _phantom: PhantomData,
        }
    }

    /// Acquires a new ID.
    ///
    /// This method first checks the free list. If there are available IDs, it reuses one.
    /// Otherwise, it generates a new unique ID using the provided factory function.
    pub fn acquire_id(&mut self) -> Id {
        if let Some(reused_id) = self.free_ids.pop_front() {
            From::from(reused_id)
        } else {
            let new_id_value = self.next_id;
            self.next_id += 1;
            From::from(new_id_value)
        }
    }

    /// Releases an ID, making it available for reuse.
    ///
    /// This method takes the raw `u32` value of the ID to be released.
    pub fn release_id(&mut self, id_value: u32) {
        self.free_ids.push_back(id_value);
    }
}
