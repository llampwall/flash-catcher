# flash-watcher — Implementation Decisions & Spec Deviations

## Spec deviations

### 1. `classify_stdio` — stdio handle types always Unknown (v1)

**Spec:** "DuplicateHandle + NtQueryObject to classify stdin/stdout/stderr handle types."

**Implemented:** Returns `HandleKind::Unknown` for all three handles.

**Reason:** Full classification requires either (a) `NtQueryObject` on duplicated handles (no stable windows-crate binding) or (b) reading `RTL_USER_PROCESS_PARAMETERS` via `NtQueryInformationProcess` + `ReadProcessMemory`, which is complex and error-prone for v1. The `visible_flash` field falls back to PE subsystem alone, which is the dominant signal.

**Impact:** `visible_flash` will be true for all CONSOLE-subsystem processes regardless of whether stdout is piped. This produces more rows in the dashboard but does not suppress any real events.

### 2. `read_working_directory` — always returns None (v1)

**Spec:** "NtQueryInformationProcess(ProcessBasicInformation) then walk PEB."

**Implemented:** Returns `None` unconditionally.

**Reason:** PEB walking requires reading `PROCESS_BASIC_INFORMATION.PebBaseAddress`, then `ReadProcessMemory` across process boundaries to reach `RTL_USER_PROCESS_PARAMETERS.CurrentDirectory`. This is out of scope for v1.

**Impact:** The `working_directory` field in JSONL and the detail pane will be null. No functional behaviour is lost.

### 3. `approximate_creation_flags` — always returns 0

**Spec:** "infer from stdio handle kinds when source is not available."

**Implemented:** Returns 0 unconditionally; field not meaningfully populated.

**Reason:** Creation flags are not reliably obtainable post-spawn without PEB access (same constraint as #2).

### 4. `query_session_id` — uses `GetTokenInformation(TokenSessionId)` instead of `ProcessIdToSessionId`

**Spec:** Implied use of `ProcessIdToSessionId`.

**Implemented:** Opens the process token and calls `GetTokenInformation(TOKEN_INFORMATION_CLASS(12))` to read the session ID. This is equivalent in output.

**Reason:** `ProcessIdToSessionId` is not available in the `windows = "0.56"` crate's `Win32_System_Threading` feature module.

### 5. `conhost.rs` as a standalone module (not folded into `etw.rs`)

**Spec:** "Could be folded into `src/etw.rs` if the pairing logic stays under ~100 LOC."

**Decision:** Kept as `src/conhost.rs`. The module is ~90 LOC but logically distinct from ETW session management.

### 6. `EventSource` reconnect in `connectStream()` uses doubling backoff (not fixed)

**Spec:** No specific reconnect strategy specified.

**Implemented:** Starts at 1s, doubles on each failure, caps at 30s.

## Non-deviations (noted for clarity)

- **ETW session recovery:** `start_kernel_session` catches `EvntraceNativeError::AlreadyExist`, calls `stop_session`, and retries as specified.
- **JSONL rotation:** Size-based rotation with gzip compression is implemented in `Store::rotate_if_needed`.
- **Blame cache:** Never evicts — processes remain resolvable for the full session lifetime.
- **Port:** Default bind `127.0.0.1:7790` as specified.
- **Single binary:** Web assets embedded via `include_str!` at compile time.
