#!/usr/bin/env python3
import json
from pathlib import Path

DASL_TESTING_DIR = Path("/mnt/data1/time-2026/02-february/22/dasl/dasl-testing")
LATTICE_FIXTURES = DASL_TESTING_DIR / "INPUT" / "lattice" / "fixtures" / "lattice.json"

def collect_lattice_inputs():
    inputs = []
    if not LATTICE_FIXTURES.exists():
        return inputs
    try:
        tests = json.loads(LATTICE_FIXTURES.read_text())
    except Exception:
        return inputs
    for entry in tests:
        if "data" in entry:
            inputs.append({
                "hex_data": entry["data"],
                "source": f"lattice:{entry.get('name', 'unknown')}",
                "test_name": entry.get("name", ""),
                "test_type": "lattice",
            })
    return inputs

if __name__ == "__main__":
    print(json.dumps(collect_lattice_inputs(), indent=2))
