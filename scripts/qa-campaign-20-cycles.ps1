param()

$ErrorActionPreference = "Stop"

function Pass($msg) { Write-Host "[PASS] $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red }

Write-Host "==> Preflight outillage"
$tools = @("node.exe", "npm.cmd", "cargo.exe")
foreach ($t in $tools) {
  if (Get-Command $t -ErrorAction SilentlyContinue) {
    Pass "$t detecte"
  } else {
    Fail "$t introuvable"
    exit 1
  }
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

Write-Host "`n==> Preparation artefact campagne 20 cycles"
$qaDir = Join-Path (Get-Location) "artifacts\qa"
New-Item -ItemType Directory -Force -Path $qaDir | Out-Null

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$csvPath = Join-Path $qaDir "qa-campaign-20-cycles-$stamp.csv"

"cycle,target_app,expected_text,injected_text,widget_states_ok,injection_ok,notes" | Out-File -FilePath $csvPath -Encoding utf8
for ($i = 1; $i -le 20; $i++) {
  "$i,,,,,," | Out-File -FilePath $csvPath -Encoding utf8 -Append
}

Pass "Template CSV cree: $csvPath"

Write-Host "`n==> Protocole manuel"
Write-Host "1) Lance l'app: cd apps/desktop ; npm.cmd run tauri:dev"
Write-Host "2) Cibles recommandees: Notepad (8), Chrome (6), WordPad (6)"
Write-Host "3) Pour chaque cycle, renseigne expected_text/injected_text + statut widget/injection"
Write-Host "4) PASS cible: >= 19 succes injection sur 20 et 0 freeze widget"
Write-Host "5) En cas d'echec, noter contexte dans 'notes' + verifier logs backend"

$answer = Read-Host "Campagne executee et resultats saisis (y/n)"
if ($answer -eq "y") {
  Pass "Campagne QA 20 cycles terminee. Analysez le CSV: $csvPath"
  exit 0
}

Fail "Campagne interrompue. Reprendre avec le CSV deja cree: $csvPath"
exit 2
