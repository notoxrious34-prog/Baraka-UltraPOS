/**
 * Unit tests for the shared debounced-search runner (TASK-DSN-067).
 * Pure timer logic — no DOM. Fake timers control the debounce interval.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  createDebouncedSearchRunner,
  type DebouncedSearchRunner,
} from "@/lib/debounced-search";

let runner: DebouncedSearchRunner | null = null;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  runner?.dispose();
  runner = null;
  vi.useRealTimers();
});

describe("createDebouncedSearchRunner", () => {
  it("does not fire synchronously; fires once with the latest query after the delay", () => {
    const fired: string[] = [];
    runner = createDebouncedSearchRunner({
      delayMs: 175,
      onSearch: (query) => {
        fired.push(query);
      },
    });
    runner.search("Ca");
    runner.search("Cab");
    expect(fired).toEqual([]);
    vi.advanceTimersByTime(175);
    expect(fired).toEqual(["Cab"]);
  });

  it("flush fires the pending query immediately without waiting", () => {
    const fired: string[] = [];
    runner = createDebouncedSearchRunner({
      delayMs: 175,
      onSearch: (query) => {
        fired.push(query);
      },
    });
    runner.search("Cab");
    runner.flush();
    expect(fired).toEqual(["Cab"]);
    vi.advanceTimersByTime(1000);
    expect(fired).toEqual(["Cab"]);
  });

  it("reset cancels the pending query; clear-gated queries never schedule", () => {
    const fired: string[] = [];
    runner = createDebouncedSearchRunner({
      delayMs: 175,
      onSearch: (query) => {
        fired.push(query);
      },
    });
    runner.search("Cab");
    runner.reset();
    vi.advanceTimersByTime(1000);
    expect(fired).toEqual([]);
    runner.search("");
    runner.search("   ");
    vi.advanceTimersByTime(1000);
    expect(fired).toEqual([]);
  });

  it("honours a minimum-length gate (return-modal lookup contract)", () => {
    const fired: string[] = [];
    runner = createDebouncedSearchRunner({
      delayMs: 200,
      shouldSearch: (query) => query.trim().length >= 2,
      onSearch: (query) => {
        fired.push(query);
      },
    });
    runner.search("F");
    vi.advanceTimersByTime(500);
    expect(fired).toEqual([]);
    runner.search("FA");
    vi.advanceTimersByTime(200);
    expect(fired).toEqual(["FA"]);
  });

  it("marks superseded tickets stale so out-of-order responses are ignored", () => {
    const firedTickets: number[] = [];
    runner = createDebouncedSearchRunner({
      delayMs: 175,
      onSearch: (_query, ticket) => {
        firedTickets.push(ticket);
      },
    });
    const first = runner.search("Cab");
    vi.advanceTimersByTime(175);
    // First response still in flight while the user types on...
    const second = runner.search("Cabi");
    vi.advanceTimersByTime(175);
    expect(first).not.toBe(second);
    expect(firedTickets).toEqual([first, second]);
    // ...when the first response finally resolves, its ticket is stale.
    expect(runner.isLatest(first)).toBe(false);
    expect(runner.isLatest(second)).toBe(true);
  });
});
