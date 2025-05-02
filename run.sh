cargo bolero test bolero_tests::fuzz_smallest_power_of_two --runs 100000
cargo bolero test bolero_tests::fuzz_smallest_power_of_two --engine kani

cargo bolero test bolero_tests::fuzz_must_increase_cap --runs 100000
cargo bolero test bolero_tests::fuzz_must_increase_cap --engine kani

cargo kani --tests -Z concrete-playback --concrete-playback inplace

cargo kani --harness smallest_power_of_two_harness
cargo kani --harness smallest_power_of_two_harness -Z concrete-playback --concrete-playback inplace

cargo kani --harness must_increase_cap_harness
cargo kani --harness must_increase_cap_harness -Z concrete-playback --concrete-playback inplace
