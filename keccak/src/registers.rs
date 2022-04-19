use crate::keccak::NUM_ROUNDS;

/// A register which is set to 1 if we are in the `i`th round, otherwise 0.
pub(crate) const fn reg_step(i: usize) -> usize {
    debug_assert!(i < NUM_ROUNDS);
    i
}

const R: [[u8; 5]; 5] = [
    [0, 18, 41, 3, 36],
    [1, 2, 45, 10, 44],
    [62, 61, 15, 43, 6],
    [28, 56, 21, 25, 55],
    [27, 14, 8, 39, 20],
];

const START_A: usize = NUM_ROUNDS;
pub(crate) const fn reg_a(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    debug_assert!(z < 64);
    START_A + x * 64 * 5 + y * 64 + z
}

// C_partial[x] = xor(A[x, 0], A[x, 1], A[x, 2])
const START_C_PARTIAL: usize = START_A + 5 * 5 * 64;
pub(crate) const fn reg_c_partial(x: usize, z: usize) -> usize {
    START_C_PARTIAL + x * 64 + z
}

// C[x] = xor(C_partial[x], A[x, 3], A[x, 4])
const START_C: usize = START_C_PARTIAL + 5 * 64;
pub(crate) const fn reg_c(x: usize, z: usize) -> usize {
    START_C + x * 64 + z
}

// D is inlined.
// const fn reg_d(x: usize, z: usize) {}

// A'[x, y] = xor(A[x, y], D[x])
//          = xor(A[x, y], C[x - 1], ROT(C[x + 1], 1))
const START_A_PRIME: usize = START_C + 5 * 64;
pub(crate) const fn reg_a_prime(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    debug_assert!(z < 64);
    START_A_PRIME + x * 64 * 5 + y * 64 + z
}

pub(crate) const fn reg_b(x: usize, y: usize, z: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    debug_assert!(z < 64);
    // B is just a rotation of A', so these are aliases for A' registers.
    // From the spec,
    //     B[y, (2x + 3y) % 5] = ROT(A'[x, y], r[x, y])
    // So,
    //     B[x, y] = f((x + 3y) % 5, x)
    // where f(a, b) = ROT(A'[a, b], r[a, b])
    let a = (x + 3 * y) % 5;
    let b = x;
    let rot = R[a][b] as usize;
    reg_a_prime(a, b, (z + rot) % 64)
}

// A''[x, y] = xor(B[x, y], andn(B[x + 1, y], B[x + 2, y])).
// A''[0, 0] is additionally xor'd with RC.
const START_A_PRIME_PRIME: usize = START_A_PRIME + 5 * 5 * 64;
pub(crate) const fn reg_a_prime_prime(x: usize, y: usize) -> usize {
    debug_assert!(x < 5);
    debug_assert!(y < 5);
    START_A_PRIME_PRIME + x * 2 * 5 + y * 2
}

pub(crate) const NUM_REGISTERS: usize = START_A_PRIME_PRIME + 5 * 5 * 2;
