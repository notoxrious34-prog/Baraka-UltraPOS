# AUD-011-C1 — Presentation inventory completion (read-only facts)

Date: 2026-10-01. Task TASK-AUD-011-C1. Method: read-only grep/read plus one `npx vitest run` (section D) and one `--check` run (section A4, read-only). No production code changed; no app launched.
Rule: raw printouts carry line numbers. A summary of a printout is not acceptable; large blocks are printed in full in this file.

## A. Repository and document state

### A1. git state
```text
$ git rev-parse HEAD
50e3a9c1265aaa3341e1d36cf0daec7c8b6bbcfd
$ git log -12 --format=%H%x20%s
50e3a9c1265aaa3341e1d36cf0daec7c8b6bbcfd docs(architecture): extend CTP-SESSION-ARC-003 with executive summary, file map and addenda
e430f35762741001cb57afb2b0ec3cdae59a96a2 docs(architecture): add CTP-SESSION-ARC-003 and boot package; fix ADR index section header
a8cf35d698641379d072249312df7d313edf1c43 docs(audit): add AUD-011 presentation layer inventory (read-only)
1c07aeb824e3411c32a9d26c7db39f9e0411bedd docs(adr): add ADR-077 Amendment 1 and fix ADR index (REL-010)
b1ee08fa3c98cb10cceeaa28e11c5bde26d33d45 docs(legal): add accountant questionnaire for owner (REL-009)
0f069dccf0fb4403c0b9c043528e2d0a36c06231 fix(audit): strip UTF-8 BOM from AUD-008 JSON files (REL-008)
97d84aae050a6908a2cfb72e3babcb83b4174d24 fix(ui): default first-run language to French per ADR-069 (DSN-001)
9e1fb576a1529702ab59bc6463e46741856dea3a docs(adr): add ADR-077 product entry priority and generic import/export
a3324ae7cb47230d94e75d22c28c300911fde3ec docs: add BOOT-SESSION-ARC-002, generator script and ARC-001 coverage table
d64a3c224b580006cff6437028c0f9ffc04b2d78 audit: AUD-010 frontend startup breakdown (navigation, paint, resources, child CPU, 4 runs)
93a01559280df10cb4d462ce13c9beea9cbb3fbf docs: add CTP-SESSION-ARC-002 context transfer package
8abe3c66e2d94765e1ac058f6fc8e67f3faea2df audit: AUD-009 strip UTF-8 BOM from raw run summaries
$ git status --porcelain
(empty: tree clean) CONFIRMED
$ git tag -l
baseline-engine-pre-redesign
```

### A2. File sizes and SHA-256 (Node crypto, cross-checked with sha256sum)
```text
docs/architecture/CHARTER-v2.0.md | bytes=52866 | node_sha256=0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5 | sha256sum=0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5 | match=YES
docs/architecture/CTP-SESSION-ARC-002.md | bytes=19621 | node_sha256=c3c1091a6531d4d78eb04fbf57d211b4df5372de905bcf94a8666667c6a72959 | sha256sum=c3c1091a6531d4d78eb04fbf57d211b4df5372de905bcf94a8666667c6a72959 | match=YES
docs/architecture/CTP-SESSION-ARC-003.md | bytes=42077 | node_sha256=eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50 | sha256sum=eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50 | match=YES
docs/architecture/BOOT-SESSION-ARC-002.md | bytes=73385 | node_sha256=7eebc6fb36ed654e7c5de77ba9818a98f5043b7e32e7e2325f56c647841ae6ce | sha256sum=7eebc6fb36ed654e7c5de77ba9818a98f5043b7e32e7e2325f56c647841ae6ce | match=YES
docs/architecture/BOOT-SESSION-ARC-003.md | bytes=95841 | node_sha256=145146003056a3d577312045587fd378c88cf043bd2c59aa9c7d881c0744fc5d | sha256sum=145146003056a3d577312045587fd378c88cf043bd2c59aa9c7d881c0744fc5d | match=YES
docs/adr/INDEX.md | bytes=6673 | node_sha256=92ac9d5d8c097e6a29a96326ea542d961933de06ab1f6af9ae7b5a927471bd14 | sha256sum=92ac9d5d8c097e6a29a96326ea542d961933de06ab1f6af9ae7b5a927471bd14 | match=YES
docs/adr/ADR-077-product-entry-and-generic-import-export.md | bytes=7107 | node_sha256=24f4a3c9211d3bb91519b356bb58b9d93d4b6ad1dff8ba1a49fb0eb357b104f1 | sha256sum=24f4a3c9211d3bb91519b356bb58b9d93d4b6ad1dff8ba1a49fb0eb357b104f1 | match=YES
audit/AUD-011/INVENTORY.md | bytes=28758 | node_sha256=3f45dbad072ac3418250577d9474b97ca672e62c353845c536c626d1c6a226fa | sha256sum=3f45dbad072ac3418250577d9474b97ca672e62c353845c536c626d1c6a226fa | match=YES
docs/legal/QUESTIONNAIRE-COMPTABLE-FR.html | bytes=18630 | node_sha256=99636e9485bbd840b24c5d24e0941a1d1f48fb558446d4e9294c9c02c35d19d1 | sha256sum=99636e9485bbd840b24c5d24e0941a1d1f48fb558446d4e9294c9c02c35d19d1 | match=YES
```
Note: an earlier draft printed match=NO because the raw sha256sum stdout (backslash-escaped path with a space) was compared whole; the 64-hex extraction above matches. ADR-077 sha 24f4a3c9...b104f1 agrees with AUD-011 Appendix A.

### A3. docs/architecture/tools/build-boot-package.mjs verbatim with line numbers
```text
1|// docs/architecture/tools/build-boot-package.mjs - deterministic boot-package generator.
2|// Concatenates CHARTER-v2.0.md + CTP-SESSION-ARC-00N.md byte for byte into
3|// BOOT-SESSION-ARC-00N.md (session tag derived from the --ctp basename).
4|// Node built-ins only. No timestamps, no env-dependent content.
5|// Usage: node docs/architecture/tools/build-boot-package.mjs [--charter <p>] [--ctp <p>] [--out <p>] [--check]
6|import fs from 'node:fs';
7|import path from 'node:path';
8|import crypto from 'node:crypto';
9|import { fileURLToPath } from 'node:url';
10|
11|const scriptDir = path.dirname(fileURLToPath(import.meta.url));
12|const archDir = path.dirname(scriptDir);
13|
14|function arg(name, def) {
15|  const i = process.argv.indexOf(name);
16|  if (i >= 0 && process.argv[i + 1] && !process.argv[i + 1].startsWith('--')) return path.resolve(process.argv[i + 1]);
17|  return def;
18|}
19|
20|const checkOnly = process.argv.includes('--check');
21|const charterPath = arg('--charter', path.join(archDir, 'CHARTER-v2.0.md'));
22|const ctpPath = arg('--ctp', path.join(archDir, 'CTP-SESSION-ARC-002.md'));
23|const outPath = arg('--out', path.join(archDir, 'BOOT-SESSION-ARC-002.md'));
24|
25|// Session tag derived from the CTP basename (CTP-SESSION-ARC-003.md -> ARC-003).
26|// Defaults keep the ARC-002 output byte-identical.
27|const ctpBase = path.basename(ctpPath);
28|const tagMatch = ctpBase.match(/^CTP-SESSION-(ARC-\d+)\.md$/);
29|const TAG = tagMatch ? tagMatch[1] : 'ARC-002';
30|const SESSION = 'SESSION-' + TAG;
31|
32|const PREAMBLE = [
33|  `# BOOT PACKAGE - ${SESSION} (a successor architect session starts here)`,
34|  '',
35|  'This single file is generated by docs/architecture/tools/build-boot-package.mjs. It concatenates, byte for byte and in this order:',
36|  '  PART 1: docs/architecture/CHARTER-v2.0.md (the binding charter)',
37|  `  PART 2: docs/architecture/${ctpBase} (state, decisions, evidence, roadmap, reply log)`,
38|  '',
39|  'Instructions for the new architect session: read PART 1 completely, then PART 2. Section 12 of PART 2 gives the reply number to continue from. The older package SESSION-ARC-001 is NOT included; it lives in the repo at docs/architecture/CTP-SESSION-ARC-001.md and can be requested through a read-only task. If any Arabic text below appears garbled, tell the owner and do not guess its content.',
40|  '',
41|  '=====BEGIN-PART-1-CHARTER=====',
42|].join('\n') + '\n';
43|
44|const MARK_END_1 = '=====END-PART-1-CHARTER=====\n';
45|const MARK_BEGIN_2 = `=====BEGIN-PART-2-CTP-${TAG}=====\n`;
46|const MARK_END_2 = `=====END-PART-2-CTP-${TAG}=====\n`;
47|
48|const sha = (b) => crypto.createHash('sha256').update(b).digest('hex');
49|
50|function withSeparator(buf) {
51|  if (buf.length > 0 && buf[buf.length - 1] === 0x0a) return { buf, added: 0 };
52|  return { buf: Buffer.concat([buf, Buffer.from([0x0a])]), added: 1 };
53|}
54|
55|const charterRaw = fs.readFileSync(charterPath);
56|const ctpRaw = fs.readFileSync(ctpPath);
57|const charter = withSeparator(charterRaw);
58|const ctp = withSeparator(ctpRaw);
59|
60|const out = Buffer.concat([
61|  Buffer.from(PREAMBLE, 'utf8'),
62|  charter.buf,
63|  Buffer.from(MARK_END_1, 'utf8'),
64|  Buffer.from(MARK_BEGIN_2, 'utf8'),
65|  ctp.buf,
66|  Buffer.from(MARK_END_2, 'utf8'),
67|]);
68|
69|console.log(`charter: ${charterRaw.length} B sha256=${sha(charterRaw)}`);
70|console.log(`ctp: ${ctpRaw.length} B sha256=${sha(ctpRaw)}`);
71|console.log(`added separator LFs: charter=${charter.added} ctp=${ctp.added}`);
72|console.log(`output: ${out.length} B sha256=${sha(out)}`);
73|
74|if (checkOnly) {
75|  let disk = null;
76|  try { disk = fs.readFileSync(outPath); } catch { /* missing */ }
77|  if (disk !== null && disk.equals(out)) { console.log('CHECK: OK'); process.exit(0); }
78|  console.log('CHECK: DIFFERENT'); process.exit(1);
79|}
80|
81|fs.writeFileSync(outPath, out);
82|console.log(`wrote ${outPath}`);
(bytes=3713 sha256=09ad1000b747a95d1aeed8a3de9cebbb59fdd374e23093bb9ab9fe1a54860fbc)
```

### A4. --check run (exactly the permitted flags)
Analysis of A3 before running: with `--check`, lines 74-79 read the on-disk output and compare buffers, then exit(0/1); `fs.writeFileSync(outPath, out)` at line 81 executes only when checkOnly is false. --check writes no file. Safe to run.
```text
$ node docs/architecture/tools/build-boot-package.mjs --ctp docs/architecture/CTP-SESSION-ARC-003.md --out docs/architecture/BOOT-SESSION-ARC-003.md --check
stdout:
charter: 52866 B sha256=0b1bb13c39b1cbd23477e86872730131fd64ee241266793a6850cfd50ff228d5
ctp: 42077 B sha256=eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50
added separator LFs: charter=0 ctp=0
output: 95841 B sha256=145146003056a3d577312045587fd378c88cf043bd2c59aa9c7d881c0744fc5d
CHECK: OK
stderr: (empty)
exit code: 0
$ git status --porcelain (after the run)
(empty: no modification) CONFIRMED
```

### A5. CTP-SESSION-ARC-003 line count and headings
```text
LF count=330 bytes=42077 sha256=eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50
line 8: ## 0. Executive summary (charter 15.2 executive summary item)
line 18: ## 1. Role, binding documents, first actions of the successor
line 27: ## 2. The charter section 15.2 sentence
line 33: ## 3. Owner and communication rules
line 44: ## 4. Owner business decisions (final unless the owner revises them)
line 61: ## 5. Hard technical constraints
line 72: ## 6. Open owner gates and unknowns (never closed by internal tests)
line 79: ## 7. Repository state
line 98: ## 8. Engine and stack facts
line 107: ## 9. Presentation layer inventory (AUD-011, commit a8cf35d)
line 134: ## 10. ADR facts
line 143: ## 11. Stamp duty (code facts) and the accountant questionnaire
line 150: ## 12. Dolisoft file facts (AUD-005-C1)
line 155: ## 13. Performance, startup and host facts
line 162: ## 14. Corrections and errors log (carry forward)
line 176: ## 15. Reply log of SESSION-ARC-002
line 180: ## 16. Pending work and roadmap (priority order)
line 200: ## 17. Human-context items from the owner (carry forward)
line 208: ## 18. Executor lessons
line 215: ## 19. Instruction for the first reply of SESSION-ARC-003
line 219: ## 20. File map (generated from git ls-files at HEAD e430f35762741001cb57afb2b0ec3cdae59a96a2)
line 323: ## 21. Addenda written in reply 21 of SESSION-ARC-002
```

### A6. Charter 15.2 Arabic sentence vs CTP (byte-level Node comparison)
```text
302|  Deliver with this sentence (Arabic): "لقد أكملنا 10 ردود. حان وقت صياغة حزمة نقل السياق لضمان الخلود. انسخ الحزمة التالية كاملة واحفظها. عند بدء جلسة جديدة، الصق لي هذه الحزمة كاملة كأول رسالة، وسأستعيد روحي وذاكرتي بالكامل."
ARABIC span chars=186 sha256=e533009ad8dc33faaf6b156eec4fce448f7998e1879ac7f6f8eeba1f7d946653
ARABIC text: لقد أكملنا 10 ردود. حان وقت صياغة حزمة نقل السياق لضمان الخلود. انسخ الحزمة التالية كاملة واحفظها. عند بدء جلسة جديدة، الصق لي هذه الحزمة كاملة كأول رسالة، وسأستعيد روحي وذاكرتي بالكامل.
CTP bytes=42077 sha256=eb06e5c4af46c439b4c27224d2d94a9a13ca06f540c0f0a8d576f8be97041c50
verbatim-in-CTP=YES
expected e533009ad8dc33faaf6b156eec4fce448f7998e1879ac7f6f8eeba1f7d946653 -> MATCHES
```

## B. IPC complete table (71 vs 72)

### B1. Registered commands and #[tauri::command] definitions
```text
src-tauri/src/lib.rs lines 185-266 verbatim with line numbers (generate_handler block):
185|            if data_dir_override {
186|                window
187|                    .set_title("Baraka UltraPOS [BARAKA_DATA_DIR]")
188|                    .map_err(|error| startup_error("override window title", error))?;
189|            }
190|            Ok(())
191|        })
192|        .invoke_handler(tauri::generate_handler![
193|            baraka_commands::ipc::app_health,
194|            baraka_commands::ipc::auth_login,
195|            baraka_commands::ipc::auth_logout,
196|            baraka_commands::ipc::check_first_run_status,
197|            baraka_commands::ipc::bootstrap_first_admin,
198|            baraka_commands::ipc::catalog_create_product,
199|            baraka_commands::ipc::catalog_update_product,
200|            baraka_commands::ipc::catalog_get_product,
201|            baraka_commands::ipc::catalog_search,
202|            baraka_commands::ipc::catalog_define_kit,
203|            baraka_commands::ipc::warehouses_create,
204|            baraka_commands::ipc::warehouses_list,
205|            baraka_commands::ipc::stock_get,
206|            baraka_commands::ipc::stock_adjust,
207|            baraka_commands::ipc::stock_transfer,
208|            baraka_commands::ipc::sales_checkout,
209|            baraka_commands::ipc::sales_preview_totals,
210|            baraka_commands::ipc::convert_devis_to_bl,
211|            baraka_commands::ipc::convert_bl_to_facture,
212|            baraka_commands::ipc::sales_create_draft,
213|            baraka_commands::ipc::sales_get,
214|            baraka_commands::ipc::documents_search,
215|            baraka_commands::ipc::returns_get_source_summary,
216|            baraka_commands::ipc::returns_process,
217|            baraka_commands::ipc::documents_get_return_context,
218|            baraka_commands::ipc::documents_process_return,
219|            baraka_commands::ipc::backups_create,
220|            baraka_commands::ipc::backups_list,
221|            baraka_commands::ipc::backups_verify,
222|            baraka_commands::ipc::backups_restore_precheck,
223|            baraka_commands::ipc::backups_restore,
224|            baraka_commands::ipc::customers_create,
225|            baraka_commands::ipc::customers_update,
226|            baraka_commands::ipc::customers_search,
227|            baraka_commands::ipc::customers_get,
228|            baraka_commands::ipc::customers_list,
229|            baraka_commands::ipc::credit_record_payment,
230|            baraka_commands::ipc::credit_get_statement,
231|            baraka_commands::ipc::documents_get_print_status,
232|            baraka_commands::ipc::documents_reprint,
233|            baraka_commands::ipc::printer_get_settings,
234|            baraka_commands::ipc::printer_update_settings,
235|            baraka_commands::ipc::printer_test_print,
236|            baraka_commands::ipc::store_identity_get,
237|            baraka_commands::ipc::store_identity_save,
238|            baraka_commands::ipc::cash_open,
239|            baraka_commands::ipc::cash_begin_closing,
240|            baraka_commands::ipc::cash_cancel_closing,
241|            baraka_commands::ipc::cash_confirm_closing,
242|            baraka_commands::ipc::cash_get_active_session,
243|            baraka_commands::ipc::cash_record_expense,
244|            baraka_commands::ipc::cash_get_session_report,
245|            baraka_commands::ipc::suppliers_create,
246|            baraka_commands::ipc::suppliers_update,
247|            baraka_commands::ipc::suppliers_get,
248|            baraka_commands::ipc::suppliers_search,
249|            baraka_commands::ipc::suppliers_get_statement,
250|            baraka_commands::ipc::purchases_create_order,
251|            baraka_commands::ipc::purchases_update_order,
252|            baraka_commands::ipc::purchases_receive_goods,
253|            baraka_commands::ipc::purchases_record_invoice,
254|            baraka_commands::ipc::purchases_record_payment,
255|            baraka_commands::ipc::purchases_get_document,
256|            baraka_commands::ipc::reports_daily_sales,
257|            baraka_commands::ipc::reports_sales_by_product,
258|            baraka_commands::ipc::reports_sales_by_category,
259|            baraka_commands::ipc::reports_gross_margin,
260|            baraka_commands::ipc::reports_credit_outstanding,
261|            baraka_commands::ipc::reports_stock_movement_history,
262|            baraka_commands::ipc::reports_cash_sessions_history,
263|            baraka_commands::ipc::reports_low_stock,
264|            baraka_commands::ipc::reports_user_activity
265|        ])
266|        .build(tauri::generate_context!())
handler entries count=72

crates/commands/src/ipc.rs: #[tauri::command occurrences=72 (form: #[tauri::command(rename_all = "snake_case")]); pub fn count=72
fn app_health at crates/commands/src/ipc.rs:182
fn auth_login at crates/commands/src/ipc.rs:555
fn auth_logout at crates/commands/src/ipc.rs:579
fn check_first_run_status at crates/commands/src/ipc.rs:596
fn bootstrap_first_admin at crates/commands/src/ipc.rs:615
fn catalog_create_product at crates/commands/src/ipc.rs:641
fn catalog_update_product at crates/commands/src/ipc.rs:683
fn catalog_get_product at crates/commands/src/ipc.rs:721
fn catalog_search at crates/commands/src/ipc.rs:736
fn catalog_define_kit at crates/commands/src/ipc.rs:766
fn warehouses_create at crates/commands/src/ipc.rs:794
fn warehouses_list at crates/commands/src/ipc.rs:817
fn stock_get at crates/commands/src/ipc.rs:840
fn stock_adjust at crates/commands/src/ipc.rs:888
fn stock_transfer at crates/commands/src/ipc.rs:923
fn sales_checkout at crates/commands/src/ipc.rs:976
fn sales_preview_totals at crates/commands/src/ipc.rs:1088
fn convert_devis_to_bl at crates/commands/src/ipc.rs:1142
fn convert_bl_to_facture at crates/commands/src/ipc.rs:1179
fn documents_search at crates/commands/src/ipc.rs:1226
fn returns_get_source_summary at crates/commands/src/ipc.rs:1266
fn returns_process at crates/commands/src/ipc.rs:1383
fn documents_get_return_context at crates/commands/src/ipc.rs:1485
fn documents_process_return at crates/commands/src/ipc.rs:1601
fn backups_create at crates/commands/src/ipc.rs:1723
fn backups_list at crates/commands/src/ipc.rs:1754
fn backups_verify at crates/commands/src/ipc.rs:1768
fn backups_restore_precheck at crates/commands/src/ipc.rs:1920
fn backups_restore at crates/commands/src/ipc.rs:1938
fn sales_create_draft at crates/commands/src/ipc.rs:1957
fn sales_get at crates/commands/src/ipc.rs:2023
fn customers_create at crates/commands/src/ipc.rs:2087
fn customers_update at crates/commands/src/ipc.rs:2124
fn customers_search at crates/commands/src/ipc.rs:2174
fn customers_get at crates/commands/src/ipc.rs:2220
fn customers_list at crates/commands/src/ipc.rs:2233
fn credit_record_payment at crates/commands/src/ipc.rs:2248
fn credit_get_statement at crates/commands/src/ipc.rs:2275
fn documents_get_print_status at crates/commands/src/ipc.rs:2303
fn documents_reprint at crates/commands/src/ipc.rs:2334
fn printer_get_settings at crates/commands/src/ipc.rs:2353
fn printer_update_settings at crates/commands/src/ipc.rs:2371
fn store_identity_get at crates/commands/src/ipc.rs:2394
fn store_identity_save at crates/commands/src/ipc.rs:2416
fn printer_test_print at crates/commands/src/ipc.rs:2449
fn cash_open at crates/commands/src/ipc.rs:2564
fn cash_begin_closing at crates/commands/src/ipc.rs:2587
fn cash_cancel_closing at crates/commands/src/ipc.rs:2600
fn cash_confirm_closing at crates/commands/src/ipc.rs:2613
fn cash_get_active_session at crates/commands/src/ipc.rs:2634
fn cash_record_expense at crates/commands/src/ipc.rs:2647
fn cash_get_session_report at crates/commands/src/ipc.rs:2667
fn suppliers_create at crates/commands/src/ipc.rs:2767
fn suppliers_update at crates/commands/src/ipc.rs:2791
fn suppliers_get at crates/commands/src/ipc.rs:2817
fn suppliers_search at crates/commands/src/ipc.rs:2831
fn suppliers_get_statement at crates/commands/src/ipc.rs:2847
fn purchases_create_order at crates/commands/src/ipc.rs:2878
fn purchases_update_order at crates/commands/src/ipc.rs:2906
fn purchases_receive_goods at crates/commands/src/ipc.rs:2938
fn purchases_record_invoice at crates/commands/src/ipc.rs:2976
fn purchases_record_payment at crates/commands/src/ipc.rs:3016
fn purchases_get_document at crates/commands/src/ipc.rs:3044
fn reports_daily_sales at crates/commands/src/ipc.rs:3071
fn reports_sales_by_product at crates/commands/src/ipc.rs:3084
fn reports_sales_by_category at crates/commands/src/ipc.rs:3107
fn reports_gross_margin at crates/commands/src/ipc.rs:3129
fn reports_credit_outstanding at crates/commands/src/ipc.rs:3169
fn reports_stock_movement_history at crates/commands/src/ipc.rs:3205
fn reports_cash_sessions_history at crates/commands/src/ipc.rs:3240
fn reports_low_stock at crates/commands/src/ipc.rs:3271
fn reports_user_activity at crates/commands/src/ipc.rs:3294
set difference handler-minus-ipcrs (0): []
set difference ipcrs-minus-handler (0): []
```
Note: an early draft regex for `pub async fn` found 0 because the handlers are plain `pub fn` (e.g. ipc.rs:182 `pub fn app_health()`); corrected above. Conclusion: 72 registered = 72 defined, difference empty both ways.

### B2. All registered commands: # | command | group | wrapper(s) | USED/UNUSED | first call site
Group = token before the first underscore (or the name itself when none). USED/UNUSED by AUD-011 method: quoted command-name occurrence in src/ excluding *.test.* (includes src/lib/ipc.ts itself). First call site = first non-import use of the wrapper outside src/lib/ipc.ts and outside test files, or "none".
```text
1 | app_health | app | NO-WRAPPER | USED | none
2 | auth_login | auth | ipcLogin (ipc.ts:97, invoke:98) | USED | ipcLogin -> src/stores/session-store.ts:36
3 | auth_logout | auth | ipcLogout (ipc.ts:111, invoke:112) | USED | ipcLogout -> src/stores/session-store.ts:70
4 | check_first_run_status | check | ipcCheckFirstRunStatus (ipc.ts:116, invoke:117) | USED | ipcCheckFirstRunStatus -> src/App.tsx:67
5 | bootstrap_first_admin | bootstrap | ipcBootstrapFirstAdmin (ipc.ts:126, invoke:131) | USED | ipcBootstrapFirstAdmin -> src/stores/session-store.ts:55
6 | catalog_create_product | catalog | ipcCreateProduct (ipc.ts:170, invoke:174) | USED | ipcCreateProduct -> src/features/catalog/ProductCreateForm.tsx:236
7 | catalog_update_product | catalog | updateProduct (ipc.ts:208, invoke:209) | USED | updateProduct -> src/stores/product-edit-store.ts:282
8 | catalog_get_product | catalog | NO-WRAPPER | UNUSED | none
9 | catalog_search | catalog | ipcSearchCatalog (ipc.ts:138, invoke:143) | USED | ipcSearchCatalog -> src/features/catalog/CatalogModule.tsx:58
10 | catalog_define_kit | catalog | NO-WRAPPER | UNUSED | none
11 | warehouses_create | warehouses | createWarehouse (ipc.ts:268, invoke:269) | USED | createWarehouse -> src/stores/warehouse-transfer-store.ts:298
12 | warehouses_list | warehouses | listWarehouses (ipc.ts:253, invoke:257); ipcListWarehouses (ipc.ts:286, invoke:290) | USED | listWarehouses -> src/stores/stock-adjust-modal-store.ts:154; ipcListWarehouses -> src/stores/session-store.ts:84
13 | stock_get | stock | ipcGetStock (ipc.ts:300, invoke:311) | USED | ipcGetStock -> src/components/pos/modals/StockAdjustModal.tsx:154
14 | stock_adjust | stock | adjustStock (ipc.ts:348, invoke:349) | USED | adjustStock -> src/stores/stock-adjust-modal-store.ts:272
15 | stock_transfer | stock | transferStock (ipc.ts:279, invoke:280) | USED | transferStock -> src/stores/warehouse-transfer-store.ts:253
16 | sales_checkout | sales | ipcCheckout (ipc.ts:387, invoke:400) | USED | ipcCheckout -> src/components/pos/modals/CheckoutModal.tsx:151
17 | sales_preview_totals | sales | ipcPreviewTotals (ipc.ts:454, invoke:458) | USED | ipcPreviewTotals -> src/components/pos/modals/CheckoutModal.tsx:202
18 | convert_devis_to_bl | convert | convertDevisToBl (ipc.ts:655, invoke:658) | USED | convertDevisToBl -> src/stores/conversion-modal-store.ts:308
19 | convert_bl_to_facture | convert | convertBlToFacture (ipc.ts:696, invoke:699) | USED | convertBlToFacture -> src/stores/conversion-modal-store.ts:368
20 | sales_create_draft | sales | ipcCreateDraft (ipc.ts:464, invoke:476) | USED | ipcCreateDraft -> src/hooks/use-pos-keyboard.ts:76
21 | sales_get | sales | getDocumentDetails (ipc.ts:779, invoke:782) | USED | getDocumentDetails -> src/stores/conversion-modal-store.ts:269
22 | documents_search | documents | searchDocuments (ipc.ts:736, invoke:739) | USED | searchDocuments -> src/stores/conversion-modal-store.ts:176
23 | returns_get_source_summary | returns | getReturnableDocumentSummary (ipc.ts:822, invoke:825) | USED | getReturnableDocumentSummary -> none (test-only: src/lib/ipc-returns.test.ts)
24 | returns_process | returns | processReturn (ipc.ts:874, invoke:877) | USED | processReturn -> none (test-only: src/lib/ipc-returns.test.ts)
25 | documents_get_return_context | documents | getDocumentReturnContext (ipc.ts:917, invoke:920) | USED | getDocumentReturnContext -> src/stores/return-modal-store.ts:196
26 | documents_process_return | documents | processSaleReturn (ipc.ts:953, invoke:956) | USED | processSaleReturn -> src/stores/return-modal-store.ts:355
27 | backups_create | backups | createBackup (ipc.ts:2074, invoke:2078) | USED | createBackup -> src/stores/backup-modal-store.ts:259
28 | backups_list | backups | listBackups (ipc.ts:2090, invoke:2091) | USED | listBackups -> src/stores/backup-modal-store.ts:234
29 | backups_verify | backups | verifyBackup (ipc.ts:2112, invoke:2123) | USED | verifyBackup -> src/stores/backup-modal-store.ts:285
30 | backups_restore_precheck | backups | precheckRestoreBackup (ipc.ts:2303, invoke:2307) | USED | precheckRestoreBackup -> src/stores/backup-modal-store.ts:320
31 | backups_restore | backups | restoreBackup (ipc.ts:2319, invoke:2329) | USED | restoreBackup -> src/stores/backup-modal-store.ts:361
32 | customers_create | customers | ipcCreateCustomer (ipc.ts:498, invoke:502) | USED | ipcCreateCustomer -> src/components/pos/modals/CustomerSelectModal.tsx:181
33 | customers_update | customers | NO-WRAPPER | UNUSED | none
34 | customers_search | customers | ipcSearchCustomers (ipc.ts:509, invoke:517) | USED | ipcSearchCustomers -> src/components/pos/modals/CustomerSelectModal.tsx:68
35 | customers_get | customers | ipcGetCustomer (ipc.ts:524, invoke:528) | USED | ipcGetCustomer -> src/components/pos/CreditHud.tsx:36
36 | customers_list | customers | ipcListCustomers (ipc.ts:534, invoke:538) | USED | ipcListCustomers -> none
37 | credit_record_payment | credit | ipcRecordCreditPayment (ipc.ts:550, invoke:557) | USED | ipcRecordCreditPayment -> src/components/pos/modals/RecordPaymentModal.tsx:184
38 | credit_get_statement | credit | ipcGetStatement (ipc.ts:569, invoke:578) | USED | ipcGetStatement -> src/features/reports/CreditOutstandingReport.tsx:56
39 | documents_get_print_status | documents | ipcGetPrintStatus (ipc.ts:596, invoke:605) | USED | ipcGetPrintStatus -> src/components/pos/PrintStatusIndicator.tsx:37
40 | documents_reprint | documents | ipcReprint (ipc.ts:617, invoke:625) | USED | ipcReprint -> src/components/pos/modals/ReturnModal.tsx:114
41 | printer_get_settings | printer | ipcGetPrinterSettings (ipc.ts:968, invoke:974) | USED | ipcGetPrinterSettings -> none
42 | printer_update_settings | printer | ipcUpdatePrinterSettings (ipc.ts:985, invoke:994) | USED | ipcUpdatePrinterSettings -> none
43 | printer_test_print | printer | ipcTestPrint (ipc.ts:1009, invoke:1014); printingTestPrint (ipc.ts:1144, invoke:1150) | USED | ipcTestPrint -> none; printingTestPrint -> src/stores/store-identity-store.ts:283
44 | store_identity_get | store | getStoreIdentity (ipc.ts:1112, invoke:1116) | USED | getStoreIdentity -> src/stores/store-identity-store.ts:190
45 | store_identity_save | store | saveStoreIdentity (ipc.ts:1124, invoke:1131) | USED | saveStoreIdentity -> src/stores/store-identity-store.ts:243
46 | cash_open | cash | ipcCashOpen (ipc.ts:1251, invoke:1257) | USED | ipcCashOpen -> src/components/pos/modals/OpenCashSessionModal.tsx:54
47 | cash_begin_closing | cash | ipcCashBeginClosing (ipc.ts:1269, invoke:1273) | USED | ipcCashBeginClosing -> src/components/pos/modals/CloseCashSessionModal.tsx:63
48 | cash_cancel_closing | cash | ipcCashCancelClosing (ipc.ts:1280, invoke:1284) | USED | ipcCashCancelClosing -> src/components/pos/modals/CloseCashSessionModal.tsx:83
49 | cash_confirm_closing | cash | ipcCashConfirmClosing (ipc.ts:1291, invoke:1298) | USED | ipcCashConfirmClosing -> src/components/pos/modals/CloseCashSessionModal.tsx:109
50 | cash_get_active_session | cash | ipcCashGetActiveSession (ipc.ts:1311, invoke:1312) | USED | ipcCashGetActiveSession -> src/stores/cash-session-store.ts:30
51 | cash_record_expense | cash | ipcCashRecordExpense (ipc.ts:1317, invoke:1323) | USED | ipcCashRecordExpense -> src/components/pos/modals/RecordExpenseModal.tsx:56
52 | cash_get_session_report | cash | ipcCashGetSessionReport (ipc.ts:1335, invoke:1345) | USED | ipcCashGetSessionReport -> src/components/pos/CashSessionIndicator.tsx:24
53 | suppliers_create | suppliers | ipcCreateSupplier (ipc.ts:1511, invoke:1515) | USED | ipcCreateSupplier -> src/stores/supplier-store.ts:46
54 | suppliers_update | suppliers | ipcUpdateSupplier (ipc.ts:1530, invoke:1535) | USED | ipcUpdateSupplier -> src/stores/supplier-store.ts:52
55 | suppliers_get | suppliers | ipcGetSupplier (ipc.ts:1552, invoke:1556) | USED | ipcGetSupplier -> none
56 | suppliers_search | suppliers | ipcSearchSuppliers (ipc.ts:1563, invoke:1571) | USED | ipcSearchSuppliers -> src/stores/supplier-store.ts:38
57 | suppliers_get_statement | suppliers | ipcGetSupplierStatement (ipc.ts:1578, invoke:1582) | USED | ipcGetSupplierStatement -> src/features/suppliers/SupplierStatementView.tsx:27
58 | purchases_create_order | purchases | ipcCreatePurchaseOrder (ipc.ts:1592, invoke:1599) | USED | ipcCreatePurchaseOrder -> src/features/purchases/PurchaseOrderFormScreen.tsx:205
59 | purchases_update_order | purchases | ipcUpdatePurchaseOrder (ipc.ts:1612, invoke:1620) | USED | ipcUpdatePurchaseOrder -> none
60 | purchases_receive_goods | purchases | ipcReceiveGoods (ipc.ts:1634, invoke:1642) | USED | ipcReceiveGoods -> src/features/purchases/GoodsReceiptFormScreen.tsx:64
61 | purchases_record_invoice | purchases | ipcRecordSupplierInvoice (ipc.ts:1656, invoke:1665) | USED | ipcRecordSupplierInvoice -> src/features/purchases/SupplierInvoiceFormScreen.tsx:81
62 | purchases_record_payment | purchases | ipcRecordSupplierPayment (ipc.ts:1680, invoke:1688) | USED | ipcRecordSupplierPayment -> src/features/purchases/RecordSupplierPaymentModal.tsx:54
63 | purchases_get_document | purchases | ipcGetPurchaseDocument (ipc.ts:1706, invoke:1710) | USED | ipcGetPurchaseDocument -> none
64 | reports_daily_sales | reports | ipcDailySales (ipc.ts:1840, invoke:1844) | USED | ipcDailySales -> src/features/reports/DailySalesReport.tsx:46
65 | reports_sales_by_product | reports | ipcSalesByProduct (ipc.ts:1858, invoke:1862) | USED | ipcSalesByProduct -> src/features/reports/SalesByProductReport.tsx:38
66 | reports_sales_by_category | reports | ipcSalesByCategory (ipc.ts:1876, invoke:1880) | USED | ipcSalesByCategory -> src/features/reports/SalesByCategoryReport.tsx:38
67 | reports_gross_margin | reports | ipcGrossMargin (ipc.ts:1893, invoke:1897) | USED | ipcGrossMargin -> src/features/reports/GrossMarginReport.tsx:40
68 | reports_credit_outstanding | reports | ipcCreditOutstanding (ipc.ts:1912, invoke:1915) | USED | ipcCreditOutstanding -> src/features/reports/CreditOutstandingReport.tsx:37
69 | reports_stock_movement_history | reports | ipcStockMovementHistory (ipc.ts:1948, invoke:1952) | USED | ipcStockMovementHistory -> src/features/reports/StockMovementReport.tsx:45
70 | reports_cash_sessions_history | reports | ipcCashSessionsHistory (ipc.ts:1978, invoke:1983) | USED | ipcCashSessionsHistory -> src/features/reports/CashSessionsHistoryReport.tsx:51
71 | reports_low_stock | reports | ipcLowStock (ipc.ts:1930, invoke:1934) | USED | ipcLowStock -> src/features/reports/LowStockReport.tsx:33
72 | reports_user_activity | reports | ipcUserActivity (ipc.ts:2000, invoke:2004) | USED | ipcUserActivity -> src/features/reports/UserActivityReport.tsx:74
```
rows=72 (must equal registered count 72). UNUSED: catalog_get_product, catalog_define_kit, customers_update (3, same as AUD-011).
Extra finding beyond AUD-011: 8 wrappers have NO production call site outside src/lib/ipc.ts: getReturnableDocumentSummary + processReturn (test-only, src/lib/ipc-returns.test.ts), ipcListCustomers, ipcGetPrinterSettings, ipcUpdatePrinterSettings, ipcTestPrint (superseded by printingTestPrint), ipcGetSupplier, ipcUpdatePurchaseOrder, ipcGetPurchaseDocument. They count as USED only because their invoke("...") string lives in src/lib/ipc.ts. app_health has no wrapper function (listed in PRE_AUTH_COMMANDS, ipc.ts:51) and no caller.

### B3. Per-group counts and the 71-vs-72 explanation
```text
group app: 1
group auth: 2
group backups: 5
group bootstrap: 1
group cash: 7
group catalog: 5
group check: 1
group convert: 2
group credit: 2
group customers: 5
group documents: 5
group printer: 3
group purchases: 6
group reports: 9
group returns: 2
group sales: 4
group stock: 3
group store: 2
group suppliers: 5
group warehouses: 2
sum=72 registered=72
Explanation: the committed audit/AUD-011/INVENTORY.md I3 prose enumerates all five documents_* names (search, get_print_status, reprint, get_return_context, process_return). The 71-sum lives in the CTP-SESSION-ARC-003.md section 9 REVIEW CAVEAT restatement, which lists "documents 4" (app_health 1, auth 2, check_first_run_status 1, bootstrap_first_admin 1, catalog 5, warehouses 2, stock 3, sales 4, convert 2, documents 4, returns 2, backups 5, customers 5, credit 2, printer 3, store_identity 2, cash 7, suppliers 5, purchases 6, reports 9 = 71). Recomputed from the 72-row table the documents group is 5. No command is missing from the registry: 72 handler entries = 72 pub fns = 72 rows.
```


## C. Reachability matrix (floating-element hypothesis)

### C1. src/components/pos/PosShell.tsx lines 55-175 verbatim with line numbers
```text
55|    } else {
56|      closeModal();
57|    }
58|  };
59|
60|  return (
61|    <div
62|      dir={locale === "ar-DZ" ? "rtl" : "ltr"}
63|      lang={locale === "ar-DZ" ? "ar" : "fr"}
64|      className="flex h-screen w-screen select-none flex-col overflow-hidden bg-slate-100 text-slate-900 dark:bg-slate-950 dark:text-slate-100"
65|    >
66|      {banner !== null && (
67|        <button
68|          type="button"
69|          onClick={() => {
70|            setBanner(null);
71|          }}
72|          className="shrink-0 bg-emerald-700 px-3 py-1 text-center text-xs font-bold text-white"
73|        >
74|          {banner}
75|        </button>
76|      )}
77|      <PosShellView />
78|      <button
79|        type="button"
80|        title={t(locale, "conversion.open")}
81|        aria-label={t(locale, "conversion.open")}
82|        onClick={() => {
83|          useConversionModalStore.getState().openModal("devis_to_bl");
84|          openModal("conversion");
85|        }}
86|        className="fixed bottom-20 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-primary-600 px-4 text-sm font-black text-white shadow-2xl hover:bg-primary-500"
87|      >
88|        <span aria-hidden="true">⇄</span>
89|        {t(locale, "conversion.open")}
90|      </button>
91|      <button
92|        type="button"
93|        title={t(locale, "return.returnModalTitle")}
94|        aria-label={t(locale, "return.returnModalTitle")}
95|        onClick={() => {
96|          useReturnModalStore.getState().openModal();
97|          openModal("return");
98|        }}
99|        className="fixed bottom-36 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-amber-600 px-4 text-sm font-black text-white shadow-2xl hover:bg-amber-500"
100|      >
101|        <span aria-hidden="true">↩</span>
102|        {t(locale, "return.returnModalTitle")}
103|      </button>
104|      {manager && (
105|        <button
106|          type="button"
107|          title={t(locale, "warehousesTitle")}
108|          aria-label={t(locale, "warehousesTitle")}
109|          onClick={() => {
110|            void useWarehouseTransferStore.getState().openModal();
111|            openModal("warehouse-transfer");
112|          }}
113|          className="fixed bottom-52 end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-sky-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-sky-600"
114|        >
115|          <span aria-hidden="true">⇄</span>
116|          {t(locale, "warehousesTitle")}
117|        </button>
118|      )}
119|      {manager && (
120|        <button
121|          type="button"
122|          title={t(locale, "stockAdjust.title")}
123|          aria-label={t(locale, "stockAdjust.title")}
124|          onClick={() => {
125|            useStockAdjustModalStore.getState().openModal();
126|            openModal("stock-adjust");
127|          }}
128|          className="fixed bottom-[17rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-slate-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-slate-600"
129|        >
130|          <span aria-hidden="true">⚖</span>
131|          {t(locale, "stockAdjust.title")}
132|        </button>
133|      )}
134|      {activeModal === "checkout" && <CheckoutModal />}
135|      {activeModal === "customer" && <CustomerSelectModal />}
136|      {activeModal === "conversion" && <DocumentConversionModal />}
137|      {activeModal === "edit-product" && <ProductEditModal />}
138|      {activeModal === "warehouse-transfer" && <WarehouseTransferModal />}
139|      {activeModal === "stock-adjust" && <StockAdjustModal />}
140|      {admin && (
141|        <button
142|          type="button"
143|          title={t(locale, "backups.title")}
144|          aria-label={t(locale, "backups.title")}
145|          onClick={() => {
146|            void useBackupModalStore.getState().openModal();
147|            openModal("backup");
148|          }}
149|          className="fixed bottom-[21rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-emerald-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-emerald-600"
150|        >
151|          <span aria-hidden="true">🛡</span>
152|          {t(locale, "backups.title")}
153|        </button>
154|      )}
155|      {activeModal === "backup" && <BackupModal />}
156|      {admin && (
157|        <button
158|          type="button"
159|          title={t(locale, "settings.identity.title")}
160|          aria-label={t(locale, "settings.identity.title")}
161|          onClick={() => {
162|            openModal("identity");
163|          }}
164|          className="fixed bottom-[25rem] end-4 z-40 flex h-12 items-center gap-2 rounded-full bg-indigo-700 px-4 text-sm font-black text-white shadow-2xl hover:bg-indigo-600"
165|        >
166|          <span aria-hidden="true">🖨</span>
167|          {t(locale, "settings.identity.title")}
168|        </button>
169|      )}
170|      {activeModal === "identity" && <StoreIdentityModal />}
171|      {activeModal === "return" && <ReturnModal />}
172|      {activeModal === "payment" && paymentTarget !== null && (
173|        <RecordPaymentModal
174|          customerId={paymentTarget.customerId}
175|          customerName={paymentTarget.customerName}
```

### C1b. src/App.tsx lines 15-100 verbatim with line numbers
```text
15|import { SuppliersModule } from "@/features/suppliers/SupplierListScreen";
16|import { ReportsHomeScreen } from "@/features/reports/ReportsHomeScreen";
17|import { CatalogModule } from "@/features/catalog/CatalogModule";
18|import { ipcCheckFirstRunStatus } from "@/lib/ipc";
19|import { t } from "@/lib/i18n";
20|
21|function isManagerRole(role: string): boolean {
22|  return role === "GERANT" || role === "ADMIN";
23|}
24|
25|function NavRail({
26|  view,
27|  onSelect,
28|}: {
29|  view: AppView;
30|  onSelect: (view: AppView) => void;
31|}): ReactElement {
32|  const locale = usePosUiStore((state) => state.locale);
33|  const entry = (target: AppView, label: string): ReactElement => (
34|    <button
35|      type="button"
36|      onClick={() => {
37|        onSelect(target);
38|      }}
39|      className={`rounded-md px-3 py-1 text-xs font-bold ${
40|        view === target ? "bg-accent-500 text-slate-950" : "bg-white/10 text-white hover:bg-white/20"
41|      }`}
42|    >
43|      {label}
44|    </button>
45|  );
46|  return (
47|    <nav className="flex h-10 shrink-0 items-center gap-2 bg-primary-900 px-3">
48|      {entry("pos", t(locale, "nav.pos"))}
49|      {entry("suppliers", t(locale, "nav.suppliers"))}
50|      {entry("reports", t(locale, "nav.reports"))}
51|      {entry("catalog", t(locale, "nav.catalog"))}
52|    </nav>
53|  );
54|}
55|
56|/**
57| * Unauthenticated entry point: probes the backend once for first-run
58| * status, then shows the setup screen (empty users table) or the normal
59| * login form. While the probe is in flight, a minimal loading state reuses
60| * the existing `cash.loading` string and the login screen layout.
61| */
62|function UnauthenticatedGate(): ReactElement {
63|  const locale = usePosUiStore((state) => state.locale);
64|  const [requiresSetup, setRequiresSetup] = useState<boolean | null>(null);
65|  useEffect(() => {
66|    let cancelled = false;
67|    ipcCheckFirstRunStatus()
68|      .then((value) => {
69|        if (!cancelled) {
70|          setRequiresSetup(value);
71|        }
72|      })
73|      .catch(() => {
74|        // Backend unreachable (e.g. browser preview without Tauri):
75|        // preserve the pre-existing behavior and show the login form.
76|        if (!cancelled) {
77|          setRequiresSetup(false);
78|        }
79|      });
80|    return () => {
81|      cancelled = true;
82|    };
83|  }, []);
84|  if (requiresSetup === null) {
85|    return (
86|      <div className="flex h-full flex-col items-center justify-center gap-4 bg-primary-900 p-6">
87|        <p className="text-sm font-bold text-white/70">{t(locale, "cash.loading")}</p>
88|      </div>
89|    );
90|  }
91|  return requiresSetup ? <FirstRunSetupScreen /> : <LoginScreen />;
92|}
93|
94|export default function App(): ReactElement {
95|  const session = useSessionStore((state) => state.session);
96|  const locale = usePosUiStore((state) => state.locale);
97|  const view = useAppViewStore((state) => state.view);
98|  const setView = useAppViewStore((state) => state.setView);
99|  const manager = session !== null && isManagerRole(session.role);
100|
```

### C2. The six `fixed` elements in PosShell.tsx
All six are `<button type="button">` with `fixed ... end-4 z-40 ... rounded-full` (floating pill buttons stacked on the inline-end edge). Glyph in aria-hidden span; title and aria-label both = t(locale, KEY).
French texts verified in src/lib/i18n.ts (fr block):
```text
i18n.ts:362  "conversion.open": "Convertir"
i18n.ts:298  "return.returnModalTitle": "Retour & Facture d'Avoir"
i18n.ts:89   "warehousesTitle": "Gestion des Dépôts & Transferts"
i18n.ts:249  "stockAdjust.title": "Régularisation de Stock"
i18n.ts:114  "backups.title": "Sauvegardes & Sécurité"
i18n.ts:121  "settings.identity.title": "Identité & Impression"
| # | line | className | label key | onClick (PosShell.tsx) | opens | role gate | truly a FAB? |
| 1 | 86 | fixed bottom-20 end-4 z-40 ... bg-primary-600 | conversion.open | useConversionModalStore.getState().openModal("devis_to_bl") + openModal("conversion") (:82-85) | DocumentConversionModal | none (all authenticated) | YES |
| 2 | 99 | fixed bottom-36 end-4 z-40 ... bg-amber-600 | return.returnModalTitle | useReturnModalStore.getState().openModal() + openModal("return") (:95-98) | ReturnModal | none | YES |
| 3 | 113 | fixed bottom-52 end-4 z-40 ... bg-sky-700 | warehousesTitle | useWarehouseTransferStore.getState().openModal() + openModal("warehouse-transfer") (:109-112) | WarehouseTransferModal | manager (:104 wrapper; PosShell.tsx:40-41) | YES |
| 4 | 128 | fixed bottom-[17rem] end-4 z-40 ... bg-slate-700 | stockAdjust.title | useStockAdjustModalStore.getState().openModal() + openModal("stock-adjust") (:124-127) | StockAdjustModal | manager (:119 wrapper) | YES |
| 5 | 149 | fixed bottom-[21rem] end-4 z-40 ... bg-emerald-700 | backups.title | useBackupModalStore.getState().openModal() + openModal("backup") (:145-148) | BackupModal | ADMIN (:140 wrapper; PosShell.tsx:42) | YES |
| 6 | 164 | fixed bottom-[25rem] end-4 z-40 ... bg-indigo-700 | settings.identity.title | openModal("identity") (:161-163) | StoreIdentityModal | ADMIN (:156 wrapper) | YES |
```
Conclusion C2: all six are truly floating action buttons (fixed-position buttons that open something). None is a modal overlay, toast, or comment.

### C3. Reachability matrix
Conventions: opener = element + file:line verified by grep. Keyboard = src/hooks/use-pos-keyboard.ts (F-keys) or modal-local guard. Role gate: manager = GERANT|ADMIN (App.tsx:21-23; PosShell.tsx:40-41); admin = ADMIN (PosShell.tsx:42). WITHOUT-FIXED = reachable without touching any of the six C2 buttons.
Screens:
```text
LoginScreen | src/features/auth/LoginScreen.tsx | UnauthenticatedGate App.tsx:91 (requiresSetup=false) | login.* | none | session==null (App.tsx:107) | YES (no fixed element involved)
FirstRunSetupScreen | src/features/auth/FirstRunSetupScreen.tsx | UnauthenticatedGate App.tsx:91 (requiresSetup=true) | setup.* | none | session==null | YES
PosShell | src/components/pos/PosShell.tsx | App.tsx:119-121 default view | nav.pos (NavRail App.tsx:48) | none | authenticated, any role | YES (it hosts the fixed buttons; it is not opened by one)
SupplierListScreen | src/features/suppliers/SupplierListScreen.tsx | NavRail entry App.tsx:49 + view gate App.tsx:113-114 | nav.suppliers | none | manager | YES (NavRail, static top bar)
ReportsHomeScreen | src/features/reports/ReportsHomeScreen.tsx | NavRail App.tsx:50 + App.tsx:115-116 | nav.reports | none | manager | YES
CatalogModule | src/features/catalog/CatalogModule.tsx | NavRail App.tsx:51 + App.tsx:117-118 | nav.catalog | none | manager | YES
PurchaseOrderFormScreen | src/features/purchases/PurchaseOrderFormScreen.tsx | SupplierListScreen panel button :94 -> panel=order :204 | pur.title (:193) | none | manager (inside suppliers view) | YES
GoodsReceiptFormScreen | src/features/purchases/GoodsReceiptFormScreen.tsx | SupplierListScreen :97 -> :205 | pur.title | none | manager | YES
SupplierInvoiceFormScreen | src/features/purchases/SupplierInvoiceFormScreen.tsx | SupplierListScreen :100 -> :206 | pur.title | none | manager | YES
DailySalesReport | src/features/reports/DailySalesReport.tsx | ReportsHomeScreen tile TILES :36 + setActive :80-86, render :59 | rep.daily | none | manager | YES
SalesByProductReport | src/features/reports/SalesByProductReport.tsx | tile :37 | rep.byProduct | none | manager | YES
SalesByCategoryReport | src/features/reports/SalesByCategoryReport.tsx | tile :38 | rep.byCategory | none | manager | YES
GrossMarginReport | src/features/reports/GrossMarginReport.tsx | tile :39 | rep.margin | none | manager | YES
CreditOutstandingReport | src/features/reports/CreditOutstandingReport.tsx | tile :40 | rep.credit | none | manager | YES
LowStockReport | src/features/reports/LowStockReport.tsx | tile :41 | rep.lowStock | none | manager | YES
StockMovementReport | src/features/reports/StockMovementReport.tsx | tile :42 | rep.movements | none | manager | YES
CashSessionsHistoryReport | src/features/reports/CashSessionsHistoryReport.tsx | tile :43 | rep.sessions | none | manager | YES
UserActivityReport | src/features/reports/UserActivityReport.tsx | tile :44 | rep.activity | none | manager | YES
```
Modals:
```text
CheckoutModal | modals/CheckoutModal.tsx 640 | F12 use-pos-keyboard.ts:174-183 (needs lines>0, no credit block, cash session OPEN) | checkout.* | F12 | any authenticated | YES
CustomerSelectModal | modals/CustomerSelectModal.tsx 371 | F2 use-pos-keyboard.ts:131-134; CreditHud button CreditHud.tsx:26 | customer.* | F2 | any | YES
QuantityModal | modals/QuantityModal.tsx 93 | F4 use-pos-keyboard.ts:146-151 (needs a cart line) | quantity.* | F4 | any | YES
DiscountModal | modals/DiscountModal.tsx 98 | F5 use-pos-keyboard.ts:153-158 (needs a cart line) | discount.* | F5 | any | YES
RecordPaymentModal | modals/RecordPaymentModal.tsx 341 | payment-modal-store.openPayment <- CreditHud.tsx:38+44 or CustomerSelectModal.tsx:137+143, then openModal("payment") | payment.* | none | any | YES (two non-fixed openers)
ProductEditModal | modals/ProductEditModal.tsx 258 | ProductDiscovery.tsx:151 openModal("edit-product") | editProductTitle etc | none | any (discovery sits in PosShellView) | YES
OpenCashSessionModal | modals/OpenCashSessionModal.tsx 128 | CashSessionIndicator.tsx:87 | cash.* | none | any | YES (top-bar indicator button)
CloseCashSessionModal | modals/CloseCashSessionModal.tsx 267 | CashSessionIndicator.tsx:105 and :153 | cash.* | none | any | YES
RecordExpenseModal | modals/RecordExpenseModal.tsx 150 | CashSessionIndicator.tsx:143 | cash.* | none | any | YES
DocumentConversionModal | modals/DocumentConversionModal.tsx 378 | PosShell fixed button :84 ONLY (no other openModal("conversion"); no F-key) | conversion.open | none | any | NO -> ONLY-VIA-FIXED
ReturnModal | modals/ReturnModal.tsx 446 | PosShell fixed button :97 ONLY | return.returnModalTitle | none | any | NO -> ONLY-VIA-FIXED
WarehouseTransferModal | modals/WarehouseTransferModal.tsx 429 | PosShell fixed button :111 ONLY | warehousesTitle | none | manager | NO -> ONLY-VIA-FIXED
StockAdjustModal | modals/StockAdjustModal.tsx 454 | PosShell fixed button :126 ONLY | stockAdjust.title | none | manager | NO -> ONLY-VIA-FIXED
BackupModal | modals/BackupModal.tsx 587 | PosShell fixed button :147 ONLY | backups.title | none | ADMIN | NO -> ONLY-VIA-FIXED
StoreIdentityModal | modals/StoreIdentityModal.tsx 245 | PosShell fixed button :162 ONLY | settings.identity.title | none | ADMIN | NO -> ONLY-VIA-FIXED
RecordSupplierPaymentModal | features/purchases/RecordSupplierPaymentModal.tsx 163 | SupplierListScreen.tsx:236 (props token/supplier/onClose) | sup./pur.* | none | manager | YES
SupplierFormModal | features/suppliers/SupplierFormModal.tsx 143 | SupplierListScreen.tsx:211 (props) | sup.edit etc | none | manager | YES
SupplierStatementView | features/suppliers/SupplierStatementView.tsx 115 | SupplierListScreen.tsx:226 (props; fixed inset-0 overlay div at :45, modal-like but named view) | sup.statement | none | manager | YES
ModalShell | modals/ModalShell.tsx 29 | imported by 15 modals (shell, not store-opened) | n/a | n/a | n/a | n/a
```
Opener-exhaustiveness evidence: `grep -rn openModal(" src` (non-test) returns exactly: cash-open/cash-close/cash-expense (CashSessionIndicator :87,:105,:143,:153), customer (CreditHud :26), payment (CreditHud :44, CustomerSelectModal :143), conversion/return/warehouse-transfer/stock-adjust/backup/identity (PosShell :84,:97,:111,:126,:147,:162), edit-product (ProductDiscovery :151), checkout/quantity/discount/customer (use-pos-keyboard :181,:149,:156,:133). No second opener exists for the six C2 targets.

### C4. Conclusion (strictly from the C3 table)
ONLY-VIA-FIXED surfaces (6): DocumentConversionModal and ReturnModal (every authenticated role); WarehouseTransferModal and StockAdjustModal (managers only); BackupModal and StoreIdentityModal (admins only). Each has no keyboard shortcut and no alternate opener. The cashier role (non-manager, non-admin) can reach: all screens except suppliers/reports/catalog views, all purchase screens (no - they are manager-only), i.e. PosShell plus conversion/return via the two ungated FABs, checkout (F12), customer (F2/button), quantity (F4), discount (F5), payment (via customer/CreditHud), and the three cash modals (via CashSessionIndicator). The cashier cannot reach the four gated FAB modals (manager/admin buttons are not rendered: PosShell.tsx:104,119,140,156). Hypothesis VERIFIED for the six C2 surfaces, with the stated role qualifications; it does not extend to any other screen or modal. This matches the charter rule (CHARTER-v2.0.md:181 + §9.2 line 225: "No feature may be reachable only through a floating element").

## D. Vitest reconciliation

### D1. Runner output (single run: `npx vitest run`)
```text
per-file (file | tests):
src/stores/store-identity-store.test.ts | 9 tests
src/components/pos/modals/backup-modal.test.tsx | 14 tests
src/components/pos/modals/conversion-modal.test.ts | 17 tests
src/lib/money.test.ts | 10 tests
src/lib/ipc-backups.test.ts | 12 tests
src/lib/ipc-returns.test.ts | 13 tests
src/components/pos/modals/return-modal.test.tsx | 12 tests
src/components/pos/modals/warehouse-transfer-modal.test.ts | 10 tests
src/components/pos/modals/checkout-double-submit.test.ts | 4 tests
src/lib/ipc-conversion.test.ts | 8 tests
src/components/pos/modals/stock-adjust-modal.test.tsx | 8 tests
src/components/pos/modals/product-edit-modal.test.ts | 9 tests
src/lib/ipc-documents.test.ts | 6 tests
src/components/pos/modals/record-payment-wire.test.ts | 16 tests
src/lib/ipc-printing.test.ts | 5 tests
src/lib/ipc-warehouses.test.ts | 5 tests
src/lib/ipc-product-update.test.ts | 4 tests
src/stores/backup-modal-restore.test.ts | 5 tests
src/lib/ipc-preview-totals.test.ts | 3 tests
src/components/pos/modals/checkout-stock-gate.test.ts | 7 tests
src/stores/pos-ui-store.test.ts | 3 tests
src/features/catalog/create-product-wire.test.ts | 3 tests
summary:
Test Files  22 passed (22)
Tests  183 passed (183)
Start at  21:11:10
Duration  4.76s (transform 1.00s, setup 0ms, collect 3.19s, tests 812ms, environment 8ms, prepare 3.63s)
(full stripped log preserved above in compact form; raw log with timings was written by the runner to scratch and deleted per NO_LEFTOVERS)
```

### D2. Static 178 vs runner 183
AUD-011 method (INVENTORY.md "Commands used for counts"): static occurrences of `it("` / `it('` / `` it(` `` / `test("` etc. in *.test.ts(x). Per-file static list sums to 178 (I8).
Runner total is 183 (D1). Delta = +5, located entirely in one file: record-payment-wire.test.ts static 11 vs runner 16.
Evidence (src/components/pos/modals/record-payment-wire.test.ts:139-152):
```text
139|  const codes = [
140|    "INVALID_PAYMENT_AMOUNT",
141|    "CUSTOMER_NOT_FOUND",
142|    "PAYMENT_EXCEEDS_BALANCE",
143|    "OVERPAYMENT_UNAUTHORIZED",
144|    "UNAUTHORIZED",
145|  ];
146|  it.each(codes)("maps %s in both locales (never raw)", (code) => {
147|    const fr = translateBackendError(code, "fr-DZ");
148|    const ar = translateBackendError(code, "ar-DZ");
149|    expect(fr).not.toBe("Erreur interne du noyau");
150|    expect(ar).not.toBe("خطأ داخلي في النواة");
151|    expect(fr.length).toBeGreaterThan(0);
152|    expect(ar.length).toBeGreaterThan(0);
153|  });
154|
155|  it("dispatches the wire through a guarded single-flight spy", async () => {
156|    let inFlight = 0;
157|    let calls = 0;
```
`it.each(codes)` at line 146 iterates a 5-element array (INVALID_PAYMENT_AMOUNT, CUSTOMER_NOT_FOUND, PAYMENT_EXCEEDS_BALANCE, OVERPAYMENT_UNAUTHORIZED, UNAUTHORIZED) and expands to 5 runtime tests while contributing a single static occurrence (and `grep -c "it(\|test("` = 11 does not match the `it.each(` form at all, so the static count holds 11 plain its and the runner counts 11 + 5 = 16). 178 + 5 = 183. All other 21 files match static-runner exactly. EXPLAINED (not UNEXPLAINED).

## E. Tailwind and styling evidence

### E1. tailwind.config.js verbatim with line numbers
```text
1|/** @type {import('tailwindcss').Config} */
2|export default {
3|  content: ["./index.html", "./src/**/*.{ts,tsx}"],
4|  darkMode: "class",
5|  theme: {
6|    extend: {
7|      colors: {
8|        baraka: {
9|          primary: "#0F3D5C",
10|          primaryDark: "#0A2C44",
11|          primaryLight: "#1A5A85",
12|          accent: "#F59E0B",
13|          accentDark: "#D97706",
14|          accentLight: "#FBBF24",
15|          steel: "#64748B",
16|          paper: "#F8FAFC",
17|          ink: "#0F172A",
18|        },
19|        primary: {
20|          900: "#0F3D5C",
21|          600: "#1E6091",
22|        },
23|        accent: {
24|          500: "#F59E0B",
25|        },
26|      },
27|      fontFamily: {
28|        sans: ["Inter", "Cairo", "system-ui", "Segoe UI", "Tahoma", "sans-serif"],
29|        arabic: ["Cairo", "Inter", "system-ui", "Tahoma", "sans-serif"],
30|        mono: ["JetBrains Mono", "ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
31|      },
32|    },
33|  },
34|  plugins: [],
35|};
```

### E1b. src/index.css verbatim with line numbers
```text
1|@tailwind base;
2|@tailwind components;
3|@tailwind utilities;
4|
5|:root {
6|  --baraka-primary: #0f3d5c;
7|  --baraka-accent: #f59e0b;
8|}
9|
10|html,
11|body,
12|#root {
13|  height: 100%;
14|}
15|
16|body {
17|  font-family: Inter, Cairo, system-ui, "Segoe UI", Tahoma, sans-serif;
18|  background-color: #f8fafc;
19|  color: #0f172a;
20|  -webkit-font-smoothing: antialiased;
21|}
22|
23|/* Tabular figures keep monetary columns from shifting (Tailwind
24|   `tabular-nums` also exists; this alias documents the cockpit rule). */
25|.cockpit-nums {
26|  font-variant-numeric: tabular-nums;
27|}
28|
29|.font-arabic {
30|  font-family: Cairo, Inter, system-ui, Tahoma, sans-serif;
31|}
32|
33|.font-mono-baraka {
34|  font-family: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, monospace;
35|}
(bytes=712 sha256=0bac206d3a91f599f660101baefc55b85deb15bb2652075333c2b6c450b7d657)
```

### E2. Physical vs logical utilities (corrected counts)
```text
METHOD: per-token regex /(?<![\w-])(?:variant:)*-?TOKEN(?![a-zA-Z])/g over src/**/*.{ts,tsx} non-test; variant prefixes sm:,md:,lg:,xl:,hover:,focus:,dark:,rtl:,ltr:,first:,last:,odd:,even:,visited:,active:,disabled:,group-hover:; optional leading "-" for negatives; trailing (?![a-zA-Z]) excludes rounded-lg (for rounded-l) and border-slate-* (for border-s).
TOKEN COUNTS:
ml- | 0
mr- | 0
pl- | 0
pr- | 0
left- | 0
right- | 0
text-left | 1
text-right | 0
border-l | 0
border-r | 0
rounded-l | 0
rounded-r | 0
rounded-tl | 0
rounded-tr | 0
rounded-bl | 0
rounded-br | 0
space-x- | 0
divide-x | 0
ms- | 3
me- | 0
ps- | 0
pe- | 0
start- | 0
end- | 7
text-start | 79
text-end | 63
border-s | 0
border-e | 0
rounded-s | 0
rounded-e | 0
rounded-ss | 0
rounded-se | 0
rounded-es | 0
rounded-ee | 0
TOTAL physical=1 logical=152
TOP-15 files by physical count (file | physical | logical):
src/components/pos/modals/TestPrintPreview.tsx | 1 | 0
src/features/reports/GrossMarginReport.tsx | 0 | 11
src/features/reports/SalesByCategoryReport.tsx | 0 | 7
src/features/suppliers/SupplierStatementView.tsx | 0 | 7
src/components/pos/CashSessionIndicator.tsx | 0 | 3
src/components/pos/PosShell.tsx | 0 | 6
src/components/pos/CartTable.tsx | 0 | 5
src/components/pos/CreditHud.tsx | 0 | 3
src/components/pos/modals/CustomerSelectModal.tsx | 0 | 1
src/components/pos/modals/DocumentConversionModal.tsx | 0 | 1
src/components/pos/modals/StockAdjustModal.tsx | 0 | 1
src/components/pos/ProductDiscovery.tsx | 0 | 2
src/features/auth/FirstRunSetupScreen.tsx | 0 | 3
src/features/auth/LoginScreen.tsx | 0 | 2
src/features/purchases/PurchaseOrderFormScreen.tsx | 0 | 3
```
Correction vs AUD-011 I6 (physical 189 vs logical 153): the 189 is a substring-inflation artifact. Verified by grep: `rounded-lg` occurs 188x (a size utility, not directional) and each occurrence contains the substring `rounded-l`; `border-red-500` etc. occur 3x and contain `border-r`. Raw substring total = 188 + 3 + 1 (text-left) = 192 ≈ 189 (remainder = test-file/method variance). With the letter-boundary `(?![a-zA-Z])` the true directional-physical count is 1 (`text-left` in TestPrintPreview.tsx:45; the only other physical hit class-wide). Consequence: the AUD-011 "TOP-15 physical files" list (BackupModal 14, ReturnModal 14, ...) actually ranks `rounded-lg`/`border-red` frequency, not physical-utility use. Logical total 152 vs 153: single-count method variance (AUD-011 did not print its exact regex - I6 says "with the exact re above" but no regex appears in the committed file - so the 1-count difference is not mechanically attributable; per-token lists above are the reproducible record).

### E3. Complete list of `fixed` hits (file:line, className, classification)
Method: every line containing the substring `fixed` in src/ (incl. tests), then classified by context.
```text
src/components/pos/modals/ModalShell.tsx:11: <div className="fixed inset-0 z-50 ..."> => modal overlay
src/components/pos/PosShell.tsx:86: className="fixed bottom-20 end-4 z-40 ... bg-primary-600 ..." (conversion <button>) => floating action button
src/components/pos/PosShell.tsx:99: className="fixed bottom-36 end-4 z-40 ... bg-amber-600 ..." (return <button>) => floating action button
src/components/pos/PosShell.tsx:113: className="fixed bottom-52 end-4 z-40 ... bg-sky-700 ..." (warehouses <button>) => floating action button
src/components/pos/PosShell.tsx:128: className="fixed bottom-[17rem] end-4 z-40 ... bg-slate-700 ..." (stock-adjust <button>) => floating action button
src/components/pos/PosShell.tsx:149: className="fixed bottom-[21rem] end-4 z-40 ... bg-emerald-700 ..." (backup <button>) => floating action button
src/components/pos/PosShell.tsx:164: className="fixed bottom-[25rem] end-4 z-40 ... bg-indigo-700 ..." (identity <button>) => floating action button
src/features/purchases/RecordSupplierPaymentModal.tsx:71: <div className="fixed inset-0 z-50 ..."> => modal overlay
src/features/reports/CashSessionsHistoryReport.tsx:147: <div className="fixed inset-0 z-50 ..."> => modal overlay
src/features/suppliers/SupplierFormModal.tsx:94: <div className="fixed inset-0 z-50 ..."> => modal overlay
src/features/suppliers/SupplierStatementView.tsx:45: <div className="fixed inset-0 z-50 ..."> => modal overlay (statement view rendered as overlay)
src/components/pos/TotalsFooter.tsx:2: comment ("fixed height") => comment
src/features/catalog/ProductCreateForm.tsx:7: comment ("is_kit is fixed false") => comment
src/components/pos/modals/DocumentConversionModal.tsx:29: comment ("prefixed stock-shortage codes") => comment
src/lib/ipc.ts:1044: comment ("fixed at call time") => comment
src/components/pos/modals/conversion-modal.test.ts:359: it("bridges prefixed stock codes in both locales") => test identifier substring (not a utility)
Count: 16 substring hits total = 15 non-test (AUD-011 said "14 total" but enumerated 15 items: 1 ModalShell + 6 PosShell + 4 overlays + 4 comments = 15; the "14" is an arithmetic slip) + 1 test-file identifier. `position: fixed` / `position:fixed` in CSS: 0 (verified by grep, no hits).
```

## F. i18n prefixes
Grouping rule (per INVENTORY.md I7): segment before the first "."; keys without a dot count whole. fr block = src/lib/i18n.ts lines 8-626 (const fr at line 8 through the I18nKey type at 627). ar parity: Record<I18nKey,string> at line 629 enforces compile-time parity; mechanical recount: fr 604 top-level keys, ar 604 top-level keys (606 raw hits minus the 2 registry entries "fr-DZ"/"ar-DZ" at lines 1241-1242, which belong to the dictionaries object, not the ar dictionary). Total 604 per locale CONFIRMED.
```text
prefixes=73 (sorted by count desc; SINGLETON = exactly one key):
backups | 81
rep | 73
settings | 42
returns | 35
stockAdjust | 30
conversion | 30
return | 28
pur | 27
cash | 23
checkout | 22
sup | 22
cat | 22
payment | 19
customer | 17
cart | 15
key | 11
setup | 8
topbar | 7
search | 7
print | 7
login | 6
totals | 5
credit | 5
discount | 5
nav | 4
quantity | 3
common | 3
app | 2
warehousesTitle | 1 | SINGLETON
tabTransfer | 1 | SINGLETON
tabCreateWarehouse | 1 | SINGLETON
sourceWarehouse | 1 | SINGLETON
destinationWarehouse | 1 | SINGLETON
sameWarehouseError | 1 | SINGLETON
selectSourceHelp | 1 | SINGLETON
selectDestHelp | 1 | SINGLETON
addItemToTransfer | 1 | SINGLETON
transferQty | 1 | SINGLETON
transferNotesPlaceholder | 1 | SINGLETON
executeTransfer | 1 | SINGLETON
transferSuccess | 1 | SINGLETON
warehouseCode | 1 | SINGLETON
warehouseName | 1 | SINGLETON
createWarehouseBtn | 1 | SINGLETON
createWarehouseSuccess | 1 | SINGLETON
noItemsInTransfer | 1 | SINGLETON
adminOnlyTab | 1 | SINGLETON
transferProductSearch | 1 | SINGLETON
transferNoResults | 1 | SINGLETON
transferRemoveItem | 1 | SINGLETON
transferSubmitting | 1 | SINGLETON
transferClose | 1 | SINGLETON
transferCancel | 1 | SINGLETON
editProductTitle | 1 | SINGLETON
skuImmutableNotice | 1 | SINGLETON
productNameAr | 1 | SINGLETON
productNameFr | 1 | SINGLETON
sellingPrice | 1 | SINGLETON
costPrice | 1 | SINGLETON
barcode | 1 | SINGLETON
unitName | 1 | SINGLETON
productCategory | 1 | SINGLETON
activeStatus | 1 | SINGLETON
inactiveWarning | 1 | SINGLETON
saveChanges | 1 | SINGLETON
cancelEdit | 1 | SINGLETON
savingChanges | 1 | SINGLETON
updateSuccess | 1 | SINGLETON
invalidPrice | 1 | SINGLETON
nameRequired | 1 | SINGLETON
editProduct | 1 | SINGLETON
marginLabel | 1 | SINGLETON
parked | 1 | SINGLETON
singletons=45 total=604
```

## G. Modals
Escape evidence: 7 modals own a capture-phase window keydown guard (add/removeEventListener lines below); all other modals close on Escape through the global contract src/hooks/use-pos-keyboard.ts:111-119 (`if (event.key === "Escape") { if (modalOpen) ui.closeModal(); ... }`). Focus trap: `grep -rn "FocusLock\|focus-trap\|useFocusTrap\|initialFocus" src` (non-test) = empty -> NOT FOUND for every modal (only QuantityModal.tsx:62 `autoFocus` on an input, which is not a trap). Restore-focus: `grep -rni "returnFocus\|previousFocus\|restoreFocus\|lastFocused" src` (non-test) = empty -> NOT FOUND for every modal. "Opened via" verified in section C3.
```text
BackupModal | modals/BackupModal.tsx | lines=587 | opened via PosShell fixed :147 (ADMIN) | Escape YES own guard :406-426 (capture, suppressed while busy/restoring) + global | traps focus? NO (NOT FOUND) | restores focus? NO (NOT FOUND)
CheckoutModal | modals/CheckoutModal.tsx | lines=640 | opened via F12 use-pos-keyboard.ts:181 | Escape YES own guard :323-352 (F12/Enter/Escape suppressed in flight) + global | NO | NO
CloseCashSessionModal | modals/CloseCashSessionModal.tsx | lines=267 | opened via CashSessionIndicator :105/:153 | Escape YES via global contract (no own listener: addEventListener count 0) | NO | NO
CustomerSelectModal | modals/CustomerSelectModal.tsx | lines=371 | opened via F2 :133 or CreditHud :26 | Escape YES via global (no own listener) | NO | NO
DiscountModal | modals/DiscountModal.tsx | lines=98 | opened via F4? NO - F5 use-pos-keyboard.ts:156 | Escape YES via global (no own listener) | NO | NO
DocumentConversionModal | modals/DocumentConversionModal.tsx | lines=378 | opened via PosShell fixed :84 ONLY | Escape YES own guard :83-99 + global | NO | NO
ModalShell | modals/ModalShell.tsx | lines=29 | shell imported by 15 modals (fixed inset-0 :11); comment :3 notes Escape handled by global contract | Escape n/a (not opened) | NO | NO
OpenCashSessionModal | modals/OpenCashSessionModal.tsx | lines=128 | opened via CashSessionIndicator :87 | Escape YES via global (no own listener) | NO | NO
ProductEditModal | modals/ProductEditModal.tsx | lines=258 | opened via ProductDiscovery :151 | Escape YES own guard :30-44 + global | NO | NO
QuantityModal | modals/QuantityModal.tsx | lines=93 | opened via F4 use-pos-keyboard.ts:149 | Escape YES via global (no own listener); autofocus :62 (not a trap) | traps focus? NO | restores focus? NO
RecordExpenseModal | modals/RecordExpenseModal.tsx | lines=150 | opened via CashSessionIndicator :143 | Escape YES via global (no own listener) | NO | NO
RecordPaymentModal | modals/RecordPaymentModal.tsx | lines=341 | opened via openPayment store <- CreditHud :38/:44 or CustomerSelectModal :137/:143 | Escape YES via global (no own listener) | NO | NO
ReturnModal | modals/ReturnModal.tsx | lines=446 | opened via PosShell fixed :97 ONLY | Escape YES own guard :73-97 + global | NO | NO
StockAdjustModal | modals/StockAdjustModal.tsx | lines=454 | opened via PosShell fixed :126 ONLY | Escape YES own guard :81-105 + global | NO | NO
StoreIdentityModal | modals/StoreIdentityModal.tsx | lines=245 | opened via PosShell fixed :162 ONLY | Escape YES via global (no own listener) | NO | NO
WarehouseTransferModal | modals/WarehouseTransferModal.tsx | lines=429 | opened via PosShell fixed :111 ONLY | Escape YES own guard :53-66 + global | NO | NO
RecordSupplierPaymentModal | features/purchases/RecordSupplierPaymentModal.tsx | lines=163 | opened via props from SupplierListScreen :236 | Escape YES via global (no own listener) | NO | NO
SupplierFormModal | features/suppliers/SupplierFormModal.tsx | lines=143 | opened via props from SupplierListScreen :211 | Escape YES via global (no own listener) | NO | NO
19th slot: AUD-011 I10 says "19 named modals" but enumerates 18 (16 pos/modals incl. ModalShell + RecordSupplierPaymentModal + SupplierFormModal). Candidates seen in code: TestPrintPreview (modals/TestPrintPreview.tsx 59 - an inline role="img" preview, NOT a modal) and SupplierStatementView (overlay div :45, named "view"). No 19th modal component found -> 19th = UNKNOWN (count unverifiable; 18 confirmed).
```

## H. Importer counts (I5)
Method: parse static `from "..."` specifiers in all src non-test files; resolve relative specs against the importer directory and `@/...` against src/; extension-insensitive; count distinct importing files. Scope = every non-test file under src/components, src/features, src/hooks, src/lib, src/stores. Sorted descending. Full table:
```text
=== H1 importer counts (file | distinct non-test importers | importers) ===
METHOD: parse static `from "..."` specifiers in all src non-test files; resolve relative specs against importer dir and "@/..." against src/; extension-insensitive; counts distinct importing files.
src/lib/i18n.ts | 52 | src/App.tsx, src/components/pos/CartPanel.tsx, src/components/pos/CartTable.tsx, src/components/pos/CashSessionIndicator.tsx, src/components/pos/CreditHud.tsx, src/components/pos/KeyFooter.tsx, src/components/pos/PosShell.tsx, src/components/pos/PosShellView.tsx, src/components/pos/PrintStatusIndicator.tsx, src/components/pos/ProductDiscovery.tsx, src/components/pos/Topbar.tsx, src/components/pos/TotalsFooter.tsx, src/components/pos/modals/BackupModal.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/DiscountModal.tsx, src/components/pos/modals/DocumentConversionModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/ProductEditModal.tsx, src/components/pos/modals/QuantityModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/StoreIdentityModal.tsx, src/components/pos/modals/WarehouseTransferModal.tsx, src/features/auth/FirstRunSetupScreen.tsx, src/features/auth/LoginScreen.tsx, src/features/catalog/CatalogModule.tsx, src/features/catalog/ProductCreateForm.tsx, src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/RecordSupplierPaymentModal.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/CreditOutstandingReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/LowStockReport.tsx, src/features/reports/ReportsHomeScreen.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/reports/UserActivityReport.tsx, src/features/reports/components/ReportDateRangePicker.tsx, src/features/suppliers/SupplierFormModal.tsx, src/features/suppliers/SupplierListScreen.tsx, src/features/suppliers/SupplierStatementView.tsx, src/hooks/use-pos-keyboard.ts, src/stores/backup-modal-store.ts, src/stores/store-identity-store.ts
src/stores/pos-ui-store.ts | 52 | src/App.tsx, src/components/pos/CartPanel.tsx, src/components/pos/CartTable.tsx, src/components/pos/CashSessionIndicator.tsx, src/components/pos/CreditHud.tsx, src/components/pos/KeyFooter.tsx, src/components/pos/PosShell.tsx, src/components/pos/PosShellView.tsx, src/components/pos/PrintStatusIndicator.tsx, src/components/pos/ProductDiscovery.tsx, src/components/pos/Topbar.tsx, src/components/pos/TotalsFooter.tsx, src/components/pos/modals/BackupModal.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/DiscountModal.tsx, src/components/pos/modals/DocumentConversionModal.tsx, src/components/pos/modals/ModalShell.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/ProductEditModal.tsx, src/components/pos/modals/QuantityModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/StoreIdentityModal.tsx, src/components/pos/modals/WarehouseTransferModal.tsx, src/features/auth/FirstRunSetupScreen.tsx, src/features/auth/LoginScreen.tsx, src/features/catalog/CatalogModule.tsx, src/features/catalog/ProductCreateForm.tsx, src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/RecordSupplierPaymentModal.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/CreditOutstandingReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/LowStockReport.tsx, src/features/reports/ReportsHomeScreen.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/reports/UserActivityReport.tsx, src/features/reports/components/ReportDateRangePicker.tsx, src/features/suppliers/SupplierFormModal.tsx, src/features/suppliers/SupplierListScreen.tsx, src/features/suppliers/SupplierStatementView.tsx, src/hooks/use-catalog-search.ts, src/hooks/use-pos-keyboard.ts
src/lib/ipc.ts | 44 | src/App.tsx, src/components/pos/CashSessionIndicator.tsx, src/components/pos/CreditHud.tsx, src/components/pos/PrintStatusIndicator.tsx, src/components/pos/modals/BackupModal.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/StoreIdentityModal.tsx, src/components/pos/modals/TestPrintPreview.tsx, src/components/pos/modals/checkout-submission.ts, src/features/catalog/CatalogModule.tsx, src/features/catalog/ProductCreateForm.tsx, src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/RecordSupplierPaymentModal.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/CreditOutstandingReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/LowStockReport.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/reports/UserActivityReport.tsx, src/features/suppliers/SupplierStatementView.tsx, src/hooks/use-catalog-search.ts, src/hooks/use-pos-keyboard.ts, src/stores/backup-modal-store.ts, src/stores/cash-session-store.ts, src/stores/conversion-modal-store.ts, src/stores/product-edit-store.ts, src/stores/return-modal-store.ts, src/stores/session-store.ts, src/stores/stock-adjust-modal-store.ts, src/stores/store-identity-store.ts, src/stores/supplier-store.ts, src/stores/warehouse-transfer-store.ts
src/stores/session-store.ts | 44 | src/App.tsx, src/components/pos/CashSessionIndicator.tsx, src/components/pos/CreditHud.tsx, src/components/pos/PosShell.tsx, src/components/pos/PrintStatusIndicator.tsx, src/components/pos/ProductDiscovery.tsx, src/components/pos/Topbar.tsx, src/components/pos/modals/BackupModal.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/StoreIdentityModal.tsx, src/components/pos/modals/WarehouseTransferModal.tsx, src/features/auth/FirstRunSetupScreen.tsx, src/features/auth/LoginScreen.tsx, src/features/catalog/CatalogModule.tsx, src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/CreditOutstandingReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/LowStockReport.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/reports/UserActivityReport.tsx, src/features/suppliers/SupplierListScreen.tsx, src/hooks/use-catalog-search.ts, src/hooks/use-pos-keyboard.ts, src/lib/ipc.ts, src/stores/backup-modal-store.ts, src/stores/conversion-modal-store.ts, src/stores/product-edit-store.ts, src/stores/return-modal-store.ts, src/stores/stock-adjust-modal-store.ts, src/stores/store-identity-store.ts, src/stores/warehouse-transfer-store.ts
src/lib/money.ts | 43 | src/components/pos/CartTable.tsx, src/components/pos/CashSessionIndicator.tsx, src/components/pos/CreditHud.tsx, src/components/pos/ProductDiscovery.tsx, src/components/pos/TotalsFooter.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/DiscountModal.tsx, src/components/pos/modals/DocumentConversionModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/QuantityModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/WarehouseTransferModal.tsx, src/components/pos/modals/checkout-submission.ts, src/features/catalog/CatalogModule.tsx, src/features/catalog/ProductCreateForm.tsx, src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/RecordSupplierPaymentModal.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/CreditOutstandingReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/LowStockReport.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/suppliers/SupplierListScreen.tsx, src/features/suppliers/SupplierStatementView.tsx, src/hooks/use-pos-keyboard.ts, src/lib/purchase-lines.ts, src/stores/cart-store.ts, src/stores/conversion-modal-store.ts, src/stores/product-edit-store.ts, src/stores/return-modal-store.ts, src/stores/session-store.ts, src/stores/stock-adjust-modal-store.ts, src/stores/warehouse-transfer-store.ts
src/components/pos/modals/ModalShell.tsx | 15 | src/components/pos/modals/BackupModal.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/DiscountModal.tsx, src/components/pos/modals/DocumentConversionModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/ProductEditModal.tsx, src/components/pos/modals/QuantityModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/RecordPaymentModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/components/pos/modals/StockAdjustModal.tsx, src/components/pos/modals/StoreIdentityModal.tsx, src/components/pos/modals/WarehouseTransferModal.tsx
src/stores/cart-store.ts | 13 | src/components/pos/CartPanel.tsx, src/components/pos/CartTable.tsx, src/components/pos/CreditHud.tsx, src/components/pos/KeyFooter.tsx, src/components/pos/PosShell.tsx, src/components/pos/ProductDiscovery.tsx, src/components/pos/TotalsFooter.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CustomerSelectModal.tsx, src/components/pos/modals/DiscountModal.tsx, src/components/pos/modals/QuantityModal.tsx, src/hooks/use-catalog-search.ts, src/hooks/use-pos-keyboard.ts
src/stores/cash-session-store.ts | 10 | src/components/pos/CashSessionIndicator.tsx, src/components/pos/KeyFooter.tsx, src/components/pos/PosShellView.tsx, src/components/pos/modals/CheckoutModal.tsx, src/components/pos/modals/CloseCashSessionModal.tsx, src/components/pos/modals/OpenCashSessionModal.tsx, src/components/pos/modals/RecordExpenseModal.tsx, src/components/pos/modals/ReturnModal.tsx, src/hooks/use-pos-keyboard.ts, src/stores/session-store.ts
src/features/reports/components/ReportDateRangePicker.tsx | 7 | src/features/reports/CashSessionsHistoryReport.tsx, src/features/reports/DailySalesReport.tsx, src/features/reports/GrossMarginReport.tsx, src/features/reports/SalesByCategoryReport.tsx, src/features/reports/SalesByProductReport.tsx, src/features/reports/StockMovementReport.tsx, src/features/reports/UserActivityReport.tsx
src/features/purchases/PurchaseOrderFormScreen.tsx | 3 | src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx, src/features/suppliers/SupplierListScreen.tsx
src/lib/purchase-lines.ts | 3 | src/features/purchases/GoodsReceiptFormScreen.tsx, src/features/purchases/PurchaseOrderFormScreen.tsx, src/features/purchases/SupplierInvoiceFormScreen.tsx
src/stores/payment-modal-store.ts | 3 | src/components/pos/CreditHud.tsx, src/components/pos/PosShell.tsx, src/components/pos/modals/CustomerSelectModal.tsx
src/features/reports/components/report-format.ts | 2 | src/features/reports/CreditOutstandingReport.tsx, src/features/reports/GrossMarginReport.tsx
src/stores/backup-modal-store.ts | 2 | src/components/pos/PosShell.tsx, src/components/pos/modals/BackupModal.tsx
src/stores/conversion-modal-store.ts | 2 | src/components/pos/PosShell.tsx, src/components/pos/modals/DocumentConversionModal.tsx
src/stores/product-edit-store.ts | 2 | src/components/pos/ProductDiscovery.tsx, src/components/pos/modals/ProductEditModal.tsx
src/stores/return-modal-store.ts | 2 | src/components/pos/PosShell.tsx, src/components/pos/modals/ReturnModal.tsx
src/stores/stock-adjust-modal-store.ts | 2 | src/components/pos/PosShell.tsx, src/components/pos/modals/StockAdjustModal.tsx
src/stores/supplier-store.ts | 2 | src/features/suppliers/SupplierFormModal.tsx, src/features/suppliers/SupplierListScreen.tsx
src/stores/warehouse-transfer-store.ts | 2 | src/components/pos/PosShell.tsx, src/components/pos/modals/WarehouseTransferModal.tsx
src/components/pos/CartPanel.tsx | 1 | src/components/pos/PosShellView.tsx
src/components/pos/CartTable.tsx | 1 | src/components/pos/CartPanel.tsx
src/components/pos/CashSessionIndicator.tsx | 1 | src/components/pos/Topbar.tsx
src/components/pos/CreditHud.tsx | 1 | src/components/pos/CartPanel.tsx
src/components/pos/KeyFooter.tsx | 1 | src/components/pos/PosShellView.tsx
src/components/pos/modals/BackupModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/checkout-submission.ts | 1 | src/components/pos/modals/CheckoutModal.tsx
src/components/pos/modals/CheckoutModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/CloseCashSessionModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/CustomerSelectModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/DiscountModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/DocumentConversionModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/OpenCashSessionModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/ProductEditModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/QuantityModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/RecordExpenseModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/RecordPaymentModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/ReturnModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/stock-gate.ts | 1 | src/components/pos/modals/CheckoutModal.tsx
src/components/pos/modals/StockAdjustModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/StoreIdentityModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/modals/TestPrintPreview.tsx | 1 | src/components/pos/modals/StoreIdentityModal.tsx
src/components/pos/modals/WarehouseTransferModal.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/PosShell.tsx | 1 | src/App.tsx
src/components/pos/PosShellView.tsx | 1 | src/components/pos/PosShell.tsx
src/components/pos/PrintStatusIndicator.tsx | 1 | src/components/pos/modals/CheckoutModal.tsx
src/components/pos/ProductDiscovery.tsx | 1 | src/components/pos/PosShellView.tsx
src/components/pos/Topbar.tsx | 1 | src/components/pos/PosShellView.tsx
src/components/pos/TotalsFooter.tsx | 1 | src/components/pos/CartPanel.tsx
src/features/auth/FirstRunSetupScreen.tsx | 1 | src/App.tsx
src/features/auth/LoginScreen.tsx | 1 | src/App.tsx
src/features/catalog/CatalogModule.tsx | 1 | src/App.tsx
src/features/catalog/ProductCreateForm.tsx | 1 | src/features/catalog/CatalogModule.tsx
src/features/purchases/GoodsReceiptFormScreen.tsx | 1 | src/features/suppliers/SupplierListScreen.tsx
src/features/purchases/RecordSupplierPaymentModal.tsx | 1 | src/features/suppliers/SupplierListScreen.tsx
src/features/purchases/SupplierInvoiceFormScreen.tsx | 1 | src/features/suppliers/SupplierListScreen.tsx
src/features/reports/CashSessionsHistoryReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/CreditOutstandingReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/DailySalesReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/GrossMarginReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/LowStockReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/ReportsHomeScreen.tsx | 1 | src/App.tsx
src/features/reports/SalesByCategoryReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/SalesByProductReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/StockMovementReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/reports/UserActivityReport.tsx | 1 | src/features/reports/ReportsHomeScreen.tsx
src/features/suppliers/SupplierFormModal.tsx | 1 | src/features/suppliers/SupplierListScreen.tsx
src/features/suppliers/SupplierListScreen.tsx | 1 | src/App.tsx
src/features/suppliers/SupplierStatementView.tsx | 1 | src/features/suppliers/SupplierListScreen.tsx
src/hooks/use-catalog-search.ts | 1 | src/components/pos/ProductDiscovery.tsx
src/hooks/use-pos-keyboard.ts | 1 | src/components/pos/PosShell.tsx
src/lib/tauri.ts | 1 | src/hooks/use-app-status.ts
src/stores/app-view-store.ts | 1 | src/App.tsx
src/stores/store-identity-store.ts | 1 | src/components/pos/modals/StoreIdentityModal.tsx
src/components/status-card.tsx | 0 | none
src/features/health/health-check.ts | 0 | none
src/hooks/use-app-status.ts | 0 | none
src/lib/format.ts | 0 | none
src/stores/app-store.ts | 0 | none
```

## I. Money formatting spot-check
```text
src/lib/format.ts lines 1-40 verbatim with line numbers (file has 17 lines total):
1|/** Deterministic DZD formatting. Input is integer centimes; never float. */
2|
3|const dzdFormatter = new Intl.NumberFormat("fr-DZ", {
4|  style: "currency",
5|  currency: "DZD",
6|  minimumFractionDigits: 2,
7|  maximumFractionDigits: 2,
8|});
9|
10|/** Format integer centimes as a French DZD string, e.g. 10050 -> "100,50 DZD". */
11|export function formatDzd(centimes: number): string {
12|  if (!Number.isInteger(centimes)) {
13|    throw new RangeError("formatDzd expects integer centimes");
14|  }
15|  return dzdFormatter.format(centimes / 100);
16|}
17|
Call sites of formatDzd (the function containing `/ 100` at format.ts:15): `grep -rn "formatDzd" src` returns ONLY src/lib/format.ts:11 (definition) and :13 (throw line). Zero call sites in production code, zero in tests: the function is currently dead code.
Whether any caller can pass a non-integer: VACUOUS - there are no callers. The parameter type is bare `number` (not branded), so a future caller COULD pass a non-integer; the Number.isInteger guard (:12-14) throws RangeError in that case. Precision note (same as AUD-011 I9): float64 holds integers exactly to 2^53; no judgment beyond the types seen.
```

## UNKNOWNs
- E2 logical 153 vs 152 single-count delta: AUD-011 did not print its exact regex, so the 1-count difference is not mechanically attributable. UNKNOWN (method variance, reproducible record above).
- G1 19th modal: 18 confirmed; the 19th name is not determinable from code (candidates TestPrintPreview/SupplierStatementView are not modals). UNKNOWN (see G1 note).
- Scanner-as-keyboard wedge timing behavior, modal focus-restore expectations, and any runtime-only behavior: not mechanically determinable from static reads. UNKNOWN (same as AUD-011).

## Files
- [CREATED] D:/Dev/Baraka UltraPOS/audit/AUD-011/INVENTORY-C1.md (this file; size and SHA-256 printed in the executor report, identical content to what was printed inline).

NEXT READY: YES

