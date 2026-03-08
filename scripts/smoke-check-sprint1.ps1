param(
  [switch]$SkipBuild,
  [switch]$SkipRust,
  [switch]$NoPrompt,
  [switch]$PassManual
)

$ErrorActionPreference = "Stop"

function Write-Step($msg) {
  Write-Host "`n==> $msg" -ForegroundColor Cyan
}

function Mark-Pass($msg) {
  Write-Host "[PASS] $msg" -ForegroundColor Green
}

function Mark-Fail($msg) {
  Write-Host "[FAIL] $msg" -ForegroundColor Red
}

function Ask-YesNo($question) {
  if ($NoPrompt) {
    return $false
  }
  $reply = Read-Host "$question (y/n)"
  return $reply -match '^(y|Y|o|O)'
}

$repoRoot = "C:\Users\jerem\Desktop\WhisperPro"
$desktopDir = Join-Path $repoRoot "apps\desktop"

Write-Step "Preflight outillage"
$tools = @("node.exe", "npm.cmd", "cargo.exe")
foreach ($tool in $tools) {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
    Mark-Fail "Outil manquant: $tool"
    exit 1
  }
  Mark-Pass "$tool detecte"
}

Write-Step "Verifications build"
if (-not $SkipBuild) {
  Push-Location $desktopDir
  try {
    npm.cmd run build | Out-Host
    Mark-Pass "Build frontend OK"
  }
  catch {
    Mark-Fail "Build frontend KO"
    throw
  }
  finally {
    Pop-Location
  }
}
else {
  Write-Host "Build frontend ignore (--SkipBuild)"
}

if (-not $SkipRust) {
  Push-Location $repoRoot
  try {
    cargo.exe check --workspace | Out-Host
    Mark-Pass "Cargo workspace check OK"
  }
  catch {
    Mark-Fail "Cargo check KO"
    throw
  }
  finally {
    Pop-Location
  }
}
else {
  Write-Host "Cargo check ignore (--SkipRust)"
}

Write-Step "Verification manuelle guidee"
Write-Host "1) Lance l'app: cd apps/desktop ; npm.cmd run tauri:dev"
Write-Host "2) Dans Dashboard: Demarrer test micro, parler 5-10 sec, Arreter test micro"
Write-Host "3) Note le chemin WAV affiche"
Write-Host "4) Dans Settings: change langue+raccourci, sauvegarde, relance app et verifie persistance"
Write-Host "5) Verifie: Derniere erreur backend + Fichier log"

$manualOk = if ($PassManual) { $true } else { Ask-YesNo "Les verifications manuelles sont-elles OK" }

if ($manualOk) {
  Mark-Pass "Smoke test Sprint 1: PASS"
  exit 0
}

Mark-Fail "Smoke test Sprint 1: verifier les points manuels"
exit 2
