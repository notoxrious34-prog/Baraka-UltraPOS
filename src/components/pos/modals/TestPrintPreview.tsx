/**
 * Test-print preview (TASK-BRK-056).
 *
 * Renders PreviewLine design lines produced by the shared backend
 * layout blocks — never decoded from pixels, never fabricated
 * client-side. Max width ≈42ch approximates the rasterizer column
 * count (PAPER_WIDTH_DOTS = 576 dots ≈ 42 columns at the body glyph
 * advance; the rasterizer constant remains the single source of
 * truth — this is a visual approximation only).
 */

import type { ReactElement } from "react";
import type { PreviewLine } from "@/lib/ipc";

/** Visual scale for a line: large/bold emphasis vs normal body. */
function scaleFor(line: PreviewLine): "large" | "normal" {
  return line.sizePx >= 30 ? "large" : "normal";
}

export function TestPrintPreview({ lines }: { lines: PreviewLine[] }): ReactElement {
  return (
    <div
      dir="ltr"
      role="img"
      aria-label="ticket preview"
      className="mx-auto w-full max-w-[42ch] overflow-x-auto rounded-md bg-slate-950 p-3 font-mono text-[13px] leading-relaxed text-slate-100"
      style={{ fontFamily: "'JetBrains Mono', ui-monospace, monospace" }}
    >
      {lines.map((line, index) => {
        if (line.kind === "rule") {
          return (
            <hr
              key={index}
              aria-hidden="true"
              className="my-1 border-t-2 border-slate-100"
            />
          );
        }
        if (line.kind === "gap") {
          return <div key={index} aria-hidden="true" className="h-3" />;
        }
        return (
          <div
            key={index}
            className={line.centered ? "text-center" : "text-left"}
            style={
              scaleFor(line) === "large"
                ? { fontWeight: 800, fontSize: "1.25rem" }
                : undefined
            }
          >
            {line.text === "" ? " " : line.text}
          </div>
        );
      })}
    </div>
  );
}
