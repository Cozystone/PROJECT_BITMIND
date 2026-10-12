# Information barrier (docs/OBSERVABILITY.md section 3): the agent crate may depend on
# life-interface only. Fails if life-world, life-trace, life-run or life-lab appear in its tree.
$tree = cargo tree -p life-agent -e normal 2>&1 | Out-String
if ($tree -match "life-world|life-trace|life-run|life-lab") {
    Write-Output "BARRIER VIOLATED:"; Write-Output $tree; exit 1
}
# content audit: no world names, site indices or action names inside the agent crate
$src = Get-Content -Raw crates/life-agent/src/lib.rs
foreach ($word in @("Structured", "Shuffled", "site", "grammar", "MoveLeft", "Touch", "Rest", "token_here", "emission")) {
    if ($src -match "\b$word\b") { Write-Output "AGENT CONTENT AUDIT: found '$word' in life-agent"; exit 1 }
}
Write-Output "barrier check: OK (life-agent depends on life-interface only; no world content)"
