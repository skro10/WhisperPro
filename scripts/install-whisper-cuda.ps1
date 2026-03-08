param(
  [string]$RepoDir = "$env:TEMP\whisper.cpp-cuda",
  [string]$InstallDir = "$env:LOCALAPPDATA\WhisperPro\bin"
)

$ErrorActionPreference = "Stop"

function Find-CudaToolkitRoot {
  $base = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA"
  if (-not (Test-Path $base)) { return $null }

  $candidates = Get-ChildItem $base -Directory | Sort-Object Name -Descending
  foreach ($candidate in $candidates) {
    $nvcc = Join-Path $candidate.FullName "bin\nvcc.exe"
    if (Test-Path $nvcc) {
      return $candidate.FullName
    }
  }
  return $null
}

Write-Host "== WhisperPro CUDA installer ==" -ForegroundColor Cyan

$cudaRoot = Find-CudaToolkitRoot
if (-not $cudaRoot) {
  Write-Error @"
CUDA Toolkit introuvable.
Installe CUDA Toolkit (pas seulement le driver), puis relance ce script.
Chemin attendu: C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\vXX.X
"@
}

Write-Host "CUDA detecte: $cudaRoot"
$env:CUDAToolkit_ROOT = $cudaRoot

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
  Write-Error "git introuvable dans le PATH."
}
if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
  Write-Error "cmake introuvable dans le PATH."
}

if (-not (Test-Path $RepoDir)) {
  Write-Host "Clonage whisper.cpp..."
  git clone https://github.com/ggml-org/whisper.cpp.git $RepoDir
} else {
  Write-Host "Mise a jour whisper.cpp..."
  git -C $RepoDir pull --ff-only
}

$buildDir = Join-Path $RepoDir "build-cuda"
Write-Host "Configuration CMake..."
cmake -S $RepoDir -B $buildDir -DGGML_CUDA=ON -DBUILD_SHARED_LIBS=ON

Write-Host "Compilation whisper-cli (Release)..."
cmake --build $buildDir --config Release --target whisper-cli

if (-not (Test-Path $InstallDir)) {
  New-Item -ItemType Directory -Path $InstallDir | Out-Null
}

$binCandidates = @(
  (Join-Path $buildDir "bin\Release"),
  (Join-Path $buildDir "bin"),
  (Join-Path $buildDir "src\Release"),
  (Join-Path $buildDir "src")
)

$sourceBin = $null
foreach ($candidate in $binCandidates) {
  if (Test-Path (Join-Path $candidate "whisper-cli.exe")) {
    $sourceBin = $candidate
    break
  }
}

if (-not $sourceBin) {
  Write-Error "whisper-cli.exe compile mais introuvable dans les dossiers attendus."
}

Write-Host "Copie des binaires dans $InstallDir ..."
$filesToCopy = @("whisper-cli.exe", "whisper.dll")
foreach ($name in $filesToCopy) {
  $source = Join-Path $sourceBin $name
  if (Test-Path $source) {
    Copy-Item $source $InstallDir -Force
  }
}

Get-ChildItem $sourceBin -Filter "ggml*.dll" -File | ForEach-Object {
  Copy-Item $_.FullName $InstallDir -Force
}

Write-Host ""
Write-Host "Installation terminee." -ForegroundColor Green
Write-Host "Relance WhisperPro puis ouvre Options > Mode de calcul."
Write-Host "Si tout est bon, l'option GPU ne sera plus grisee."
