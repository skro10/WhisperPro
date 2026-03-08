param()

$ErrorActionPreference = "Stop"

function Pass($msg) { Write-Host "[PASS] $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red }

Write-Host "==> Preflight outillage"
$tools = @("node.exe", "npm.cmd", "cargo.exe")
foreach ($t in $tools) {
  if (Get-Command $t -ErrorAction SilentlyContinue) { Pass "$t detecte" } else { Fail "$t introuvable"; exit 1 }
}

Write-Host "`n==> Verifications build"
Push-Location "apps/desktop"
try {
  npm.cmd run build | Out-Host
  Pass "Build frontend OK"
} finally {
  Pop-Location
}

Push-Location "apps/desktop/src-tauri"
try {
  cargo.exe check --workspace | Out-Host
  Pass "Cargo workspace check OK"
} finally {
  Pop-Location
}

Write-Host "`n==> Verification manuelle guidee (Sprint 2)"
Write-Host "1) Lance l'app: cd apps/desktop ; npm.cmd run tauri:dev"
Write-Host "2) Verifie que le widget apparait et peut etre deplace"
Write-Host "3) Hotkey: appui 1 = listening ; appui 2 = transcribing puis done"
Write-Host "4) Injection: 5 cycles dans Notepad, texte colle correct des le 1er cycle"
Write-Host "5) Settings: desactive/active widget + sauvegarde, verifier fermeture/reapparition"
Write-Host "6) Auto-hide: done => masque en ~1.6s, nouveau cycle => reaffichage"
Write-Host "7) Relance app et verifie persistance options widget"
Write-Host "8) Clique 'Reset session' et verifie retour idle + nouveau cycle possible"

$answer = Read-Host "Les verifications manuelles sont-elles OK (y/n)"
if ($answer -eq "y") {
  Pass "Smoke test Sprint 2: PASS"
  exit 0
}

Fail "Smoke test Sprint 2: FAIL"
exit 2
