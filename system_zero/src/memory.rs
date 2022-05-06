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
    let addr_context = vars.local_values[SORTED_MEMORY_ADDR_CONTEXT];
    let addr_segment = vars.local_values[SORTED_MEMORY_ADDR_SEGMENT];
    let addr_virtual = vars.local_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let val = vars.local_values[SORTED_MEMORY_VALUE];
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_val = vars.next_values[SORTED_MEMORY_VALUE];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];
    
    let memory_trace_context = vars.local_values[MEMORY_TRACE_CONTEXT];
    let memory_trace_segment = vars.local_values[MEMORY_TRACE_SEGMENT];
    let memory_trace_virtual = vars.local_values[MEMORY_TRACE_VIRTUAL];
    let memory_current = vars.local_values[MEMORY_CURRENT];

    yield_constr.constraint(memory_trace_context * (F::ONE - memory_trace_context));
    yield_constr.constraint(memory_trace_segment * (F::ONE - memory_trace_segment));
    yield_constr.constraint(memory_trace_virtual * (F::ONE - memory_trace_virtual));

    yield_constr.constraint(memory_trace_context * (next_addr_context - addr_context));
    yield_constr.constraint(memory_trace_segment * (next_addr_segment - addr_segment));
    yield_constr.constraint(memory_trace_virtual * (next_addr_virtual - addr_virtual));

    let context_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(0)];
    let segment_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(1)];
    let virtual_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(2)];

    yield_constr.constraint(context_range_check - memory_trace_context * (next_addr_segment - addr_segment) - (F::ONE - memory_trace_context) * (next_addr_context - addr_context - F::ONE));
    yield_constr.constraint(segment_range_check - memory_trace_segment * (next_addr_virtual - addr_virtual) - (F::ONE - memory_trace_segment) * (next_addr_segment - addr_segment - F::ONE));
    yield_constr.constraint(virtual_range_check - memory_trace_virtual * (next_timestamp - timestamp) - (F::ONE - memory_trace_virtual) * (next_addr_virtual - addr_virtual - F::ONE));

}

pub(crate) fn eval_memory_recursively<F: RichField + Extendable<D>, const D: usize>(
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    
}
