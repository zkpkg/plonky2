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
    let values: Vec<Vec<F>> = (0..8)
        .map(|i| &trace_cols[memory_value_limb(i)])
        .cloned()
        .collect();
    let is_read = &trace_cols[MEMORY_IS_READ];
    let timestamp = &trace_cols[MEMORY_TIMESTAMP];

    let (
        sorted_context,
        sorted_segment,
        sorted_virtual,
        sorted_values,
        sorted_is_read,
        sorted_timestamp,
    ) = sort_memory_ops(context, segment, virtuals, &values, is_read, timestamp);

    let (context_first_change, segment_first_change, virtual_first_change) =
        generate_first_change_flags(context, segment, virtuals);

    trace_cols[SORTED_MEMORY_ADDR_CONTEXT] = sorted_context;
    trace_cols[SORTED_MEMORY_ADDR_SEGMENT] = sorted_segment;
    trace_cols[SORTED_MEMORY_ADDR_VIRTUAL] = sorted_virtual;
    for i in 0..8 {
        trace_cols[sorted_memory_value_limb(i)] = sorted_values[i].clone();
    }
    trace_cols[SORTED_MEMORY_IS_READ] = sorted_is_read;
    trace_cols[SORTED_MEMORY_TIMESTAMP] = sorted_timestamp;

    trace_cols[MEMORY_CONTEXT_FIRST_CHANGE] = context_first_change;
    trace_cols[MEMORY_SEGMENT_FIRST_CHANGE] = segment_first_change;
    trace_cols[MEMORY_VIRTUAL_FIRST_CHANGE] = virtual_first_change;
}

pub fn sort_memory_ops<F: PrimeField64>(
    context: &[F],
    segment: &[F],
    virtuals: &[F],
    values: &[Vec<F>],
    is_read: &[F],
    timestamp: &[F],
) -> (Vec<F>, Vec<F>, Vec<F>, Vec<Vec<F>>, Vec<F>, Vec<F>) {
    let mut ops: Vec<(F, F, F, Vec<F>, F, F)> = izip!(
        context.iter().cloned(),
        segment.iter().cloned(),
        virtuals.iter().cloned(),
        values.iter().cloned(),
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

pub fn generate_first_change_flags<F: PrimeField64>(
    context: &[F],
    segment: &[F],
    virtuals: &[F],
) -> (Vec<F>, Vec<F>, Vec<F>) {
    let num_ops = context.len();
    let mut context_first_change = Vec::new();
    let mut segment_first_change = Vec::new();
    let mut virtual_first_change = Vec::new();
    for idx in 0..num_ops - 1 {
        let this_context_first_change = if context[idx] == context[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };
        let this_segment_first_change = if segment[idx] == segment[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };
        let this_virtual_first_change = if virtuals[idx] == virtuals[idx + 1] {
            F::ONE
        } else {
            F::ZERO
        };

        context_first_change.push(this_context_first_change);
        segment_first_change.push(this_segment_first_change);
        virtual_first_change.push(this_virtual_first_change);
    }

    context_first_change.push(F::ZERO);
    segment_first_change.push(F::ZERO);
    virtual_first_change.push(F::ZERO);

    (
        context_first_change,
        segment_first_change,
        virtual_first_change,
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
    let values: Vec<_> = (0..8)
        .map(|i| vars.local_values[sorted_memory_value_limb(i)])
        .collect();
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_values: Vec<_> = (0..8)
        .map(|i| vars.next_values[sorted_memory_value_limb(i)])
        .collect();
    let next_is_read = vars.next_values[SORTED_MEMORY_IS_READ];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let context_first_change = vars.local_values[MEMORY_CONTEXT_FIRST_CHANGE];
    let segment_first_change = vars.local_values[MEMORY_SEGMENT_FIRST_CHANGE];
    let virtual_first_change = vars.local_values[MEMORY_VIRTUAL_FIRST_CHANGE];

    let not_context_first_change = one - context_first_change;
    let not_segment_first_change = one - segment_first_change;
    let not_virtual_first_change = one - virtual_first_change;

    // First set of ordering constraint: first_change flags are boolean.
    yield_constr.constraint(context_first_change * not_context_first_change);
    yield_constr.constraint(segment_first_change * not_segment_first_change);
    yield_constr.constraint(virtual_first_change * not_virtual_first_change);

    // Second set of ordering constraints: first_change flags are correct.
    yield_constr.constraint(context_first_change * (next_addr_context - addr_context));
    yield_constr.constraint(segment_first_change * (next_addr_segment - addr_segment));
    yield_constr.constraint(virtual_first_change * (next_addr_virtual - addr_virtual));

    // Third set of ordering constraints: range-check difference in the column that should be increasing.
    let range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(0)];

    let timestamp_first_change =
        one - context_first_change - segment_first_change - virtual_first_change;
    let range_check_value = context_first_change * (next_addr_context - addr_context - one)
        + segment_first_change * (next_addr_segment - addr_segment - one)
        + virtual_first_change * (next_addr_virtual - addr_virtual - one)
        + timestamp_first_change * (next_timestamp - timestamp - one);
    yield_constr.constraint(range_check - range_check_value);

    // Enumerate purportedly-ordered log.
    for i in 0..8 {
        yield_constr
            .constraint(next_is_read * timestamp_first_change * (next_values[i] - values[i]));
    }
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
    let values: Vec<_> = (0..8)
        .map(|i| vars.local_values[sorted_memory_value_limb(i)])
        .collect();
    let timestamp = vars.local_values[SORTED_MEMORY_TIMESTAMP];

    let next_addr_context = vars.next_values[SORTED_MEMORY_ADDR_CONTEXT];
    let next_addr_segment = vars.next_values[SORTED_MEMORY_ADDR_SEGMENT];
    let next_addr_virtual = vars.next_values[SORTED_MEMORY_ADDR_VIRTUAL];
    let next_values: Vec<_> = (0..8)
        .map(|i| vars.next_values[sorted_memory_value_limb(i)])
        .collect();
    let next_is_read = vars.next_values[SORTED_MEMORY_IS_READ];
    let next_timestamp = vars.next_values[SORTED_MEMORY_TIMESTAMP];

    let context_first_change = vars.local_values[MEMORY_CONTEXT_FIRST_CHANGE];
    let segment_first_change = vars.local_values[MEMORY_SEGMENT_FIRST_CHANGE];
    let virtual_first_change = vars.local_values[MEMORY_VIRTUAL_FIRST_CHANGE];

    let not_context_first_change = builder.sub_extension(one, context_first_change);
    let not_segment_first_change = builder.sub_extension(one, segment_first_change);
    let not_virtual_first_change = builder.sub_extension(one, virtual_first_change);
    let addr_context_diff = builder.sub_extension(next_addr_context, addr_context);
    let addr_segment_diff = builder.sub_extension(next_addr_segment, addr_segment);
    let addr_virtual_diff = builder.sub_extension(next_addr_virtual, addr_virtual);
    let timestamp_diff = builder.sub_extension(next_timestamp, timestamp);

    // First set of ordering constraint: traces are boolean.
    let context_first_change_bool =
        builder.mul_extension(context_first_change, not_context_first_change);
    yield_constr.constraint(builder, context_first_change_bool);
    let segment_first_change_bool =
        builder.mul_extension(segment_first_change, not_segment_first_change);
    yield_constr.constraint(builder, segment_first_change_bool);
    let virtual_first_change_bool =
        builder.mul_extension(virtual_first_change, not_virtual_first_change);
    yield_constr.constraint(builder, virtual_first_change_bool);

    // Second set of ordering constraints: trace matches with no change in corresponding column.
    let cond_context_diff = builder.mul_extension(context_first_change, addr_context_diff);
    yield_constr.constraint(builder, cond_context_diff);
    let cond_segment_diff = builder.mul_extension(segment_first_change, addr_segment_diff);
    yield_constr.constraint(builder, cond_segment_diff);
    let cond_virtual_diff = builder.mul_extension(virtual_first_change, addr_virtual_diff);
    yield_constr.constraint(builder, cond_virtual_diff);

    // Third set of ordering constraints: range-check difference in the column that should be increasing.
    let range_check =
        vars.local_values[crate::registers::range_check_degree::col_rc_degree_input(0)];

    let timestamp_first_change = {
        let mut cur = builder.sub_extension(one, context_first_change);
        cur = builder.sub_extension(cur, segment_first_change);
        builder.sub_extension(cur, virtual_first_change)
    };

    let context_diff = {
        let diff = builder.sub_extension(next_addr_context, addr_context);
        builder.sub_extension(diff, one)
    };
    let context_range_check = builder.mul_extension(context_first_change, context_diff);
    let segment_diff = {
        let diff = builder.sub_extension(next_addr_segment, addr_segment);
        builder.sub_extension(diff, one)
    };
    let segment_range_check = builder.mul_extension(segment_first_change, segment_diff);
    let virtual_diff = {
        let diff = builder.sub_extension(next_addr_virtual, addr_virtual);
        builder.sub_extension(diff, one)
    };
    let virtual_range_check = builder.mul_extension(virtual_first_change, virtual_diff);
    let timestamp_diff = {
        let diff = builder.sub_extension(next_timestamp, timestamp);
        builder.sub_extension(diff, one)
    };
    let timestamp_range_check = builder.mul_extension(timestamp_first_change, timestamp_diff);

    let range_check_value = {
        let mut sum = builder.add_extension(context_range_check, segment_range_check);
        sum = builder.add_extension(sum, virtual_range_check);
        builder.add_extension(sum, timestamp_range_check)
    };
    let range_check_diff = builder.sub_extension(range_check, range_check_value);
    yield_constr.constraint(builder, range_check_diff);

    let diff_if_context_equal = builder.mul_extension(context_first_change, addr_segment_diff);
    let addr_context_diff_min_one = builder.sub_extension(addr_context_diff, one);
    let diff_if_context_unequal =
        builder.mul_extension(not_context_first_change, addr_context_diff_min_one);
    let sum_of_diffs_context =
        builder.add_extension(diff_if_context_equal, diff_if_context_unequal);
    let context_range_check_constraint =
        builder.sub_extension(context_range_check, sum_of_diffs_context);
    yield_constr.constraint(builder, context_range_check_constraint);

    let diff_if_segment_equal = builder.mul_extension(segment_first_change, addr_virtual_diff);
    let addr_segment_diff_min_one = builder.sub_extension(addr_segment_diff, one);
    let diff_if_segment_unequal =
        builder.mul_extension(not_segment_first_change, addr_segment_diff_min_one);
    let sum_of_diffs_segment =
        builder.add_extension(diff_if_segment_equal, diff_if_segment_unequal);
    let segment_range_check_constraint =
        builder.sub_extension(segment_range_check, sum_of_diffs_segment);
    yield_constr.constraint(builder, segment_range_check_constraint);

    let diff_if_virtual_equal = builder.mul_extension(virtual_first_change, timestamp_diff);
    let addr_virtual_diff_min_one = builder.sub_extension(addr_virtual_diff, one);
    let diff_if_virtual_unequal =
        builder.mul_extension(not_virtual_first_change, addr_virtual_diff_min_one);
    let sum_of_diffs_virtual =
        builder.add_extension(diff_if_virtual_equal, diff_if_virtual_unequal);
    let virtual_range_check_constraint =
        builder.sub_extension(virtual_range_check, sum_of_diffs_virtual);
    yield_constr.constraint(builder, virtual_range_check_constraint);

    // Enumerate purportedly-ordered log.
    for i in 0..8 {
        let value_diff = builder.sub_extension(next_values[i], values[i]);
        let zero_if_read = builder.mul_extension(timestamp_first_change, value_diff);
        let read_constraint = builder.mul_extension(next_is_read, zero_if_read);
        yield_constr.constraint(builder, read_constraint);
    }
}
