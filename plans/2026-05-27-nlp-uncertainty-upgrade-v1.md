# NLP Upgrade for Uncertainty Analysis

## Objective

Replace the current regex-based pattern matching in `scripts/analyze_uncertainty.py` with standard NLP tools (NLTK, spaCy, TextBlox) to improve detection accuracy, reduce false positives, and enable richer uncertainty profiling. The upgrade will be reproducible via Nix by adding Python NLP packages to the flake devShell.

## Implementation Plan

- [ ] 1. **Add Python NLP packages to flake.nix devShell**

    Add `pkgs.python3`, `pkgs.python3Packages.nltk`, `pkgs.python3Packages.spacy`, `pkgs.python3Packages.textblob`, and a spaCy model to `devShells.default.packages` in `flake.nix:154-170`. This ensures the NLP analysis environment is reproducible and available to all developers via `nix develop`.

- [ ] 2. **Replace regex sentence splitting with NLTK `sent_tokenize`**

    In `scripts/analyze_uncertainty.py`, replace the current approach of scanning each message's raw text for regex patterns. Instead, use NLTK's `sent_tokenize` (Punkt tokenizer) to split messages into proper sentences first. This gives sentence-boundary-aware analysis and avoids false matches across sentence boundaries (e.g., matching "I think" and "but" across unrelated sentences).

- [ ] 3. **Add POS-tag-aware uncertainty detection with NLTK `pos_tag`**

    Replace keyword-based matching for modal verbs (`might`, `may`, `could`, `would`) with NLTK POS tagging. Detect modals by their POS tag (MD) rather than substring matching, which eliminates false positives from code snippets or variable names containing these words. Also tag hedge verbs and adverbs by POS for more precise detection.

- [ ] 4. **Add TextBlob subjectivity scoring per sentence**

    Integrate TextBlob's `sentiment.subjectivity` metric (0.0 = factual, 1.0 = opinion) to quantify how hedged each sentence is. This provides a continuous uncertainty score rather than binary pattern matches. Collect subjectivity histograms per conversation and per role (User vs Assistant) to profile which kinds of messages tend toward uncertainty.

- [ ] 5. **Add spaCy dependency parsing for cross-clause uncertainty patterns**

    Use spaCy's dependency parse to detect patterns that span clause boundaries, such as "I think X, but I'm not sure about Y". The dependency tree allows detecting subject--verb relationships and adversative clauses reliably, replacing the current `r"(?i)\bI think\b.*\bbut\b"` regex which can match across unrelated clauses.

- [ ] 6. **Add NLP-based section chunking for the reasoning pager**

    Replace the current `\n\n` paragraph boundary detection in the reasoning pager (used to build the histogram sections) with NLTK's sentence clustering or spaCy's topic segmentation. This produces more semantically meaningful sections for the reasoning viewer's numbered jump-to-section feature.

- [ ] 7. **Rerun analysis and produce comparison report**

    Run the upgraded NLP analysis against the same 109 conversation fixtures. Generate a comparison showing detection rate changes: how many more/ fewer uncertainty hits, how many false positives eliminated, and how the subjectivity distributions compare to the regex baseline.

## Verification Criteria

- All 109 conversation fixtures are analyzed with zero Python errors
- NLTK, spaCy, and TextBlob imports succeed within `nix develop` shell
- At least 2x improvement in uncertainty detection recall over the current regex baseline (measured against a hand-labeled sample of 20 conversations)
- False positive rate (verified by manual spot-check of 50 matched examples) is below 10%
- Output JSON schema remains backward-compatible with any consumers of `test/fixtures/forge-trace/analysis/uncertainty_dataset.json`
- `nix build .#devShells.x86_64-linux.default` succeeds after flake.nix changes

## Potential Risks and Mitigations

1. **spaCy model download in Nix is large (100MB+)**
   Mitigation: Use `pkgs.python3Packages.spacyModels.en_core_web_sm` (small model, ~12MB) for initial implementation. Fall back to NLTK-only analysis if spaCy model building fails in constrained environments.

2. **NLTK data download requires network access**
   Mitigation: Pre-download Punkt tokenizer data and package it as a Nix derivation, or structure the script to download on first run with a cached local directory (`~/nltk_data/`).

3. **NLP tools are slower than regex by orders of magnitude**
   Mitigation: Run spaCy in batch mode with `nlp.pipe()` over all messages at once. For 109 conversations with ~50 messages each (~5,450 messages total), spaCy processes ~1,000 sentences/second, completing the full analysis in under 30 seconds.

4. **Subjectivity scores from TextBlob are noisy on technical code text**
   Mitigation: Filter out sentences containing code snippets (detected by presence of `{`, `=`, `=>`, `fn`, `pub`, `impl`, etc.) before scoring subjectivity. This prevents code from skewing the uncertainty signal.

## Alternative Approaches

1. **transformers-only (no spaCy/NLTK)**: Use a pre-trained sequence classifier from Hugging Face (e.g., `roberta-base-uncertainty`) fine-tuned on hedge/uncertainty detection. Trade-off: better accuracy but requires GPU for practical speed and adds 500MB+ to the Nix closure.

2. **scikit-learn feature engineering only**: Extract TF-IDF features plus hand-crafted uncertainty lexicons and train a logistic regression classifier. Trade-off: lighter dependencies (scikit-learn already available) but requires labeled training data which doesn't yet exist.

3. **Hybrid regex+NLP**: Keep regex for known-high-precision patterns (like `"not sure"`) and layer NLP only for ambiguous patterns (like cross-clause hedging). Trade-off: lower implementation effort but misses the systematic improvement of a full NLP pipeline.
