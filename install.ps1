param(
  [string]$Version = "latest",
  [string]$InstallDir = "",
  [string]$Repo = "DHANUSH-web/mux",
  [switch]$System
)

$ErrorActionPreference = "Stop"

function Write-Usage {
@"
Usage: install.ps1 [-Version <tag>] [-InstallDir <dir>] [-Repo <owner/repo>] [-System]

Examples:
  iwr https://raw.githubusercontent.com/$Repo/main/install.ps1 -useb | iex
  .\install.ps1 -Version v0.1.0
  .\install.ps1 -System
"@
}

if ($Version -in @("-h", "--help", "help")) {
  Write-Usage
  exit 0
}

$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
  "AMD64" { "x86_64" }
  "ARM64" { "aarch64" }
  default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

$target = switch ($arch) {
  "x86_64" { "x86_64-pc-windows-msvc" }
  "aarch64" { "aarch64-pc-windows-msvc" }
  default { throw "Unsupported target architecture: $arch" }
}

if ([string]::IsNullOrWhiteSpace($InstallDir)) {
  if ($System) {
    $InstallDir = Join-Path $env:ProgramFiles "mux\bin"
  }
  else {
    $InstallDir = Join-Path $HOME ".mux\bin"
  }
}

$binaryAsset = "mux-$target.exe"
$archiveAsset = "mux-$target.zip"

if ($Version -eq "latest") {
  $latestApi = "https://api.github.com/repos/$Repo/releases/latest"
  $latest = Invoke-RestMethod -Uri $latestApi
  if (-not $latest.tag_name) {
    throw "Failed to resolve latest release tag from $latestApi"
  }
  $tag = $latest.tag_name
}
else {
  $tag = $Version
}

$baseUrl = "https://github.com/$Repo/releases/download/$tag"
$binaryUrl = "$baseUrl/$binaryAsset"
$archiveUrl = "$baseUrl/$archiveAsset"
$checksumsUrl = "$baseUrl/SHA256SUMS"

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("mux-install-" + [Guid]::NewGuid().ToString("N"))
$null = New-Item -ItemType Directory -Path $tempRoot -Force
$binaryPath = Join-Path $tempRoot $binaryAsset
$archivePath = Join-Path $tempRoot $archiveAsset
$extractDir = Join-Path $tempRoot "extract"

try {
  Write-Host "Installing mux $tag ($target)"

  $sourceName = $null
  $installSource = $null
  $checksumPath = $null

  try {
    Write-Host "Downloading $binaryUrl"
    Invoke-WebRequest -Uri $binaryUrl -OutFile $binaryPath
    $sourceName = $binaryAsset
    $installSource = $binaryPath
    $checksumPath = $binaryPath
  }
  catch {
    Write-Host "Direct binary not found, trying archive $archiveAsset"
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath
    Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force
    $candidate = Join-Path $extractDir $binaryAsset
    if (-not (Test-Path $candidate)) {
      throw "Could not find $binaryAsset inside downloaded archive"
    }
    $sourceName = $archiveAsset
    $installSource = $candidate
    $checksumPath = $archivePath
  }

  # Optional checksum verification if SHA256SUMS exists.
  try {
    $checksumsPath = Join-Path $tempRoot "SHA256SUMS"
    Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath

    $line = Get-Content $checksumsPath |
      Where-Object { $_ -match ("\s" + [Regex]::Escape($sourceName) + "$") } |
      Select-Object -First 1

    if ($line) {
      $expected = ($line -split "\s+")[0].ToLowerInvariant()
      $actual = (Get-FileHash -Path $checksumPath -Algorithm SHA256).Hash.ToLowerInvariant()
      if ($expected -ne $actual) {
        throw "Checksum verification failed for $sourceName"
      }
      Write-Host "Checksum verified"
    }
  }
  catch {
    if ($_.Exception.Message -like "Checksum verification failed*") {
      throw
    }
    Write-Host "Checksum file unavailable or verification skipped"
  }

  $null = New-Item -ItemType Directory -Path $InstallDir -Force
  $targetPath = Join-Path $InstallDir "mux.exe"
  Copy-Item -Path $installSource -Destination $targetPath -Force

  Write-Host "Installed mux to $targetPath"

  if ($System) {
    $current = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if ($null -eq $current) { $current = "" }
    if (-not (($current -split ';') | Where-Object { $_.TrimEnd('\\') -eq $InstallDir.TrimEnd('\\') })) {
      $newPath = if ([string]::IsNullOrWhiteSpace($current)) { $InstallDir } else { $current.TrimEnd(';') + ";$InstallDir" }
      [Environment]::SetEnvironmentVariable("Path", $newPath, "Machine")
      Write-Host "Added $InstallDir to machine PATH. Restart terminal to use mux."
    }
  }
  else {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($null -eq $current) { $current = "" }
    if (-not (($current -split ';') | Where-Object { $_.TrimEnd('\\') -eq $InstallDir.TrimEnd('\\') })) {
      $newPath = if ([string]::IsNullOrWhiteSpace($current)) { $InstallDir } else { $current.TrimEnd(';') + ";$InstallDir" }
      [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
      Write-Host "Added $InstallDir to user PATH. Restart terminal to use mux."
    }
  }
}
finally {
  if (Test-Path $tempRoot) {
    Remove-Item -Path $tempRoot -Recurse -Force
  }
}
