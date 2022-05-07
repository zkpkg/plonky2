//! Memory unit.

pub(crate) const MEMORY_ADDR_CONTEXT: usize = super::START_MEMORY;
pub(crate) const MEMORY_ADDR_SEGMENT: usize = MEMORY_ADDR_CONTEXT + 1;
pub(crate) const MEMORY_ADDR_VIRTUAL: usize = MEMORY_ADDR_SEGMENT + 1;
pub(crate) const MEMORY_FROM: usize = MEMORY_ADDR_VIRTUAL + 1;
pub(crate) const MEMORY_TO: usize = MEMORY_FROM + 1;
pub(crate) const MEMORY_TIMESTAMP: usize = MEMORY_TO + 1;

pub(crate) const SORTED_MEMORY_ADDR_CONTEXT: usize = MEMORY_TIMESTAMP + 1;
pub(crate) const SORTED_MEMORY_ADDR_SEGMENT: usize = SORTED_MEMORY_ADDR_CONTEXT + 1;
pub(crate) const SORTED_MEMORY_ADDR_VIRTUAL: usize = SORTED_MEMORY_ADDR_SEGMENT + 1;
pub(crate) const SORTED_MEMORY_FROM: usize = SORTED_MEMORY_ADDR_VIRTUAL + 1;
pub(crate) const SORTED_MEMORY_TO: usize = SORTED_MEMORY_FROM + 1;
pub(crate) const SORTED_MEMORY_TIMESTAMP: usize = SORTED_MEMORY_TO + 1;

pub(crate) const MEMORY_TRACE_CONTEXT: usize = SORTED_MEMORY_TIMESTAMP + 1;
pub(crate) const MEMORY_TRACE_SEGMENT: usize = MEMORY_TRACE_CONTEXT + 1;
pub(crate) const MEMORY_TRACE_VIRTUAL: usize = MEMORY_TRACE_SEGMENT + 1;
pub(crate) const MEMORY_CURRENT: usize = MEMORY_TRACE_VIRTUAL + 1;

pub(super) const END: usize = MEMORY_CURRENT + 1;
