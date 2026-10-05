# [CORPUS-MEASURE] [CORPUS-CEILINGS] Reports a running process's peak resident
# set size and, once it exits, its CPU time — so a corpus scan on Windows is
# measured exactly as `/usr/bin/time` measures one on macOS and Linux.
#
# POSIX has `/usr/bin/time`. Windows has no equivalent tool, but it keeps the
# same numbers. `PeakWorkingSet64` is a counter the kernel raises as the process
# grows and never lowers, so reading it periodically yields the TRUE peak, not a
# sample of current usage — the distinction matters, because a sampled lower
# bound on a ceiling assertion produces false passes. Measured on
# flutter/flutter: sampling `WorkingSet64` read 3,629 MB where this counter read
# 4,818 MB. User and kernel processor time are totals the kernel keeps for the
# process after it exits, read through the handle held for the whole run.
#
# The pid is the only input, so nothing here has to quote a path. The harness
# spawns the scan itself and passes the id it got back.
#
# Output is GNU `/usr/bin/time -v` lines on stdout, so the same parser reads
# every platform. A figure that was not measured is not printed: a zero would
# parse as a real number — clearing every memory ceiling in the corpus at once,
# or reading as a scan that cost no CPU — so the harness must see an absent line
# and error instead.

param(
  [Parameter(Mandatory = $true)][int] $ProcessId,
  [int] $PollMilliseconds = 200
)

$ErrorActionPreference = 'Stop'

$peakBytes = 0
try {
  $process = Get-Process -Id $ProcessId -ErrorAction Stop
  # Opening the handle now keeps it for the process's whole life, which is what
  # lets its processor times be read after it has exited.
  $null = $process.Handle
} catch {
  $process = $null
}

while ($null -ne $process) {
  try {
    $process.Refresh()
    $current = $process.PeakWorkingSet64
    if ($current -gt $peakBytes) { $peakBytes = $current }
    if ($process.HasExited) { break }
  } catch {
    # The process ended between the refresh and the read. Whatever peak was
    # already observed stands; the counter only ever rose.
    break
  }
  Start-Sleep -Milliseconds $PollMilliseconds
}

if ($peakBytes -gt 0) {
  $peakKbytes = [math]::Floor($peakBytes / 1024)
  Write-Output "Maximum resident set size (kbytes): $peakKbytes"
}

if ($null -ne $process) {
  $process.WaitForExit()
  $invariant = [System.Globalization.CultureInfo]::InvariantCulture
  $user = $process.UserProcessorTime.TotalSeconds.ToString($invariant)
  $system = $process.PrivilegedProcessorTime.TotalSeconds.ToString($invariant)
  Write-Output "User time (seconds): $user"
  Write-Output "System time (seconds): $system"
}
