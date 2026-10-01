# ADR-066 - Viewport baseline and window policy
Status: ACCEPTED (SESSION-ARC-001)
Context: AUD-003 measured the owner's screen: physical 1920x1080 at 150% scale, logical working area 1280x689, maximized usable area about 1280x666. tauri.conf.json declares default 1280x800 and minHeight 768, both taller than the working area. The two header rows consume about 96 px of 666.
Decision: Design baseline is 1280x660 CSS px. Hard minimum is 1024x600. Tiers: S (width 1024-1279 or height 600-719), M (width 1280-1535), L (width 1536 and above). The POS screen never shows page scroll or clipped controls at any viewport of 1024x600 or larger. Target window configuration for the engine task: default inner size 1280x640, minWidth 1024, minHeight 600; that task must verify by measurement that the outer window fits the 1280x689 working area.
Consequences: Every design and every DSN task is verified on the viewport matrix. tauri.conf.json changes happen only in a BRK task.
Validation gate: none.
