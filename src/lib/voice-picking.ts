/**
 * Experimental voice picking (S6): Web Speech -> the SAME catalog search.
 * Zero engine, zero schema. The mic button only renders when the operator
 * opts in via `localStorage["baraka.voicePicking"] = "1"` (deliberate
 * procedure, documented in PROPOSALS S6 — never on by default).
 * Transcript lands in the search box (visual confirm); the operator picks
 * the row and adds it through the existing flow — no auto-add, ever.
 */

/** Minimal Web Speech surface (TS DOM lib ships no SpeechRecognition). */
interface SpeechRecognitionLike {
  lang: string;
  interimResults: boolean;
  maxAlternatives: number;
  onresult: ((event: { results: ArrayLike<ArrayLike<{ transcript: string }>> }) => void) | null;
  onerror: ((event: { error: string }) => void) | null;
  onend: (() => void) | null;
  start: () => void;
  stop: () => void;
}

type SpeechWindow = Window & {
  SpeechRecognition?: new () => SpeechRecognitionLike;
  webkitSpeechRecognition?: new () => SpeechRecognitionLike;
};

const FLAG_KEY = "baraka.voicePicking";

/** Opt-in flag. Safe in non-DOM harnesses (`localStorage` absent = off). */
export function isVoicePickingEnabled(): boolean {
  try {
    return typeof localStorage !== "undefined" && localStorage.getItem(FLAG_KEY) === "1";
  } catch {
    return false;
  }
}

/** Deliberate enable/disable procedure (settings console, never UI-default). */
export function setVoicePickingEnabled(on: boolean): void {
  try {
    if (typeof localStorage === "undefined") {
      return;
    }
    if (on) {
      localStorage.setItem(FLAG_KEY, "1");
    } else {
      localStorage.removeItem(FLAG_KEY);
    }
  } catch {
    // Storage blocked (private mode): flag stays off, voice stays hidden.
  }
}

/** Browser actually ships a recognizer (Chrome/Edge desktop; not Firefox). */
export function isSpeechRecognitionSupported(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const candidate = window as unknown as SpeechWindow;
  return (
    typeof candidate.SpeechRecognition === "function" ||
    typeof candidate.webkitSpeechRecognition === "function"
  );
}

export interface VoiceCallbacks {
  onTranscript: (text: string) => void;
  onSpeechError: (code: string) => void;
  onEnd: () => void;
}

/**
 * One-shot recognition in the operator locale. Returns a `stop` thunk.
 * Throws when unsupported — callers check `isSpeechRecognitionSupported`.
 */
export function startVoiceSearch(locale: string, callbacks: VoiceCallbacks): () => void {
  const candidate = window as unknown as SpeechWindow;
  const factory = candidate.SpeechRecognition ?? candidate.webkitSpeechRecognition;
  if (typeof factory !== "function") {
    throw new Error("SPEECH_NOT_SUPPORTED");
  }
  const recognition = new factory();
  recognition.lang = locale === "ar-DZ" ? "ar-DZ" : "fr-FR";
  recognition.interimResults = false;
  recognition.maxAlternatives = 1;
  recognition.onresult = (event) => {
    const first = event.results[0]?.[0]?.transcript.trim();
    if (first !== undefined && first !== "") {
      callbacks.onTranscript(first);
    }
  };
  recognition.onerror = (event) => {
    callbacks.onSpeechError(event.error);
  };
  recognition.onend = () => {
    callbacks.onEnd();
  };
  recognition.start();
  return () => {
    try {
      recognition.stop();
    } catch {
      // Already ended: the onend path owns the state.
    }
  };
}
