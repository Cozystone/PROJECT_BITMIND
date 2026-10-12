# Freeze guard for the first experiment (docs/APPENDIX-FIRST-EXPERIMENT.md A8, A10).
# Formal seeds may only be run when every check passes.
param([string]$Tag = "bitmind-life-first-v0.1")
$fail = 0
function Fail($msg) { Write-Output "FREEZE GUARD: $msg"; $script:fail = 1 }

$tagCommit = (git rev-list -n 1 $Tag 2>$null)
$head = (git rev-parse HEAD)
if (-not $tagCommit) { Fail "tag $Tag does not exist" }
elseif ($tagCommit -ne $head) { Fail "HEAD $head is not the tagged commit $tagCommit" }

$dirty = git status --porcelain | Where-Object { $_ -notmatch '^\?\? results/' -and $_ -notmatch '^\?\? papers/' }
if ($dirty) { Fail "working tree has changes:`n$($dirty -join "`n")" }

$cfg = "experiments/formal-config.json"
$hash = (Get-FileHash $cfg -Algorithm SHA256).Hash.ToLower()
$recorded = (Select-String -Path experiments/FREEZE.md -Pattern 'formal-config.json sha256: ([0-9a-f]{64})').Matches.Groups[1].Value
if (-not $recorded) { Fail "no recorded config hash in experiments/FREEZE.md" }
elseif ($recorded -ne $hash) { Fail "formal-config.json hash $hash differs from recorded $recorded" }

$build = cargo build --release 2>&1 | Out-String
if ($build -match "Compiling") { Fail "binaries were not built from the frozen sources (cargo recompiled)" }
if ($build -notmatch "Finished") { Fail "cargo build failed" }

$barrier = powershell -ExecutionPolicy Bypass -File scripts/check_barrier.ps1 2>&1 | Out-String
if ($barrier -notmatch "barrier check: OK") { Fail "information barrier check failed" }

if ($fail -eq 0) { Write-Output "FREEZE GUARD: OK (tag $Tag at $head, config $hash)"; exit 0 } else { exit 1 }
