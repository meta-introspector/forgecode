#!/usr/bin/env python3
import json
from pathlib import Path

DASL_TESTING_DIR = Path("/mnt/data1/time-2026/02-february/22/dasl/dasl-testing")
ATTACK_CORPUS_DIR = DASL_TESTING_DIR.parent / "INPUT" / "attacks"

def collect_timing_attack_inputs():
    inputs = []
    if not ATTACK_CORPUS_DIR.exists():
        return inputs
    for fpath in sorted(ATTACK_CORPUS_DIR.glob("*.bin")):
        try:
            data = fpath.read_bytes()
            inputs.append({
                "hex_data": data.hex(),
                "source": f"timing:{fpath.name}",
                "test_name": "",
                "test_type": "timing",
            })
        except Exception:
            continue
    return inputs

if __name__ == "__main__":
    print(json.dumps(collect_timing_attack_inputs(), indent=2))
