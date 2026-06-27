#!/usr/bin/env python3
import json
from pathlib import Path

DASL_TESTING_DIR = Path("/mnt/data1/time-2026/02-february/22/dasl/dasl-testing")
HARNESSES_DIR = DASL_TESTING_DIR / "harnesses"

def collect_crash_inputs():
    inputs = []
    for harness_dir in sorted(HARNESSES_DIR.iterdir()):
        crashes_dir = harness_dir / "crashes"
        if not crashes_dir.exists():
            continue
        # Collect .hex files
        for fpath in sorted(crashes_dir.glob("*.hex")):
            try:
                hex_data = fpath.read_text().strip()
                if hex_data:
                    inputs.append({
                        "hex_data": hex_data,
                        "source": f"crash:{harness_dir.name}/{fpath.name}",
                        "test_name": "",
                        "test_type": "crash",
                    })
            except Exception:
                continue
        # Collect .bin files (no .hex companion)
        for fpath in sorted(crashes_dir.glob("*.bin")):
            hex_companion = fpath.with_suffix(".hex")
            if hex_companion.exists():
                continue  # Already collected via .hex
            try:
                data = fpath.read_bytes()
                inputs.append({
                    "hex_data": data.hex(),
                    "source": f"crash:{harness_dir.name}/{fpath.name}",
                    "test_name": "",
                    "test_type": "crash",
                })
            except Exception:
                continue
    # Java crashes (raw binary files, no extension)
    java_dir = HARNESSES_DIR / "java-dag-cbor"
    if java_dir.exists():
        for fpath in sorted(java_dir.iterdir()):
            if fpath.name.startswith("crash-") and fpath.is_file() and not fpath.suffix:
                try:
                    data = fpath.read_bytes()
                    if data:  # Skip empty
                        inputs.append({
                            "hex_data": data.hex(),
                            "source": f"crash:java-dag-cbor/{fpath.name}",
                            "test_name": "",
                            "test_type": "crash",
                        })
                except Exception:
                    continue
    return inputs

if __name__ == "__main__":
    print(json.dumps(collect_crash_inputs(), indent=2))
