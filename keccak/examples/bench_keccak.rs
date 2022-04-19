use itertools::Itertools;
use keccak::keccak::Keccak;
use log::Level;
use plonky2::field::field_types::Field;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::fri::FriConfig;
use plonky2::iop::witness::PartialWitness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::util::timing::TimingTree;
use rand::Rng;
use starky::config::StarkConfig;
use starky::prover::prove;
use starky::recursive_verifier::{
    add_virtual_stark_proof_with_pis, recursively_verify_stark_proof,
    set_stark_proof_with_pis_target,
};
use starky::stark::Stark;
use starky::verifier::verify_stark_proof;

type S = Keccak;
type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

const NUM_INPUTS: usize = 85;

fn main() -> anyhow::Result<()> {
    let _ = env_logger::builder().format_timestamp(None).try_init();

    let inputs = (0..NUM_INPUTS).map(|_| rand_input()).collect_vec();

    let stark = S::default();
    let public_inputs = [F::ZERO; S::PUBLIC_INPUTS];
    let inner_config = StarkConfig {
        fri_config: FriConfig {
            proof_of_work_bits: 15,
            num_query_rounds: 85,
            ..StarkConfig::standard_fast_config().fri_config
        },
        ..StarkConfig::standard_fast_config()
    };
    let mut timing = TimingTree::new("prove", Level::Debug);
    let trace = stark.generate_trace(inputs);
    let inner_proof = prove::<F, C, S, D>(stark, &inner_config, trace, public_inputs, &mut timing)?;
    let inner_degree_bits = inner_proof.proof.recover_degree_bits(&inner_config);
    timing.print();

    verify_stark_proof(stark, inner_proof.clone(), &inner_config)?;

    let circuit_config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::new(circuit_config);
    let pt =
        add_virtual_stark_proof_with_pis(&mut builder, stark, &inner_config, inner_degree_bits);
    let mut pw = PartialWitness::new();
    set_stark_proof_with_pis_target(&mut pw, &pt, &inner_proof);
    recursively_verify_stark_proof::<F, C, S, D>(&mut builder, stark, pt, &inner_config);
    builder.print_gate_counts(0);

    let data = builder.build::<C>();
    let proof = data.prove(pw)?;
    data.verify(proof)
}

fn rand_input() -> [u64; 25] {
    let mut input = [0; 25];
    rand::thread_rng().fill(&mut input);
    input
}
