#!/usr/bin/env python3
import json
from pathlib import Path

DASL_TESTING_DIR = Path("/mnt/data1/time-2026/02-february/22/dasl/dasl-testing")
FIXTURES_DIR = DASL_TESTING_DIR / "fixtures" / "cbor"

def collect_fixture_inputs():
    inputs = []
    if not FIXTURES_DIR.exists():
        return inputs
    for fpath in sorted(FIXTURES_DIR.glob("*.json")):
        try:
            tests = json.loads(fpath.read_text())
        except Exception:
            continue
        for entry in tests:
            if "data" in entry:
                inputs.append({
                    "hex_data": entry["data"],
                    "source": f"fixture:{fpath.name}",
                    "test_name": entry.get("name", ""),
                    "test_type": entry.get("type", ""),
                })
    return inputs

if __name__ == "__main__":
    print(json.dumps(collect_fixture_inputs(), indent=2))
