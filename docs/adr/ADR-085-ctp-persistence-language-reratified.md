# ADR-085: English-Language CTP Persistence Re-Ratified by Owner, Superseding ADR-084

Status: ACCEPTED
Date: 2026-10-06
Session: SESSION-ARC-007, Reply 15

## Context

ADR-084 (SESSION-ARC-007, Reply 13) unilaterally decided, on the architect's own
authority, that persisted CTP files would switch to Arabic starting with
CTP-SESSION-ARC-008.md, reasoning that CHARTER-v2.0 Section 15.1's literal
wording ("you MUST produce a complete CTP in Arabic") overrides the
never-formally-ratified ADR-080 (English CTP practice, followed in practice
through CTP-SESSION-ARC-001.md to CTP-SESSION-ARC-007.md).

Following ADR-084's own commitment (Section 2.3's mandatory structure requires
putting such decisions before the owner for objection), the architect presented
this decision to the owner in SESSION-ARC-007 Reply 13's report and explicitly
asked whether the owner objected. The owner responded directly and
unambiguously: the owner wants persisted CTP content to remain in English,
written with high technical precision, professionalism, efficiency, and great
care.

## Decision

ADR-084 is superseded. Persisted CTP file content continues to be authored in
English, exactly as ADR-080 originally established and as practiced in
CTP-SESSION-ARC-001.md through CTP-SESSION-ARC-007.md, including
CTP-SESSION-ARC-008.md and all future sessions' CTP files, unless and until the
owner gives a different explicit instruction.

The owner-facing in-reply summary (Section 1 of every Triad Protocol reply)
remains in Modern Standard Arabic, unchanged, per CHARTER-v2.0 Section 2.2 —
this ADR concerns only the language of the PERSISTED CTP MARKDOWN FILE content,
not the architect's own replies to the owner, which were never in question.

## Reasons

- CHARTER-v2.0 Section 1.2 reserves "taste" and the weighing of a document's
  professional register/language of internal technical artifacts to decisions
  the owner is entitled to make directly, once asked — and the owner was asked
  and answered directly, which the architect did not have in hand at the time
  ADR-084 was written.
- The owner explicitly values the English CTP for its technical precision and
  professional register for a document that is overwhelmingly composed of
  literal code, SQL, file paths, hashes, and git output — content for which
  English technical register is the architect's and the industry's working
  norm, and for which an Arabic rendering would risk exactly the kind of
  translation ambiguity the owner's own terminology guidance (see the owner's
  Reply-14 answer on the five glossary terms) explicitly warns against.
- CHARTER-v2.0 Section 14.1 instructs that, where source documents contradict
  the charter, "this charter wins except for engine invariants they confirm" —
  but that clause governs conflicts between INHERITED DOCUMENTS and the
  charter, not a live, explicit, current instruction from the owner exercising
  the owner's own reserved authority under Section 1.2. The architect's
  ADR-084 reasoning conflated these two different situations; this ADR
  corrects that error explicitly.

## Consequences

- CTP-SESSION-ARC-008.md (and every subsequent session's CTP, unless a future
  owner instruction changes this again) is authored in English, matching the
  established CTP-001 through CTP-007 practice.
- ADR-084's own file is NOT deleted (ADRs are not edited away once committed);
  its Status line alone is updated to point here, per this same task's second
  deliverable, so the historical record of the architect's reasoning error and
  its correction remains fully visible and auditable.
- The owner-facing in-reply Arabic summary practice (Section 2.2) is entirely
  unaffected by this ADR and continues exactly as before.
- This episode is recorded in CTP-SESSION-ARC-007.md's Errors & Lessons log
  (Section 10) at the next checkpoint, as an architect self-correction: the
  architect must present ANY decision affecting the OWNER's own reserved
  authority (taste, business register, language of deliverables addressed to
  or read primarily by the owner) as a QUESTION before acting, not as a
  unilateral ruling subject only to a retroactive objection window — ADR-084's
  "I will do X unless you object" framing (CTP-SESSION-ARC-007.md Reply 13)
  should instead have been phrased as "I recommend X, please confirm" before
  any ADR was written or filed as "ACCEPTED".

## References

- ADR-080 (unfiled proposal, SESSION-ARC-005).
- ADR-084 (docs/adr/ADR-084-ctp-persistence-language.md), now superseded.
- CHARTER-v2.0, Section 1.2, Section 2.2, Section 14.1, Section 15.1.
- CTP-SESSION-ARC-007.md, Reply 13 (ADR-084 proposal) and Reply 15 (owner's
  direct ruling and this reversal).
