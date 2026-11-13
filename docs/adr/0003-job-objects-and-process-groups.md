# ADR-0003: Process containment is the source of truth, not the root PID

**Status:** Accepted · 2026-11-13

## Context

A launcher that reports playtime must answer one question correctly: *is the
game still running?* The obvious implementation — `child.wait()` on the PID we
spawned — is wrong for most real games.

Observed failure modes:

- **Windows.** `EpicGamesLauncher.exe`, Uplay, `MinecraftLauncher.exe`, `.bat`
  shims and DRM bootstrappers exit within a second or two of starting the real
  game. `child.wait()` returns immediately, so playtime is ~2 s.
- **Linux/Proton.** `umu-run` and `wine` hand off to `wineserver` and exit. The
  game lives on in a process whose parent has already gone.
- **Anti-cheat / helper chains.** Games spawn supervised children that must also
  die when the user asks to quit, or they leak.

`sysinfo` was considered as an alternative signal and rejected: its refresh
granularity and PID-reuse behaviour make it unsuitable for determining exit.

## Decision

Model each launch as a **containment** and poll the containment's occupancy.

- **Windows:** a Job Object, attached at process creation via
  `PROC_THREAD_ATTRIBUTE_JOB_LIST` inside a `STARTUPINFOEX`, with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Exit is
  `QueryInformationJobObject(JobObjectBasicAccountingInformation).ActiveProcesses == 0`,
  polled at 250 ms. The root's exit code is recorded but is not the signal.
- **Linux:** `prctl(PR_SET_CHILD_SUBREAPER, 1)` at startup plus `setsid()` in
  `pre_exec`. Membership is the union of process-group membership and the
  PPID-descendant closure — the second set is what catches `setsid()` escapees
  such as `wineserver`.

`sysinfo` remains available for the UI's process panel only.

## Consequences

**Positive**

- Playtime is correct for bootstrapper-launched games, Proton titles, and
  emulator front-ends without special-casing any of them.
- `KILL_ON_JOB_CLOSE` means no game outlives a launcher crash on Windows.
- "Quit" terminates the whole tree, not just the parent.
- The tracker is indifferent to whether the target is an `.exe` or a ROM — this
  is what makes ADR-0004 cheap.

**Negative**

- Two substantially different implementations behind one trait; both need a
  Windows CI runner and real-process tests.
- Elevated games launched via `ShellExecuteEx(runas)` break job containment;
  tracking is degraded and the UI must say so.
- Linux tracking has a grace window because `wineserver` legitimately lingers;
  a stuck session is possible if a straggler never exits, so the window is
  configurable per game.

## Evidence required to close this ADR

The five-case acceptance suite in the blueprint, run against
`crates/core/tests/fixtures/testgame` on both operating systems, in CI.
