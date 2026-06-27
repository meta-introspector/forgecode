#!/usr/bin/env python3
import json
from pathlib import Path

DASL_TESTING_DIR = Path("/mnt/data1/time-2026/02-february/22/dasl/dasl-testing")
CBOR_CPP_CORPUS_DIR = DASL_TESTING_DIR.parent / "fuzzing" / "corpus" / "cbor_cpp"

def collect_cbor_cpp_inputs():
    inputs = []
    if not CBOR_CPP_CORPUS_DIR.exists():
        return inputs
    for fpath in sorted(CBOR_CPP_CORPUS_DIR.iterdir()):
        if fpath.is_file():
            try:
                data = fpath.read_bytes()
                inputs.append({
                    "hex_data": data.hex(),
                    "source": f"cbor_cpp:{fpath.name}",
                    "test_name": "",
                    "test_type": "cbor_cpp",
                })
            except Exception:
                continue
    return inputs

if __name__ == "__main__":
    print(json.dumps(collect_cbor_cpp_inputs(), indent=2))
