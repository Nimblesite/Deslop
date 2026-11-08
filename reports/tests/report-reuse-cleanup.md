# Report reuse build cleanup

Removed the disposable debug incremental-compilation directory after debug compilation completed; test executables and release build output are retained.

## Before

```text
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s5   926Gi   872Gi    16Gi    99%    6.3M  169M    4%   /System/Volumes/Data
```

## After

```text
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s5   926Gi   871Gi    17Gi    99%    6.3M  181M    3%   /System/Volumes/Data
```

## Debug diagnostic cleanup

The redundant debug run ended with exit 130 after an already-corrected test-harness failure; no debug-suite pass is claimed. Its owned cargo/test/CLI process tree was verified before interruption, and unused debug build artifacts were removed while the release gate continued.

Before:

```text
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s5   926Gi   874Gi    15Gi    99%    6.3M  157M    4%   /System/Volumes/Data
```

After:

```text
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s5   926Gi   873Gi    16Gi    99%    6.3M  168M    4%   /System/Volumes/Data
```

## Instrumented workspace cleanup

Coverage collection finished with the recorded baseline failures before threshold calculation. Cleaned this workspace's instrumented artifacts and profiles while retaining third-party dependency caches and the release binaries.

Command: `cargo llvm-cov clean --workspace`

Exit: `0`

```text
```

Disk after cleanup:

```text
Filesystem      Size    Used   Avail Capacity iused ifree %iused  Mounted on
/dev/disk3s5   926Gi   874Gi    18Gi    98%    6.3M  187M    3%   /System/Volumes/Data
```
