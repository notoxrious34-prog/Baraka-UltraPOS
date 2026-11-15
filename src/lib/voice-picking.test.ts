import { describe, expect, it } from "vitest";

import { isSpeechRecognitionSupported, isVoicePickingEnabled } from "@/lib/voice-picking";

describe("voice picking gate", () => {
  it("stays off without a DOM store", () => {
    // Node harness: no localStorage -> flag unreadable -> hidden.
    expect(isVoicePickingEnabled()).toBe(false);
  });

  it("reports no recognizer in the node harness", () => {
    expect(isSpeechRecognitionSupported()).toBe(false);
  });
});
