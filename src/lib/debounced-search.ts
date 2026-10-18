/**
 * Shared debounced-search runner (TASK-DSN-067).
 *
 * Project standard for modal instant search (established in TASK-DSN-064):
 * 175–200ms debounce, monotonic ticket guard against stale out-of-order async
 * responses, immediate flush on Enter, instant cancel on Escape/clear.
 * Components own result state; the runner owns timing and ticket identity.
 */

export interface DebouncedSearchOptions {
  delayMs: number;
  shouldSearch?: (query: string) => boolean;
  onSearch: (query: string, ticket: number) => void;
}

export interface DebouncedSearchRunner {
  /** Schedule a debounced search; returns this attempt's ticket. */
  search: (query: string) => number;
  /** Fire the pending query immediately (Enter); no-op when idle. */
  flush: () => void;
  /** Cancel the pending query without firing (Escape/clear). */
  reset: () => void;
  /** Cancel any pending query (unmount cleanup). */
  dispose: () => void;
  /** True only for the most recent attempt's ticket. */
  isLatest: (ticket: number) => boolean;
}

const DEFAULT_SHOULD_SEARCH = (query: string): boolean => query.trim() !== "";

export function createDebouncedSearchRunner(
  options: DebouncedSearchOptions,
): DebouncedSearchRunner {
  const shouldSearch = options.shouldSearch ?? DEFAULT_SHOULD_SEARCH;
  const { delayMs, onSearch } = options;
  let latestTicket = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pendingQuery: string | null = null;

  const clearTimer = (): void => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };

  return {
    search: (query: string): number => {
      latestTicket += 1;
      const ticket = latestTicket;
      clearTimer();
      if (!shouldSearch(query)) {
        pendingQuery = null;
        return ticket;
      }
      pendingQuery = query;
      timer = setTimeout(() => {
        timer = null;
        const fire = pendingQuery;
        pendingQuery = null;
        if (fire !== null) {
          onSearch(fire, ticket);
        }
      }, delayMs);
      return ticket;
    },
    flush: (): void => {
      if (timer === null || pendingQuery === null) {
        return;
      }
      clearTimer();
      const fire = pendingQuery;
      pendingQuery = null;
      onSearch(fire, latestTicket);
    },
    reset: (): void => {
      clearTimer();
      pendingQuery = null;
    },
    dispose: (): void => {
      clearTimer();
      pendingQuery = null;
    },
    isLatest: (ticket: number): boolean => ticket === latestTicket,
  };
}
