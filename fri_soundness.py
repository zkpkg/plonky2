from sage.all import *

def soundness(num_functions,
              rate_bits,
              codeword_size_bits,
              m,
              arity_bits,
              num_rounds,
              field_size_bits,
              num_queries):

    rho = 1.0 / (2**rate_bits)
    alpha = sqrt(rho) * (1 + 1 / (2 * m))

    term_1 = pow(m + 0.5, 7) * 2 ** (2 * codeword_size_bits) / (2 * pow(rho, 1.5) * 2 ** field_size_bits)
    term_2 = (2 * m + 1) * (2**codeword_size_bits + 1) / sqrt(rho) * sum([2**x for x in arity_bits]) / 2**field_size_bits

    return term_1 + term_2 + pow(alpha, num_queries)
