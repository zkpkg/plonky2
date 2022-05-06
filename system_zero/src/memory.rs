use crate::public_input_layout::NUM_PUBLIC_INPUTS;
use crate::registers::memory::*;
use crate::registers::NUM_COLUMNS;

#[derive(Default)]
pub struct TransactionMemory {
    pub calls: Vec<ContractMemory>,
}

/// A virtual memory space specific to the current contract call.
pub struct ContractMemory {
    pub code: MemorySegment,
    pub main: MemorySegment,
    pub calldata: MemorySegment,
    pub returndata: MemorySegment,
}

pub struct MemorySegment {
    pub content: Vec<u8>,
}


pub(crate) fn generate_memory<F: PrimeField64>(values: &mut [F; NUM_COLUMNS]) {
    todo!()
}

pub(crate) fn eval_memory<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let addr_context = vars.local_values[MEMORY_ADDR_CONTEXT];
    let addr_segment = vars.local_values[MEMORY_ADDR_SEGMENT];
    let addr_virtual = vars.local_values[MEMORY_ADDR_VIRTUAL];
    let val = vars.local_values[MEMORY_VALUE];
    let timestamp = vars.local_values[MEMORY_TIMESTAMP];

    let sorted_addr_context = vars.local_values[SORTED_MEMORY_ADDR_CONTEXT];
    let sorted_addr_segment = vars.local_values[SORTED_MEMORY_ADDR_SEGMENT];
    let sorted_addr_virtual = vars.local_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let sorted_val = vars.local_values[SORTED_MEMORY_VALUE];
    let sorted_timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_sorted_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_sorted_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_sorted_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_sorted_val = vars.next_values[SORTED_MEMORY_VALUE];
    let next_sorted_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let memory_trace = vars.local_values[MEMORY_TRACE];
    let memory_current = vars.local_values[MEMORY_CURRENT];

    yield_constr.constraint(memory_trace * (F::ONE - memory_trace));
    
}

pub(crate) fn eval_memory_recursively<F: RichField + Extendable<D>, const D: usize>(
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    
}
