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
    let from = vars.local_values[SORTED_MEMORY_FROM]; // TODO: replace "from" and "to" with "val" and "R/W"
    let to = vars.local_values[SORTED_MEMORY_TO];
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_from = vars.next_values[SORTED_MEMORY_FROM];
    let next_to = vars.next_values[SORTED_MEMORY_TO];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let trace_context = vars.local_values[MEMORY_TRACE_CONTEXT];
    let trace_segment = vars.local_values[MEMORY_TRACE_SEGMENT];
    let trace_virtual = vars.local_values[MEMORY_TRACE_VIRTUAL];
    let two_traces_combined = vars.local_values[MEMORY_TWO_TRACES_COMBINED];
    let all_traces_combined = vars.local_values[MEMORY_ALL_TRACES_COMBINED];

    let current = vars.local_values[MEMORY_CURRENT];
    let next_current = vars.next_values[MEMORY_CURRENT];

    yield_constr.constraint(trace_context * (F::ONE - trace_context));
    yield_constr.constraint(trace_segment * (F::ONE - trace_segment));
    yield_constr.constraint(trace_virtual * (F::ONE - trace_virtual));

    yield_constr.constraint(trace_context * (next_addr_context - addr_context));
    yield_constr.constraint(trace_segment * (next_addr_segment - addr_segment));
    yield_constr.constraint(trace_virtual * (next_addr_virtual - addr_virtual));

    let context_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(0)];
    let segment_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(1)];
    let virtual_range_check = vars.local_values[super::range_check_degree::col_rc_16_input(2)];

    yield_constr.constraint(
        context_range_check
            - trace_context * (next_addr_segment - addr_segment)
            - (F::ONE - trace_context) * (next_addr_context - addr_context - F::ONE),
    );
    yield_constr.constraint(
        segment_range_check
            - trace_segment * (next_addr_virtual - addr_virtual)
            - (F::ONE - trace_segment) * (next_addr_segment - addr_segment - F::ONE),
    );
    yield_constr.constraint(
        virtual_range_check
            - trace_virtual * (next_timestamp - timestamp)
            - (F::ONE - trace_virtual) * (next_addr_virtual - addr_virtual - F::ONE),
    );

    // Helper constraints to get the product of (1 - trace_context), (1 - trace_segment), and (1 - trace_virtual).
    yield_constr.constraint(two_traces_combined - (F::ONE - trace_context) * (F::ONE - trace_segment));
    yield_constr.constraint(all_traces_combined - two_traces_combined * (F::ONE - trace_virtual));

    // Enumerate purportedly-ordered log using current value c.
    yield_constr.constraint_first_row(current);
    yield_constr.constraint(current - from);
    yield_constr.constraint(next_current - all_traces_combined * to);
}

pub(crate) fn eval_memory_recursively<F: RichField + Extendable<D>, const D: usize>(
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
}
