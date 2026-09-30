# Baraka ESC/POS Skill
Use for thermal printing and receipt rendering.

- Generate ReceiptIR before byte rendering.
- Latin content may use native ESC/POS text modes.
- Arabic requires deterministic shaping/rasterization according to the architecture.
- Printing occurs after a committed sale from a durable print job.
- Retries must be bounded and duplicate reprints must be visibly marked.
