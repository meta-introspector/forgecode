#!/usr/bin/env python3
"""
Scan forge conversation fixtures for uncertainty markers, questions, and
decision points. Compiles everything into structured datasets for
training/testing the DecisionCaptureHandler.

Outputs:
  test/fixtures/forge-trace/analysis/
    uncertainty_dataset.json    — all extracted patterns with context
    uncertainty_stats.json      — aggregate statistics
    question_corpus.json        — user questions for training
    decision_point_corpus.json  — "I need to either" type patterns
"""

import json
import glob
import re
import os
from collections import Counter, defaultdict
from pathlib import Path

FIXTURE_DIR = Path("test/fixtures/forge-trace")
OUT_DIR = FIXTURE_DIR / "analysis"
OUT_DIR.mkdir(parents=True, exist_ok=True)

# ─── Pattern Definitions ──────────────────────────────────────────────────

UNCERTAINTY_PATTERNS = [
    # Direct uncertainty
    (r"(?i)\bnot sure\b", "not_sure"),
    (r"(?i)\buncertain\b", "uncertain"),
    (r"(?i)\bnot certain\b", "not_certain"),
    (r"(?i)\bnot confident\b", "not_confident"),
    (r"(?i)\bmaybe\b", "maybe"),
    (r"(?i)\bperhaps\b", "perhaps"),
    (r"(?i)\b(I'm|I am)\s+(not\s+sure|uncertain|undecided)", "i_am_uncertain"),
    # Hedged language
    (r"(?i)\bI think\b.*\bbut\b", "hedged_think_but"),
    (r"(?i)\bseems?\s+like\b", "seems_like"),
    (r"(?i)\b(?:might|may|could)\s+(?:be|work|try|want)\b", "tentative"),
    (r"(?i)\bpossibly\b", "possibly"),
    (r"(?i)\b(?:not\s+sure|hard\s+to\s+say|difficult\s+to)\b", "evasion"),
    (r"(?i)\b(?:I'd|I\s+would)\s+(?:guess|imagine|think|say)\b", "hedged_guess"),
    # Decision fork markers
    (r"(?i)\b(either|neither)\s+\w+\s+(or|nor)\b", "either_or"),
    (r"(?i)\b(?:need\s+to\s+decide|need\s+to\s+choose|have\s+to\s+pick)\b", "need_decide"),
    (r"(?i)\b(?:option|alternative)s?\b", "option_alternative"),
    (r"(?i)\b(?:weigh|compare|trade.?off)\b", "tradeoff"),
    (r"(?i)\b(?:depend|depends)\s+on\b", "depends"),
    (r"(?i)\bnot sure (?:which|what|how|whether|if)\b", "uncertain_which"),
    (r"(?i)\bthis or that\b", "this_or_that"),
]

QUESTION_PATTERNS = [
    # Direct questions
    (r"(?i)\bshould\s+(?:I|we|it)\b", "should_we"),
    (r"(?i)\b(?:what|which)\s+(?:should|would|do|does|is|are)\b", "what_which"),
    (r"(?i)\b(?:can|could|would|will|do|does|did)\s+(?:I|we|you)\b", "can_could"),
    (r"(?i)\bhow\s+(?:do|does|would|should|can|could|about)\b", "how_to"),
    (r"(?i)\bwhy\s+(?:not|don't|doesn't|isn't|wouldn't)\b", "why_not"),
    (r"(?i)\bis\s+(?:it|that|this|there)\b", "is_it"),
    (r"(?i)\?$", "question_mark_end"),
    # Explicit requests
    (r"(?i)\b(?:can|could)\s+you\s+(?:help|show|tell|explain|check|review)\b", "request_help"),
    (r"(?i)\b(?:please|pls)\s+(?:explain|show|tell|check)\b", "polite_request"),
    (r"(?i)\b(?:let me know|lmk)\b", "let_me_know"),
    # Uncertainty as question
    (r"(?i)\b(?:what do you think|wdyt|what's your take)\b", "opinion_ask"),
]

# Combined feedback tag pattern
FEEDBACK_RE = re.compile(r"<feedback>(.*?)</feedback>", re.DOTALL)
TASK_RE = re.compile(r"<task>(.*?)</task>", re.DOTALL)


# ─── Extractors ────────────────────────────────────────────────────────────

def extract_uncertainty(text: str) -> list[dict]:
    """Return all matching uncertainty patterns with positions."""
    results = []
    for pattern, label in UNCERTAINTY_PATTERNS:
        for m in re.finditer(pattern, text):
            start = max(0, m.start() - 40)
            end = min(len(text), m.end() + 100)
            context = text[start:end].strip()
            results.append({
                "pattern": label,
                "match": m.group(),
                "pos_start": m.start(),
                "pos_end": m.end(),
                "context": context,
            })
    return results


def extract_questions(text: str) -> list[dict]:
    """Return all matching question patterns with positions."""
    results = []
    for pattern, label in QUESTION_PATTERNS:
        for m in re.finditer(pattern, text):
            snippet = text[max(0, m.start() - 200):m.end() + 50]
            if "<system_information>" in snippet:
                continue
            start = max(0, m.start() - 30)
            end = min(len(text), m.end() + 80)
            context = text[start:end].strip()
            results.append({
                "pattern": label,
                "match": m.group(),
                "pos_start": m.start(),
                "context": context,
            })
    return results


def extract_decision_points(text: str) -> list[dict]:
    """Specifically extract 'I need to either' and similar decision forks."""
    patterns = [
        (r"(?i)I need to either\b", "need_to_either"),
        (r"(?i)\bneed to decide\b", "need_to_decide"),
        (r"(?i)\b(?:choose|pick|select)\s+(?:between|from)\b", "choose_between"),
        (r"(?i)\balternatively\b", "alternatively"),
        (r"(?i)\b(?:on the one hand|on the other hand)\b", "on_the_other_hand"),
        (r"(?i)\bbetter\s+(?:to|off)\b", "better_to"),
    ]
    results = []
    for pattern, label in patterns:
        for m in re.finditer(pattern, text):
            start = max(0, m.start() - 30)
            end = min(len(text), m.end() + 120)
            context = text[start:end].strip()
            results.append({
                "pattern": label,
                "match": m.group(),
                "pos_start": m.start(),
                "context": context,
            })
    return results


# ─── Analyzer ──────────────────────────────────────────────────────────────

def analyze_conversation(conv: dict) -> dict:
    """Run all pattern extractors on a single conversation."""
    cid = conv["conversation_id"]
    title = conv["title"]
    result = {
        "conversation_id": cid,
        "title": title,
        "msg_count": conv["msg_count"],
        "tool_count": len(conv.get("tools_used", [])),
        "uncertainty": [],
        "questions": [],
        "decision_points": [],
        "feedback_uncertainty": [],
    }

    for msg in conv["messages"]:
        text = msg["content_preview"]
        if not text or len(text) < 10:
            continue
        role = msg["role"]
        content_len = msg["content_len"]

        unc = extract_uncertainty(text)
        if unc:
            result["uncertainty"].extend([
                {**u, "role": role, "content_len": content_len}
                for u in unc
            ])

        qs = extract_questions(text)
        if qs:
            result["questions"].extend([
                {**q, "role": role, "content_len": content_len}
                for q in qs
            ])

        dp = extract_decision_points(text)
        if dp:
            result["decision_points"].extend([
                {**d, "role": role, "content_len": content_len}
                for d in dp
            ])

        for fb_match in FEEDBACK_RE.finditer(text):
            fb_text = fb_match.group(1).strip()
            if len(fb_text) > 10:
                fb_unc = extract_uncertainty(fb_text)
                if fb_unc:
                    result["feedback_uncertainty"].extend([
                        {**u, "feedback_context": fb_text[:300]}
                        for u in fb_unc
                    ])

    return result


# ─── Main ──────────────────────────────────────────────────────────────────

def main():
    fixtures = sorted(glob.glob(str(FIXTURE_DIR / "conversations" / "*.json")))
    print(f"Analyzing {len(fixtures)} conversation fixtures...")

    all_analyses = []
    pattern_counts = Counter()
    question_counts = Counter()
    decision_counts = Counter()
    role_counts = Counter()
    feedback_uncert_count = 0
    conversations_with_uncertainty = 0
    conversations_with_questions = 0
    conversations_with_decisions = 0

    for fpath in fixtures:
        with open(fpath) as f:
            conv = json.load(f)

        analysis = analyze_conversation(conv)
        all_analyses.append(analysis)

        if analysis["uncertainty"]:
            conversations_with_uncertainty += 1
        if analysis["questions"]:
            conversations_with_questions += 1
        if analysis["decision_points"]:
            conversations_with_decisions += 1
        if analysis["feedback_uncertainty"]:
            feedback_uncert_count += 1

        for u in analysis["uncertainty"]:
            pattern_counts[u["pattern"]] += 1
            role_counts[("uncertainty", u["role"])] += 1

        for q in analysis["questions"]:
            question_counts[q["pattern"]] += 1
            role_counts[("question", q["role"])] += 1

        for d in analysis["decision_points"]:
            decision_counts[d["pattern"]] += 1
            role_counts[("decision", d["role"])] += 1

    total_uncertainty = sum(pattern_counts.values())
    total_questions = sum(question_counts.values())
    total_decisions = sum(decision_counts.values())

    stats = {
        "conversations_scanned": len(fixtures),
        "with_uncertainty": conversations_with_uncertainty,
        "with_questions": conversations_with_questions,
        "with_decisions": conversations_with_decisions,
        "with_feedback_uncertainty": feedback_uncert_count,
        "total_uncertainty_hits": total_uncertainty,
        "total_question_hits": total_questions,
        "total_decision_hits": total_decisions,
        "uncertainty_by_pattern": dict(pattern_counts.most_common()),
        "questions_by_pattern": dict(question_counts.most_common()),
        "decisions_by_pattern": dict(decision_counts.most_common()),
        "hits_by_role": {f"{k[0]}::{k[1]}": v for k, v in role_counts.most_common()},
        "avg_uncertainty_per_conv": round(total_uncertainty / len(fixtures), 2),
        "avg_questions_per_conv": round(total_questions / len(fixtures), 2),
    }

    def build_examples(analyses, key, max_examples=200):
        examples = []
        for a in analyses:
            for item in a[key]:
                examples.append({
                    "conversation_id": a["conversation_id"],
                    "title": a["title"],
                    "pattern": item["pattern"],
                    "match": item["match"],
                    "context": item["context"][:250],
                    "role": item.get("role", "?"),
                })
                if len(examples) >= max_examples:
                    break
            if len(examples) >= max_examples:
                break
        return examples

    uncertainty_examples = build_examples(all_analyses, "uncertainty", 300)
    question_examples = build_examples(all_analyses, "questions", 300)
    decision_examples = build_examples(all_analyses, "decision_points", 200)

    dataset = {
        "stats": stats,
        "uncertainty_examples": uncertainty_examples,
        "question_examples": question_examples,
        "decision_examples": decision_examples,
    }

    with open(OUT_DIR / "uncertainty_dataset.json", "w") as f:
        json.dump(dataset, f, indent=2, default=str)

    question_corpus = []
    for a in all_analyses:
        for q in a["questions"]:
            if q["role"] == "User":
                question_corpus.append({
                    "context": q["context"][:200],
                    "pattern": q["pattern"],
                    "conversation": a["title"],
                })
    with open(OUT_DIR / "question_corpus.json", "w") as f:
        json.dump(question_corpus[:500], f, indent=2, default=str)

    dp_corpus = []
    for a in all_analyses:
        for d in a["decision_points"]:
            if d.get("pattern") in ("need_to_either", "choose_between", "alternatively"):
                dp_corpus.append({
                    "context": d["context"][:300],
                    "pattern": d["pattern"],
                    "conversation": a["title"],
                })
    with open(OUT_DIR / "decision_point_corpus.json", "w") as f:
        json.dump(dp_corpus[:200], f, indent=2, default=str)

    print(f"\n{'='*60}")
    print("Uncertainty & Question Analysis Complete")
    print(f"{'='*60}")
    print(f"Scanned:          {len(fixtures)} conversations")
    print(f"With uncertainty: {conversations_with_uncertainty} ({total_uncertainty} hits)")
    print(f"With questions:   {conversations_with_questions} ({total_questions} hits)")
    print(f"With decisions:   {conversations_with_decisions} ({total_decisions} hits)")
    print(f"Feedback uncert:  {feedback_uncert_count} conversations")
    print()
    print("Top uncertainty patterns:")
    for pat, cnt in pattern_counts.most_common(10):
        print(f"  {pat:25s} {cnt:4d}")
    print()
    print("Top question patterns:")
    for pat, cnt in question_counts.most_common(8):
        print(f"  {pat:25s} {cnt:4d}")
    print()
    print("Top decision patterns:")
    for pat, cnt in decision_counts.most_common(8):
        print(f"  {pat:25s} {cnt:4d}")
    print()
    print(f"Output: {OUT_DIR}/")


if __name__ == "__main__":
    main()
