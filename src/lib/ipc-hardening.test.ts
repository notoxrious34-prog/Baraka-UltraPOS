/**
 * Contract tests for the dual-USB hardening IPC wrappers (TASK-BRK-128).
 *
 * Pins the exact backend command names (`backup_hardening_status`,
 * `backup_hardening_configure`, `backup_schedule_run`), wire shapes
 * field-for-field (including `usb_destination_2` and `stale_48h`), the
 * snake_case-to-camelCase mapping, and verbatim error propagation.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  configureHardening,
  getHardeningStatus,
  runScheduledBackup,
} from "@/lib/ipc";
import { useSessionStore } from "@/stores/session-store";
import type { SessionInfo } from "@/types/pos";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

function fakeTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  globals.window = globalThis;
  globals.__TAURI_INTERNALS__ = {};
}

function clearTauriWindow(): void {
  const globals = globalThis as unknown as Record<string, unknown>;
  delete globals.window;
  delete globals.__TAURI_INTERNALS__;
}

function primeSession(token: string): void {
  const session: SessionInfo = {
    token,
    userId: "55555555-5555-5555-5555-555555555555",
    username: "admin1",
    role: "ADMIN",
    expiresAt: "2026-01-01T00:00:00Z",
    permissions: [],
  };
  useSessionStore.setState({ session });
}

beforeEach(() => {
  fakeTauriWindow();
  invokeMock.mockReset();
});

afterEach(() => {
  clearTauriWindow();
  useSessionStore.setState({ session: null });
});

function statusWire(stale: boolean): Record<string, unknown> {
  return {
    schedule: { cadence: "daily", time: "02:00" },
    usb_destination: "E:/BRK1",
    usb_destination_2: "F:/BRK2",
    last_status: {
      status: "SUCCESS",
      file_name: "baraka_pos_backup_20261025T020000_x.db",
      created_at: "2026-10-25T02:00:00.000Z",
      error_message: null,
      stale_48h: stale,
    },
    due: { due: false, last_backup_at: "2026-10-25T02:00:00.000Z", next_run_at: null },
  };
}

describe("dual-USB hardening IPC contract (BRK-128)", () => {
  it("fetches status with both destinations and the staleness flag", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue(statusWire(true));
    const status = await getHardeningStatus("tok");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      Record<string, unknown>,
    ];
    expect(command).toBe("backup_hardening_status");
    expect(args.session_token).toBe("tok");
    expect(status.usbDestination).toBe("E:/BRK1");
    expect(status.usbDestination2).toBe("F:/BRK2");
    expect(status.stale48h).toBe(true);
    expect(status.cadence).toBe("daily");
  });

  it("configures both destinations and propagates errors verbatim", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({ cadence: "daily", time: "02:00" });
    await configureHardening("tok", "daily", "02:00", "E:/BRK1", "F:/BRK2");
    const [command, args] = invokeMock.mock.calls[0] as unknown as [
      string,
      Record<string, unknown>,
    ];
    expect(command).toBe("backup_hardening_configure");
    const request = args.request as Record<string, unknown>;
    expect(request.usb_destination).toBe("E:/BRK1");
    expect(request.usb_destination_2).toBe("F:/BRK2");
    invokeMock.mockRejectedValue("INVALID_BACKUP_TIME");
    await expect(
      configureHardening("tok", "daily", "99:99", "E:/BRK1", null),
    ).rejects.toBe("INVALID_BACKUP_TIME");
  });

  it("returns the mirror count from the scheduled run", async () => {
    primeSession("tok");
    invokeMock.mockResolvedValue({
      backup: { id: "b1" },
      usb: [{ destination: "E:/BRK1" }, { destination: "F:/BRK2" }],
    });
    const mirrors = await runScheduledBackup("tok");
    expect(invokeMock).toHaveBeenCalledTimes(1);
    const [command] = invokeMock.mock.calls[0] as unknown as [string];
    expect(command).toBe("backup_schedule_run");
    expect(mirrors).toBe(2);
  });
});
