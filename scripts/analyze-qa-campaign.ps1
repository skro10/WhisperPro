param(
  [string]$CsvPath = ""
)

$ErrorActionPreference = "Stop"

function Pass($msg) { Write-Host "[PASS] $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red }

$qaDir = Join-Path (Get-Location) "artifacts\qa"
if (-not (Test-Path $qaDir)) {
  Fail "Dossier artifacts\\qa introuvable. Lance d'abord la campagne QA."
  exit 1
}

if ([string]::IsNullOrWhiteSpace($CsvPath)) {
  $latest = Get-ChildItem -Path $qaDir -Filter "qa-campaign-20-cycles-*.csv" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if (-not $latest) {
    Fail "Aucun CSV de campagne trouve dans artifacts\\qa."
    exit 1
  }
  $CsvPath = $latest.FullName
}

if (-not (Test-Path $CsvPath)) {
  Fail "CSV introuvable: $CsvPath"
  exit 1
}

Write-Host "==> Lecture campagne"
$rows = Import-Csv -Path $CsvPath
if (-not $rows -or $rows.Count -eq 0) {
  Fail "CSV vide: $CsvPath"
  exit 1
}

$completed = $rows | Where-Object { -not [string]::IsNullOrWhiteSpace($_.injection_ok) -or -not [string]::IsNullOrWhiteSpace($_.widget_states_ok) }
if (-not $completed -or $completed.Count -eq 0) {
  Fail "Aucune ligne completee dans le CSV."
  exit 2
}

$total = $completed.Count
$injOk = ($completed | Where-Object { $_.injection_ok -eq "yes" }).Count
$widgetOk = ($completed | Where-Object { $_.widget_states_ok -eq "yes" }).Count
$globalRate = [math]::Round(($injOk / [double]$total) * 100, 1)
$widgetRate = [math]::Round(($widgetOk / [double]$total) * 100, 1)
$pass = ($injOk -ge [math]::Ceiling($total * 0.95)) -and ($widgetOk -eq $total)

Write-Host "CSV: $CsvPath"
Write-Host "Cycles completes: $total"
Write-Host "Injection OK: $injOk/$total ($globalRate`%)"
Write-Host "Widget OK: $widgetOk/$total ($widgetRate`%)"

Write-Host "`n==> Detail par application"
$byApp = $completed | Group-Object target_app | Sort-Object Name
foreach ($group in $byApp) {
  $appTotal = $group.Count
  $appInjOk = ($group.Group | Where-Object { $_.injection_ok -eq "yes" }).Count
  $appWidgetOk = ($group.Group | Where-Object { $_.widget_states_ok -eq "yes" }).Count
  $appRate = [math]::Round(($appInjOk / [double]$appTotal) * 100, 1)
  Write-Host "- $($group.Name): injection $appInjOk/$appTotal ($appRate`%), widget $appWidgetOk/$appTotal"
}

$issues = $completed | Where-Object { $_.injection_ok -ne "yes" -or $_.widget_states_ok -ne "yes" }

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$reportPath = Join-Path $qaDir "qa-campaign-report-$stamp.md"

$lines = @()
$lines += "# QA Campaign Report"
$lines += ""
$lines += "Date: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")"
$lines += "Source CSV: $CsvPath"
$lines += ""
$lines += "## Global"
$lines += ""
$lines += "- Cycles completes: $total"
$lines += "- Injection OK: $injOk/$total ($globalRate`%)"
$lines += "- Widget OK: $widgetOk/$total ($widgetRate`%)"
$lines += "- Verdict: $(if ($pass) { "PASS" } else { "FAIL" })"
$lines += ""
$lines += "## Par application"
$lines += ""
foreach ($group in $byApp) {
  $appTotal = $group.Count
  $appInjOk = ($group.Group | Where-Object { $_.injection_ok -eq "yes" }).Count
  $appWidgetOk = ($group.Group | Where-Object { $_.widget_states_ok -eq "yes" }).Count
  $appRate = [math]::Round(($appInjOk / [double]$appTotal) * 100, 1)
  $lines += "- $($group.Name): injection $appInjOk/$appTotal ($appRate`%), widget $appWidgetOk/$appTotal"
}

$lines += ""
$lines += "## Incidents"
$lines += ""
if ($issues.Count -eq 0) {
  $lines += "- Aucun incident declare."
} else {
  foreach ($row in $issues) {
    $lines += "- Cycle $($row.cycle) [$($row.target_app)] injection_ok=$($row.injection_ok) widget_states_ok=$($row.widget_states_ok) notes=$($row.notes)"
  }
}

$lines += ""
$lines += "## Actions recommandees"
$lines += ""
if ($pass) {
  $lines += "- Conserver la configuration actuelle."
  $lines += "- Planifier une campagne de revalidation apres prochaine modification de l'injection."
} else {
  $lines += "- Prioriser les incidents repetes par application cible."
  $lines += "- Rejouer 20 cycles apres correction."
}

$lines | Out-File -FilePath $reportPath -Encoding utf8
Pass "Rapport genere: $reportPath"

if ($pass) {
  Pass "Verdict campagne: PASS"
  exit 0
}

Fail "Verdict campagne: FAIL"
exit 3
