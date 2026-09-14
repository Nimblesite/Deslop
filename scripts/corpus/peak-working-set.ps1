# [CORPUS-CEILINGS] Reports a running process's peak resident set size, so the
# corpus gate can assert `ceilings.max_peak_rss_mb` on Windows.
#
# POSIX has `/usr/bin/time -v`. Windows has no equivalent tool, but it does
# maintain the number: `PeakWorkingSet64` is a counter the kernel raises as the
# process grows and never lowers. Reading it periodically therefore yields the
# TRUE peak, not a sample of current usage — the distinction matters, because a
# sampled lower bound on a ceiling assertion produces false passes. Measured on
# flutter/flutter: sampling `WorkingSet64` read 3,629 MB where this counter read
# 4,818 MB.
#
# The pid is the only input, so nothing here has to quote a path. The harness
# spawns the scan itself and passes the id it got back.
#
# Output is one line on stdout in the GNU `/usr/bin/time -v` form, so the same
# parser reads both platforms. Nothing is printed when no measurement was
# taken: a zero would parse as a real number and clear every memory ceiling in
# the corpus at once, so the harness must see an absent line and error instead.

param(
  [Parameter(Mandatory = $true)][int] $ProcessId,
  [int] $PollMilliseconds = 200
)

$ErrorActionPreference = 'Stop'

$peakBytes = 0
try {
  $process = Get-Process -Id $ProcessId -ErrorAction Stop
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
