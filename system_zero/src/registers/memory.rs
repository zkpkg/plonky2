//! Memory unit.

pub(crate) const MEMORY_ADDR_CONTEXT: usize = super::START_MEMORY;
pub(crate) const MEMORY_ADDR_SEGMENT: usize = MEMORY_ADDR_CONTEXT + 1;
pub(crate) const MEMORY_ADDR_VIRTUAL: usize = MEMORY_ADDR_SEGMENT + 1;
pub(crate) const MEMORY_VALUE_START: usize = MEMORY_ADDR_VIRTUAL + 1;

pub const fn memory_value_limb(i: usize) -> usize {
    MEMORY_VALUE_START + i
}

pub(crate) const MEMORY_IS_READ: usize = MEMORY_VALUE_START + 8;
pub(crate) const MEMORY_TIMESTAMP: usize = MEMORY_IS_READ + 1;

pub(crate) const SORTED_MEMORY_ADDR_CONTEXT: usize = MEMORY_TIMESTAMP + 1;
pub(crate) const SORTED_MEMORY_ADDR_SEGMENT: usize = SORTED_MEMORY_ADDR_CONTEXT + 1;
pub(crate) const SORTED_MEMORY_ADDR_VIRTUAL: usize = SORTED_MEMORY_ADDR_SEGMENT + 1;
pub(crate) const SORTED_MEMORY_VALUE_START: usize = SORTED_MEMORY_ADDR_VIRTUAL + 1;

pub const fn sorted_memory_value_limb(i: usize) -> usize {
    SORTED_MEMORY_VALUE_START + i
}

pub(crate) const SORTED_MEMORY_IS_READ: usize = SORTED_MEMORY_VALUE_START + 8;
pub(crate) const SORTED_MEMORY_TIMESTAMP: usize = SORTED_MEMORY_IS_READ + 1;

pub(crate) const MEMORY_CONTEXT_UNCHANGED: usize = SORTED_MEMORY_TIMESTAMP + 1;
pub(crate) const MEMORY_SEGMENT_UNCHANGED: usize = MEMORY_CONTEXT_UNCHANGED + 1;
pub(crate) const MEMORY_VIRTUAL_UNCHANGED: usize = MEMORY_SEGMENT_UNCHANGED + 1;
pub(crate) const MEMORY_ADDRESS_UNCHANGED: usize = MEMORY_VIRTUAL_UNCHANGED + 1;

pub(super) const END: usize = MEMORY_ADDRESS_UNCHANGED + 1;
