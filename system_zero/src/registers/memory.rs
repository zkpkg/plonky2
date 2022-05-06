//! Memory unit.

pub(crate) const MEMORY_ADDR_CONTEXT: usize = super::START_MEMORY;
pub(crate) const MEMORY_ADDR_SEGMENT: usize = MEMORY_ADDR_CONTEXT + 1;
pub(crate) const MEMORY_ADDR_VIRTUAL: usize = MEMORY_ADDR_SEGMENT + 1;
pub(crate) const MEMORY_VALUE: usize = MEMORY_ADDR_VIRTUAL + 1;
pub(crate) const MEMORY_READ_OR_WRITE: usize = MEMORY_VALUE + 1;
pub(crate) const MEMORY_TIMESTAMP: usize = MEMORY_READ_OR_WRITE + 1;

pub(crate) const SORTED_MEMORY_ADDR_CONTEXT: usize = MEMORY_TIMESTAMP + 1;
pub(crate) const SORTED_MEMORY_ADDR_SEGMENT: usize = SORTED_MEMORY_ADDR_CONTEXT + 1;
pub(crate) const SORTED_MEMORY_ADDR_VIRTUAL: usize = SORTED_MEMORY_ADDR_SEGMENT + 1;
pub(crate) const SORTED_MEMORY_VALUE: usize = SORTED_MEMORY_ADDR_VIRTUAL + 1;
pub(crate) const SORTED_MEMORY_READ_OR_WRITE: usize = SORTED_MEMORY_VALUE + 1;
pub(crate) const SORTED_MEMORY_TIMESTAMP: usize = SORTED_MEMORY_READ_OR_WRITE + 1;

pub(crate) const MEMORY_TRACE: usize = SORTED_MEMORY_TIMESTAMP + 1;
pub(crate) const MEMORY_CURRENT: usize = MEMORY_TRACE + 1;

pub(super) const END: usize = MEMORY_CURRENT + 1;
