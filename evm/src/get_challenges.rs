use plonky2::field::extension::Extendable;
use plonky2::fri::proof::{FriProof, FriProofTarget};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::challenger::{Challenger, RecursiveChallenger};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig};

use crate::all_stark::{AllStark, NUM_TABLES};
use crate::config::StarkConfig;
use crate::cross_table_lookup::get_grand_product_challenge_set;
use crate::proof::*;

impl<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize> AllProof<F, C, D> {
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllProofChallenges<F, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        // TODO: Observe public values.

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        let lookups = all_stark.num_lookups_helper_columns(config);

        AllProofChallenges {
            stark_challenges: core::array::from_fn(|i| {
                challenger.compact();
                self.stark_proofs[i]
                    .proof
                    .get_challenges(&mut challenger, lookups[i] > 0, config)
            }),
            ctl_challenges,
        }
    }

    #[allow(unused)] // TODO: should be used soon
    pub(crate) fn get_challenger_states(
        &self,
        all_stark: &AllStark<F, D>,
        config: &StarkConfig,
    ) -> AllChallengerState<F, D> {
        let mut challenger = Challenger::<F, C::Hasher>::new();

        for proof in &self.stark_proofs {
            challenger.observe_cap(&proof.proof.trace_cap);
        }

        // TODO: Observe public values.

        let ctl_challenges =
            get_grand_product_challenge_set(&mut challenger, config.num_challenges);

        let lookups = all_stark.num_lookups_helper_columns(config);

        let mut challenger_states = vec![challenger.compact()];
        for i in 0..NUM_TABLES {
            self.stark_proofs[i]
                .proof
                .get_challenges(&mut challenger, lookups[i] > 0, config);
            challenger_states.push(challenger.compact());
        }

        AllChallengerState {
            states: challenger_states.try_into().unwrap(),
            ctl_challenges,
        }
    }
}

impl<F, C, const D: usize> StarkProof<F, C, D>
where
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
{
    /// Computes all Fiat-Shamir challenges used in the STARK proof.
    pub(crate) fn get_challenges(
        &self,
        challenger: &mut Challenger<F, C::Hasher>,
        stark_use_lookup: bool,
        config: &StarkConfig,
    ) -> StarkProofChallenges<F, D> {
        let degree_bits = self.recover_degree_bits(config);

        let StarkProof {
            auxiliary_polys_cap,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProof {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let lookup_challenges =
            stark_use_lookup.then(|| challenger.get_n_challenges(config.num_challenges));

        challenger.observe_cap(auxiliary_polys_cap);

        let stark_alphas = challenger.get_n_challenges(num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge::<D>();

        challenger.observe_openings(&openings.to_fri_openings());

        StarkProofChallenges {
            lookup_challenges,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C, D>(
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                degree_bits,
                &config.fri_config,
            ),
        }
    }
}

impl<const D: usize> StarkProofTarget<D> {
    pub(crate) fn get_challenges<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>>(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        challenger: &mut RecursiveChallenger<F, C::Hasher, D>,
        stark_use_lookup: bool,
        config: &StarkConfig,
    ) -> StarkProofChallengesTarget<D>
    where
        C::Hasher: AlgebraicHasher<F>,
    {
        let StarkProofTarget {
            auxiliary_polys,
            quotient_polys_cap,
            openings,
            opening_proof:
                FriProofTarget {
                    commit_phase_merkle_caps,
                    final_poly,
                    pow_witness,
                    ..
                },
            ..
        } = &self;

        let num_challenges = config.num_challenges;

        let lookup_challenges =
            stark_use_lookup.then(|| challenger.get_n_challenges(builder, num_challenges));

        challenger.observe_cap(auxiliary_polys);

        let stark_alphas = challenger.get_n_challenges(builder, num_challenges);

        challenger.observe_cap(quotient_polys_cap);
        let stark_zeta = challenger.get_extension_challenge(builder);

        challenger.observe_openings(&openings.to_fri_openings(builder.zero()));

        StarkProofChallengesTarget {
            lookup_challenges,
            stark_alphas,
            stark_zeta,
            fri_challenges: challenger.fri_challenges::<C>(
                builder,
                commit_phase_merkle_caps,
                final_poly,
                *pow_witness,
                &config.fri_config,
            ),
        }
    }
}
