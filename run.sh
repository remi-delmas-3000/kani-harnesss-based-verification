cargo bolero test bolero_tests::fuzz_smallest_power_of_two --runs 100000
cargo bolero test bolero_tests::fuzz_smallest_power_of_two --engine kani
cargo bolero test bolero_tests::fuzz_must_increase_cap --runs 100000
cargo bolero test bolero_tests::fuzz_must_increase_cap --engine kani
cargo bolero test bolero_tests::fuzz_push_pop --runs 100000
cargo bolero test bolero_tests::fuzz_push_capacity --runs 100000
cargo bolero test bolero_tests::fuzz_resize --runs 100000
cargo bolero test bolero_tests::fuzz_transition_to_heap --runs 100000
