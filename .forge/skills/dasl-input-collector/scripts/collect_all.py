#!/usr/bin/env python3
import json
import argparse
from collect_fixtures import collect_fixture_inputs
from collect_raw_corpus import collect_raw_corpus_inputs
from collect_adversarial import collect_adversarial_inputs
from collect_crashes import collect_crash_inputs
from collect_timing_attacks import collect_timing_attack_inputs
from collect_cbor_cpp import collect_cbor_cpp_inputs
from collect_lattice import collect_lattice_inputs

def collect_all_inputs(input_sets: set[str]):
    """Collect inputs from specified sets, deduplicate by hex data."""
    collectors = {
        "fixtures": collect_fixture_inputs,
        "raw": collect_raw_corpus_inputs,
        "adversarial": collect_adversarial_inputs,
        "crashes": collect_crash_inputs,
        "timing": collect_timing_attack_inputs,
        "cbor_cpp": collect_cbor_cpp_inputs,
        "lattice": collect_lattice_inputs,
    }
    all_inputs = []
    seen_hex = set()

    sets_to_collect = input_sets
    if "all" in input_sets:
        sets_to_collect = collectors.keys()

    for name in sets_to_collect:
        if name in collectors:
            all_inputs.extend(collectors[name]())

    # Deduplicate by hex data
    unique = []
    for item in all_inputs:
        if item["hex_data"] not in seen_hex:
            seen_hex.add(item["hex_data"])
            unique.append(item)
    return unique

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Collect and deduplicate CBOR test inputs.")
    parser.add_argument(
        "--inputs", default="all",
        help="Comma-separated input sets: fixtures,raw,adversarial,crashes,timing,cbor_cpp,lattice,all (default: all)"
    )
    args = parser.parse_args()
    input_sets = set(args.inputs.split(","))
    
    all_inputs = collect_all_inputs(input_sets)
    print(json.dumps(all_inputs, indent=2))
