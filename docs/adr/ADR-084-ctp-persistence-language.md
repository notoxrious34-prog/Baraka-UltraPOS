# ADR-084: Persisted CTP Files Are Authored in Arabic, Superseding ADR-080

Status: ACCEPTED (supersedes ADR-080)
Date: 2026-10-06
Session: SESSION-ARC-007, Reply 13

## Context

CHARTER-v2.0 Section 15.1 states explicitly and without qualification:
"After every 10th reply in a session ... you MUST produce a complete CTP in
Arabic (Markdown)." ADR-080 (proposed in SESSION-ARC-005, never explicitly
ratified by the owner) instead established the practice, followed through
CTP-SESSION-ARC-001.md through CTP-SESSION-ARC-007.md, that persisted CTP
file content stays in English, reserving Arabic for the owner-facing
in-reply summary only. This direct contradiction was flagged as an open item
at the end of SESSION-ARC-006 (CTP-SESSION-ARC-007.md, Section 5 and Section
12 item 2) and carried, still unresolved, through SESSION-ARC-007's own
Reply 10 checkpoint (Section 17.5 item (a)).

## Decision

ADR-080 is superseded. Starting with CTP-SESSION-ARC-008.md (to be authored
at the handoff of SESSION-ARC-007), all persisted CTP file content is
authored in Modern Standard Arabic (فصحى), as the Charter's Section 15.1
literally and unambiguously requires.

Technical identifiers are exempt from translation and remain written in
their original Latin-script form, exactly as the Charter's own INIT-8
principle treats money and codes in UI text: file paths, commit SHAs,
SHA-256 hashes, table/column/function/struct names, IPC command names, task
IDs (e.g. TASK-BRK-061), ADR IDs, and literal command-line output. These are
identifiers, not language content, and translating or transliterating them
would destroy their verifiability.

CTP-SESSION-ARC-001.md through CTP-SESSION-ARC-007.md are NOT retroactively
translated. They remain valid as historical English-language records; this
ADR governs future authoring only.

## Reasons

- Per CHARTER-v2.0 Section 1.2 and Section 14.1 ("Where they contradict this
  charter, this charter wins except for engine invariants they confirm"),
  the Charter is binding and ADR-080 was never ratified by the owner as an
  explicit, documented exception — it was an unratified proposal that the
  Charter's own supremacy clause resolves.
- The architect holds sovereign authority over protocol/process decisions
  (Charter Section 1.2); the language in which internal project-memory
  documents are authored is a process decision, not a business decision
  reserved to the owner, so no further owner approval is required to
  correct this course — though the owner was invited to object in
  SESSION-ARC-007 Reply 13's report and has final taste/business authority
  to overrule this ADR if desired.
- Retroactive translation of seven large, already-accepted historical
  documents would consume disproportionate executor budget for no safety or
  compliance benefit, since their content remains fully readable and
  already serves its evidentiary purpose in English.

## Consequences

- CTP-SESSION-ARC-008.md (and every subsequent session's CTP) is authored
  in Arabic from its first line, including technical sections (executive
  summary, roadmap, technical state, errors log), with the Latin-script
  exemption above.
- This materially changes the CTP-authoring task template used in every
  future TASK-DOC-xxx CTP checkpoint/handoff task: the task must now
  instruct the executor to write Arabic prose for all non-identifier
  content.
- If this proves impractical in later sessions (e.g. mixing Arabic prose
  with large literal code/SQL/diff pastes becomes unreadable), a future ADR
  may re-open this question — but only via an explicit, documented ADR, not
  a silent reversion.

## References

- CHARTER-v2.0, Section 15.1, Section 1.2, Section 14.1, INIT-8.
- CTP-SESSION-ARC-007.md, Section 5, Section 12 item 2, Section 17.5 item
  (a).
