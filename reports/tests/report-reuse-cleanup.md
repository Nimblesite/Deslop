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
