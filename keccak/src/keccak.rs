use itertools::Itertools;
use log::info;
use plonky2::field::extension_field::FieldExtension;
use plonky2::field::field_types::{Field, PrimeField64};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::field::packed_field::PackedField;
use plonky2::field::polynomial::PolynomialValues;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::plonk_common::reduce_with_powers_ext_recursive;
use plonky2::timed;
use plonky2::util::timing::TimingTree;
use starky::constraint_consumer::{ConstraintConsumer, RecursiveConstraintConsumer};
use starky::stark::Stark;
use starky::util::trace_rows_to_poly_values;
use starky::vars::StarkEvaluationTargets;
use starky::vars::StarkEvaluationVars;

use crate::logic::{
    andn, andn_gen, andn_gen_circuit, xor, xor3_gen_circuit, xor_gen, xor_gen_circuit,
};
use crate::registers::{
    rc_value_bit, reg_a, reg_a_prime, reg_a_prime_prime, reg_a_prime_prime_0_0_bit, reg_b, reg_c,
    reg_c_partial, reg_step, NUM_REGISTERS, REG_A_PRIME_PRIME_PRIME_0_0_HI,
    REG_A_PRIME_PRIME_PRIME_0_0_LO,
};
use crate::round_flags::{eval_round_flags, eval_round_flags_recursively};

/// Number of rounds in a Keccak permutation.
pub(crate) const NUM_ROUNDS: usize = 24;

/// Number of 64-bit limbs in a preimage of the Keccak permutation.
pub(crate) const INPUT_LIMBS: usize = 25;

type F = GoldilocksField;
const D: usize = 2;

pub(crate) const NUM_PUBLIC_INPUTS: usize = 4;

#[derive(Copy, Clone)]
pub struct Keccak;

impl Keccak {
    /// Generate the rows of the trace. Note that this does not generate the permuted columns used
    /// in our lookup arguments, as those are computed after transposing to column-wise form.
    fn generate_trace_rows(&self, inputs: Vec<[u64; INPUT_LIMBS]>) -> Vec<[F; NUM_REGISTERS]> {
        let num_rows = (inputs.len() * NUM_ROUNDS).next_power_of_two();
        info!("{} rows", num_rows);
        let mut rows = Vec::with_capacity(num_rows);
        for input in inputs {
            rows.extend(self.generate_trace_rows_for_perm(input));
        }

        for i in rows.len()..num_rows {
            let mut row = [F::ZERO; NUM_REGISTERS];
            self.generate_trace_rows_for_round(&mut row, i % NUM_ROUNDS);
            rows.push(row);
        }

        rows
    }

    fn generate_trace_rows_for_perm(
        &self,
        input: [u64; INPUT_LIMBS],
    ) -> [[F; NUM_REGISTERS]; NUM_ROUNDS] {
        let mut rows = [[F::ZERO; NUM_REGISTERS]; NUM_ROUNDS];

        for x in 0..5 {
            for y in 0..5 {
                let input_xy = input[x * 5 + y];
                for z in 0..64 {
                    rows[0][reg_a(x, y, z)] = F::from_canonical_u64((input_xy >> z) & 1);
                }
            }
        }

        self.generate_trace_rows_for_round(&mut rows[0], 0);
        for round in 1..24 {
            // TODO: Populate input from prev. row output.
            self.generate_trace_rows_for_round(&mut rows[round], round);
        }

        rows
    }

    fn generate_trace_rows_for_round(&self, row: &mut [F; NUM_REGISTERS], round: usize) {
        row[round] = F::ONE;

        // Populate C partial and C.
        for x in 0..5 {
            for z in 0..64 {
                let a = [0, 1, 2, 3, 4].map(|i| row[reg_a(x, i, z)]);
                let c_partial = xor([a[0], a[1], a[2]]);
                let c = xor([c_partial, a[3], a[4]]);
                row[reg_c_partial(x, z)] = c_partial;
                row[reg_c(x, z)] = c;
            }
        }

        // Populate A'.
        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for y in 0..5 {
                for z in 0..64 {
                    row[reg_a_prime(x, y, z)] = xor([
                        row[reg_a(x, y, z)],
                        row[reg_c((x + 4) % 5, z)],
                        row[reg_c((x + 1) % 5, (z + 1) % 64)],
                    ]);
                }
            }
        }

        // Populate A''.
        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        for x in 0..5 {
            for y in 0..5 {
                let get_bit = |z| {
                    xor([
                        row[reg_b(x, y, z)],
                        andn(row[reg_b((x + 1) % 5, y, z)], row[reg_b((x + 2) % 5, y, z)]),
                    ])
                };

                let lo = (0..32)
                    .rev()
                    .fold(F::ZERO, |acc, z| acc.double() + get_bit(z));
                let hi = (32..64)
                    .rev()
                    .fold(F::ZERO, |acc, z| acc.double() + get_bit(z));

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                row[reg_lo] = lo;
                row[reg_hi] = hi;
            }
        }

        // A''[0, 0] is additionally xor'd with RC.
        let reg_lo = reg_a_prime_prime(0, 0);
        let reg_hi = reg_lo + 1;
        let rc_lo = 0; // TODO
        let rc_hi = 0; // TODO
        row[reg_lo] = F::from_canonical_u64(row[reg_lo].to_canonical_u64() ^ rc_lo);
        row[reg_hi] = F::from_canonical_u64(row[reg_hi].to_canonical_u64() ^ rc_hi);
    }

    pub fn generate_trace(&self, inputs: Vec<[u64; INPUT_LIMBS]>) -> Vec<PolynomialValues<F>> {
        let mut timing = TimingTree::new("generate trace", log::Level::Debug);

        // Generate the witness, except for permuted columns in the lookup argument.
        let trace_rows = timed!(
            &mut timing,
            "generate trace rows",
            self.generate_trace_rows(inputs)
        );

        let trace_polys = timed!(
            &mut timing,
            "convert to PolynomialValues",
            trace_rows_to_poly_values(trace_rows)
        );

        timing.print();
        trace_polys
    }
}

impl Default for Keccak {
    fn default() -> Self {
        Self
    }
}

impl Stark<F, D> for Keccak {
    const COLUMNS: usize = NUM_REGISTERS;
    const PUBLIC_INPUTS: usize = NUM_PUBLIC_INPUTS;

    fn eval_packed_generic<FE, P, const D2: usize>(
        &self,
        vars: StarkEvaluationVars<FE, P, NUM_REGISTERS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut ConstraintConsumer<P>,
    ) where
        FE: FieldExtension<D2, BaseField = F>,
        P: PackedField<Scalar = FE>,
    {
        eval_round_flags(vars, yield_constr);

        // C_partial[x] = xor(A[x, 0], A[x, 1], A[x, 2])
        for x in 0..5 {
            for z in 0..64 {
                let c_partial = vars.local_values[reg_c_partial(x, z)];
                let a_0 = vars.local_values[reg_a(x, 0, z)];
                let a_1 = vars.local_values[reg_a(x, 1, z)];
                let a_2 = vars.local_values[reg_a(x, 2, z)];
                let xor_01 = xor_gen(a_0, a_1);
                let xor_012 = xor_gen(xor_01, a_2);
                yield_constr.constraint(c_partial - xor_012);
            }
        }

        // C[x] = xor(C_partial[x], A[x, 3], A[x, 4])
        for x in 0..5 {
            for z in 0..64 {
                let c = vars.local_values[reg_c(x, z)];
                let xor_012 = vars.local_values[reg_c_partial(x, z)];
                let a_3 = vars.local_values[reg_a(x, 3, z)];
                let a_4 = vars.local_values[reg_a(x, 4, z)];
                let xor_0123 = xor_gen(xor_012, a_3);
                let xor_01234 = xor_gen(xor_0123, a_4);
                yield_constr.constraint(c - xor_01234);
            }
        }

        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for z in 0..64 {
                let c_left = vars.local_values[reg_c((x + 4) % 5, z)];
                let c_right = vars.local_values[reg_c((x + 1) % 5, (z + 1) % 64)];
                let d = xor_gen(c_left, c_right);

                for y in 0..5 {
                    let a = vars.local_values[reg_a(x, y, z)];
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let xor = xor_gen(d, a);
                    yield_constr.constraint(a_prime - xor);
                }
            }
        }

        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        // A''[0, 0] is additionally xor'd with RC.
        for x in 0..5 {
            for y in 0..5 {
                let get_bit = |z| {
                    xor_gen(
                        vars.local_values[reg_b(x, y, z)],
                        andn_gen(
                            vars.local_values[reg_b((x + 1) % 5, y, z)],
                            vars.local_values[reg_b((x + 2) % 5, y, z)],
                        ),
                    )
                };

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                let lo = vars.local_values[reg_lo];
                let hi = vars.local_values[reg_hi];
                let computed_lo = (0..32)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));
                let computed_hi = (32..64)
                    .rev()
                    .fold(P::ZEROS, |acc, z| acc.doubles() + get_bit(z));

                yield_constr.constraint(computed_lo - lo);
                yield_constr.constraint(computed_hi - hi);
            }
        }

        let a_prime_prime_0_0_bits: Vec<_> = (0..64)
            .map(|i| vars.local_values[reg_a_prime_prime_0_0_bit(i)])
            .collect();
        let computed_a_prime_prime_0_0_lo = (0..32)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + a_prime_prime_0_0_bits[z]);
        let computed_a_prime_prime_0_0_hi = (32..64)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + a_prime_prime_0_0_bits[z]);
        let a_prime_prime_0_0_lo = vars.local_values[reg_a_prime_prime(0, 0)];
        let a_prime_prime_0_0_hi = vars.local_values[reg_a_prime_prime(0, 0) + 1];
        yield_constr.constraint(computed_a_prime_prime_0_0_lo - a_prime_prime_0_0_lo);
        yield_constr.constraint(computed_a_prime_prime_0_0_hi - a_prime_prime_0_0_hi);

        let get_xored_bit = |i| {
            let mut rc_bit_i = P::ZEROS;
            for r in 0..NUM_ROUNDS {
                let this_round = vars.local_values[reg_step(r)];
                let this_round_constant =
                    P::from(FE::from_canonical_u32(rc_value_bit(r, i) as u32));
                rc_bit_i += this_round * this_round_constant;
            }

            xor_gen(a_prime_prime_0_0_bits[i], rc_bit_i)
        };

        let a_prime_prime_prime_0_0_lo = vars.local_values[REG_A_PRIME_PRIME_PRIME_0_0_LO];
        let a_prime_prime_prime_0_0_hi = vars.local_values[REG_A_PRIME_PRIME_PRIME_0_0_HI];
        let computed_a_prime_prime_prime_0_0_lo = (0..32)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + get_xored_bit(z));
        let computed_a_prime_prime_prime_0_0_hi = (32..64)
            .rev()
            .fold(P::ZEROS, |acc, z| acc.doubles() + get_xored_bit(z));
        yield_constr.constraint(computed_a_prime_prime_prime_0_0_lo - a_prime_prime_prime_0_0_lo);
        yield_constr.constraint(computed_a_prime_prime_prime_0_0_hi - a_prime_prime_prime_0_0_hi);
    }

    fn eval_ext_recursively(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: StarkEvaluationTargets<D, NUM_REGISTERS, NUM_PUBLIC_INPUTS>,
        yield_constr: &mut RecursiveConstraintConsumer<F, D>,
    ) {
        let two = builder.two();

        eval_round_flags_recursively(builder, vars, yield_constr);

        // C_partial[x] = xor(A[x, 0], A[x, 1], A[x, 2])
        for x in 0..5 {
            for z in 0..64 {
                let c_partial = vars.local_values[reg_c_partial(x, z)];
                let a_0 = vars.local_values[reg_a(x, 0, z)];
                let a_1 = vars.local_values[reg_a(x, 1, z)];
                let a_2 = vars.local_values[reg_a(x, 2, z)];

                let xor_012 = xor3_gen_circuit(builder, a_0, a_1, a_2);
                let diff = builder.sub_extension(c_partial, xor_012);
                yield_constr.constraint(builder, diff);
            }
        }

        // C[x] = xor(C_partial[x], A[x, 3], A[x, 4])
        for x in 0..5 {
            for z in 0..64 {
                let c = vars.local_values[reg_c(x, z)];
                let xor_012 = vars.local_values[reg_c_partial(x, z)];
                let a_3 = vars.local_values[reg_a(x, 3, z)];
                let a_4 = vars.local_values[reg_a(x, 4, z)];

                let xor_01234 = xor3_gen_circuit(builder, xor_012, a_3, a_4);
                let diff = builder.sub_extension(c, xor_01234);
                yield_constr.constraint(builder, diff);
            }
        }

        // A'[x, y] = xor(A[x, y], D[x])
        //          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
        for x in 0..5 {
            for z in 0..64 {
                let c_left = vars.local_values[reg_c((x + 4) % 5, z)];
                let c_right = vars.local_values[reg_c((x + 1) % 5, (z + 1) % 64)];
                let d = xor_gen_circuit(builder, c_left, c_right);

                for y in 0..5 {
                    let a = vars.local_values[reg_a(x, y, z)];
                    let a_prime = vars.local_values[reg_a_prime(x, y, z)];
                    let xor = xor_gen_circuit(builder, d, a);
                    let diff = builder.sub_extension(a_prime, xor);
                    yield_constr.constraint(builder, diff);
                }
            }
        }

        // A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
        // A''[0, 0] is additionally xor'd with RC.
        for x in 0..5 {
            for y in 0..5 {
                let mut get_bit = |z| {
                    let andn = andn_gen_circuit(
                        builder,
                        vars.local_values[reg_b((x + 1) % 5, y, z)],
                        vars.local_values[reg_b((x + 2) % 5, y, z)],
                    );
                    xor_gen_circuit(builder, vars.local_values[reg_b(x, y, z)], andn)
                };

                let reg_lo = reg_a_prime_prime(x, y);
                let reg_hi = reg_lo + 1;
                let lo = vars.local_values[reg_lo];
                let hi = vars.local_values[reg_hi];
                let bits_lo = (0..32).map(|z| get_bit(z)).collect_vec();
                let bits_hi = (32..64).map(|z| get_bit(z)).collect_vec();
                let computed_lo = reduce_with_powers_ext_recursive(builder, bits_lo, two);
                let computed_hi = reduce_with_powers_ext_recursive(builder, bits_hi, two);
                let diff = builder.sub_extension(computed_lo, lo);
                yield_constr.constraint(builder, diff);
                let diff = builder.sub_extension(computed_hi, hi);
                yield_constr.constraint(builder, diff);
            }
        }
    }

    fn constraint_degree(&self) -> usize {
        3
    }
}
