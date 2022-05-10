use itertools::{izip, multiunzip};
use plonky2::field::extension_field::Extendable;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::packed_field::PackedField;
use plonky2::hash::hash_types::RichField;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::vars::{StarkEvaluationTargets, StarkEvaluationVars};

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

pub(crate) fn generate_memory<F: PrimeField64>(trace_cols: &mut [Vec<F>]) {
    let context = &trace_cols[MEMORY_ADDR_CONTEXT];
    let segment = &trace_cols[MEMORY_ADDR_SEGMENT];
    let virtuals = &trace_cols[MEMORY_ADDR_VIRTUAL];
    let value = &trace_cols[MEMORY_VALUE];
    let is_read = &trace_cols[MEMORY_IS_READ];
    let timestamp = &trace_cols[MEMORY_TIMESTAMP];

    let (
        sorted_context,
        sorted_segment,
        sorted_virtual,
        sorted_value,
        sorted_is_read,
        sorted_timestamp,
    ) = sort_memory_ops(context, segment, virtuals, value, is_read, timestamp);

    let (context_unchanged, segment_unchanged, virtual_unchanged, address_unchanged) =
        generate_unchanged_flags(context, segment, virtuals);

    trace_cols[SORTED_MEMORY_ADDR_CONTEXT] = sorted_context;
    trace_cols[SORTED_MEMORY_ADDR_SEGMENT] = sorted_segment;
    trace_cols[SORTED_MEMORY_ADDR_VIRTUAL] = sorted_virtual;
    trace_cols[SORTED_MEMORY_VALUE] = sorted_value;
    trace_cols[SORTED_MEMORY_IS_READ] = sorted_is_read;
    trace_cols[SORTED_MEMORY_TIMESTAMP] = sorted_timestamp;

    trace_cols[MEMORY_CONTEXT_UNCHANGED] = context_unchanged;
    trace_cols[MEMORY_SEGMENT_UNCHANGED] = segment_unchanged;
    trace_cols[MEMORY_VIRTUAL_UNCHANGED] = virtual_unchanged;
    trace_cols[MEMORY_ADDRESS_UNCHANGED] = address_unchanged;
}

pub fn sort_memory_ops<F: PrimeField64>(
    context: &[F],
    segment: &[F],
    virtuals: &[F],
    value: &[F],
    is_read: &[F],
    timestamp: &[F],
) -> (Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>, Vec<F>) {
    let mut ops: Vec<(F, F, F, F, F, F)> = izip!(
        context.iter().cloned(),
        segment.iter().cloned(),
        virtuals.iter().cloned(),
        value.iter().cloned(),
        is_read.iter().cloned(),
        timestamp.iter().cloned()
    )
    .collect();

    ops.sort_by(|&(c1, s1, v1, _, _, t1), &(c2, s2, v2, _, _, t2)| {
        (
            c1.to_noncanonical_u64(),
            s1.to_noncanonical_u64(),
            v1.to_noncanonical_u64(),
            t1.to_noncanonical_u64(),
        )
            .cmp(&(
                c2.to_noncanonical_u64(),
                s2.to_noncanonical_u64(),
                v2.to_noncanonical_u64(),
                t2.to_noncanonical_u64(),
            ))
    });

    multiunzip(ops)
}

pub fn generate_unchanged_flags<F: PrimeField64>(
    context: &[F],
    segment: &[F],
    virtuals: &[F],
) -> (Vec<F>, Vec<F>, Vec<F>, Vec<F>) {
    let num_ops = context.len();
    let mut context_unchanged = Vec::new();
    let mut segment_unchanged = Vec::new();
    let mut virtual_unchanged = Vec::new();
    let mut address_unchanged = Vec::new();
    for idx in 0..num_ops - 1 {
        let this_context_unchanged = if context[idx] == context[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };
        let this_segment_unchanged = if segment[idx] == segment[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };
        let this_virtual_unchanged = if virtuals[idx] == virtuals[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };

        context_unchanged.push(this_context_unchanged);
        segment_unchanged.push(this_segment_unchanged);
        virtual_unchanged.push(this_virtual_unchanged);

        address_unchanged.push(this_context_unchanged * this_segment_unchanged * this_virtual_unchanged);
    }

    context_unchanged.push(F::ZERO);
    segment_unchanged.push(F::ZERO);
    virtual_unchanged.push(F::ZERO);
    address_unchanged.push(F::ZERO);

    (
        context_unchanged,
        segment_unchanged,
        virtual_unchanged,
        address_unchanged,
    )
}

pub(crate) fn eval_memory<F: Field, P: PackedField<Scalar = F>>(
    vars: StarkEvaluationVars<F, P, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut ConstraintConsumer<P>,
) {
    let one = P::from(F::ONE);

    let addr_context = vars.local_values[SORTED_MEMORY_ADDR_CONTEXT];
    let addr_segment = vars.local_values[SORTED_MEMORY_ADDR_SEGMENT];
    let addr_virtual = vars.local_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let value = vars.local_values[SORTED_MEMORY_VALUE];
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_value = vars.next_values[SORTED_MEMORY_VALUE];
    let next_is_read = vars.next_values[SORTED_MEMORY_IS_READ];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let context_unchanged = vars.local_values[MEMORY_CONTEXT_UNCHANGED];
    let segment_unchanged = vars.local_values[MEMORY_SEGMENT_UNCHANGED];
    let virtual_unchanged = vars.local_values[MEMORY_VIRTUAL_UNCHANGED];
    let address_unchanged = vars.local_values[MEMORY_ADDRESS_UNCHANGED];

    let not_context_unchanged = one - context_unchanged;
    let not_segment_unchanged = one - segment_unchanged;
    let not_virtual_unchanged = one - virtual_unchanged;

    // First set of ordering constraint: unchanged flags are boolean.
    yield_constr.constraint(context_unchanged * not_context_unchanged);
    yield_constr.constraint(segment_unchanged * not_segment_unchanged);
    yield_constr.constraint(virtual_unchanged * not_virtual_unchanged);

    // Second set of ordering constraints: unchanged flags are correct.
    yield_constr.constraint(context_unchanged * (next_addr_context - addr_context));
    yield_constr.constraint(segment_unchanged * (next_addr_segment - addr_segment));
    yield_constr.constraint(virtual_unchanged * (next_addr_virtual - addr_virtual));

    let context_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(0)];
    let segment_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(1)];
    let virtual_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(2)];

    // Third set of ordering constraints: range-check difference in the column that should be increasing.
    yield_constr.constraint(
        context_range_check
            - context_unchanged * (next_addr_segment - addr_segment)
            - not_context_unchanged * (next_addr_context - addr_context - one),
    );
    yield_constr.constraint(
        segment_range_check
            - segment_unchanged * (next_addr_virtual - addr_virtual)
            - not_segment_unchanged * (next_addr_segment - addr_segment - one),
    );
    yield_constr.constraint(
        virtual_range_check
            - virtual_unchanged * (next_timestamp - timestamp)
            - not_virtual_unchanged * (next_addr_virtual - addr_virtual - one),
    );

    // Helper constraints to get the product of (1 - context_unchanged), (1 - segment_unchanged), and (1 - virtual_unchanged).
    yield_constr.constraint(address_unchanged - context_unchanged * segment_unchanged * virtual_unchanged);

    // Enumerate purportedly-ordered log.
    yield_constr.constraint(next_is_read * address_unchanged * (next_value - value));
}

pub(crate) fn eval_memory_recursively<F: RichField + Extendable<D>, const D: usize>(
    builder: &mut CircuitBuilder<F, D>,
    vars: StarkEvaluationTargets<D, NUM_COLUMNS, NUM_PUBLIC_INPUTS>,
    yield_constr: &mut RecursiveConstraintConsumer<F, D>,
) {
    let one = builder.one_extension();

    let addr_context = vars.local_values[SORTED_MEMORY_ADDR_CONTEXT];
    let addr_segment = vars.local_values[SORTED_MEMORY_ADDR_SEGMENT];
    let addr_virtual = vars.local_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let value = vars.local_values[SORTED_MEMORY_VALUE];
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_value = vars.next_values[SORTED_MEMORY_VALUE];
    let next_is_read = vars.next_values[SORTED_MEMORY_IS_READ];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let context_unchanged = vars.local_values[MEMORY_CONTEXT_UNCHANGED];
    let segment_unchanged = vars.local_values[MEMORY_SEGMENT_UNCHANGED];
    let virtual_unchanged = vars.local_values[MEMORY_VIRTUAL_UNCHANGED];
    let address_unchanged = vars.local_values[MEMORY_ADDRESS_UNCHANGED];

    let not_context_unchanged = builder.sub_extension(one, context_unchanged);
    let not_segment_unchanged = builder.sub_extension(one, segment_unchanged);
    let not_virtual_unchanged = builder.sub_extension(one, virtual_unchanged);
    let addr_context_diff = builder.sub_extension(next_addr_context, addr_context);
    let addr_segment_diff = builder.sub_extension(next_addr_segment, addr_segment);
    let addr_virtual_diff = builder.sub_extension(next_addr_virtual, addr_virtual);
    let timestamp_diff = builder.sub_extension(next_timestamp, timestamp);

    // First set of ordering constraint: traces are boolean.
    let context_unchanged_bool = builder.mul_extension(context_unchanged, not_context_unchanged);
    yield_constr.constraint(builder, context_unchanged_bool);
    let segment_unchanged_bool = builder.mul_extension(segment_unchanged, not_segment_unchanged);
    yield_constr.constraint(builder, segment_unchanged_bool);
    let virtual_unchanged_bool = builder.mul_extension(virtual_unchanged, not_virtual_unchanged);
    yield_constr.constraint(builder, virtual_unchanged_bool);

    // Second set of ordering constraints: trace matches with no change in corresponding column.
    let cond_context_diff = builder.mul_extension(context_unchanged, addr_context_diff);
    yield_constr.constraint(builder, cond_context_diff);
    let cond_segment_diff = builder.mul_extension(segment_unchanged, addr_segment_diff);
    yield_constr.constraint(builder, cond_segment_diff);
    let cond_virtual_diff = builder.mul_extension(virtual_unchanged, addr_virtual_diff);
    yield_constr.constraint(builder, cond_virtual_diff);

    let context_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(0)];
    let segment_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(1)];
    let virtual_range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(2)];

    // Third set of ordering constraints: range-check difference in the column that should be increasing.
    let diff_if_context_equal = builder.mul_extension(context_unchanged, addr_segment_diff);
    let addr_context_diff_min_one = builder.sub_extension(addr_context_diff, one);
    let diff_if_context_unequal =
        builder.mul_extension(not_context_unchanged, addr_context_diff_min_one);
    let sum_of_diffs_context =
        builder.add_extension(diff_if_context_equal, diff_if_context_unequal);
    let context_range_check_constraint =
        builder.sub_extension(context_range_check, sum_of_diffs_context);
    yield_constr.constraint(builder, context_range_check_constraint);

    let diff_if_segment_equal = builder.mul_extension(segment_unchanged, addr_virtual_diff);
    let addr_segment_diff_min_one = builder.sub_extension(addr_segment_diff, one);
    let diff_if_segment_unequal =
        builder.mul_extension(not_segment_unchanged, addr_segment_diff_min_one);
    let sum_of_diffs_segment =
        builder.add_extension(diff_if_segment_equal, diff_if_segment_unequal);
    let segment_range_check_constraint =
        builder.sub_extension(segment_range_check, sum_of_diffs_segment);
    yield_constr.constraint(builder, segment_range_check_constraint);

    let diff_if_virtual_equal = builder.mul_extension(virtual_unchanged, timestamp_diff);
    let addr_virtual_diff_min_one = builder.sub_extension(addr_virtual_diff, one);
    let diff_if_virtual_unequal =
        builder.mul_extension(not_virtual_unchanged, addr_virtual_diff_min_one);
    let sum_of_diffs_virtual =
        builder.add_extension(diff_if_virtual_equal, diff_if_virtual_unequal);
    let virtual_range_check_constraint =
        builder.sub_extension(virtual_range_check, sum_of_diffs_virtual);
    yield_constr.constraint(builder, virtual_range_check_constraint);

    // Helper constraint to check address_changes.
    let first_two_unchanged = builder.mul_extension(context_unchanged, segment_unchanged);
    let all_unchanged = builder.mul_extension(first_two_unchanged, virtual_unchanged);
    let all_unchanged_diff = builder.sub_extension(address_unchanged, all_unchanged);
    yield_constr.constraint(builder, all_unchanged_diff);

    // Enumerate purportedly-ordered log.
    let value_diff = builder.sub_extension(next_value, value);
    let zero_if_read = builder.mul_extension(address_unchanged, value_diff);
    let read_constraint = builder.mul_extension(next_is_read, zero_if_read);
    yield_constr.constraint(builder, read_constraint);
}
