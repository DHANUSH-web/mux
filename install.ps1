param(
  [string]$Version = "latest",
  [string]$InstallDir = "$HOME\\.local\\bin",
  [string]$Repo = "DHANUSH-web/mux"
)

$ErrorActionPreference = "Stop"

function Write-Usage {
@"
Usage: install.ps1 [-Version <tag>] [-InstallDir <dir>] [-Repo <owner/repo>]

Examples:
  iwr https://raw.githubusercontent.com/$Repo/main/install.ps1 -useb | iex
  .\\install.ps1 -Version v0.1.0
  .\\install.ps1 -InstallDir "$HOME\\bin"
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

$binaryName = "mux.exe"
$assetName = "mux-windows-$arch.zip"

if ($Version -eq "latest") {
  $latestApi = "https://api.github.com/repos/$Repo/releases/latest"
  $latest = Invoke-RestMethod -Uri $latestApi
  if (-not $latest.tag_name) {
    throw "Failed to resolve latest release tag from $latestApi"
  }
  $tag = $latest.tag_name
} else {
  $tag = $Version
}

$baseUrl = "https://github.com/$Repo/releases/download/$tag"
$assetUrl = "$baseUrl/$assetName"
$checksumsUrl = "$baseUrl/checksums.txt"

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("mux-install-" + [Guid]::NewGuid().ToString("N"))
$null = New-Item -ItemType Directory -Path $tempRoot -Force
$archivePath = Join-Path $tempRoot $assetName
$extractDir = Join-Path $tempRoot "extract"

try {
  Write-Host "Installing mux $tag (windows/$arch)"
  Write-Host "Downloading $assetUrl"
  Invoke-WebRequest -Uri $assetUrl -OutFile $archivePath

  # Optional checksum verification if checksums.txt exists.
  try {
    $checksumsPath = Join-Path $tempRoot "checksums.txt"
    Invoke-WebRequest -Uri $checksumsUrl -OutFile $checksumsPath

    $line = Get-Content $checksumsPath | Where-Object { $_ -match ("\s" + [Regex]::Escape($assetName) + "$") } | Select-Object -First 1
    if ($line) {
      $expected = ($line -split "\s+")[0]
      $actual = (Get-FileHash -Path $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
      if ($expected.ToLowerInvariant() -ne $actual) {
        throw "Checksum verification failed for $assetName"
      }
      Write-Host "Checksum verified"
    }
  } catch {
    Write-Host "Checksum file unavailable or verification skipped"
  }

  Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

  $bin = Get-ChildItem -Path $extractDir -Recurse -File |
    Where-Object { $_.Name -in @("mux.exe", "mux") } |
    Select-Object -First 1

  if (-not $bin) {
    throw "Could not find mux binary inside downloaded archive"
  }

  $null = New-Item -ItemType Directory -Path $InstallDir -Force
  $targetPath = Join-Path $InstallDir $binaryName
  Copy-Item -Path $bin.FullName -Destination $targetPath -Force

  Write-Host "Installed mux to $targetPath"

  $pathEntries = $env:PATH -split ';'
  if (-not ($pathEntries | Where-Object { $_.TrimEnd('\\') -eq $InstallDir.TrimEnd('\\') })) {
    Write-Host "Note: $InstallDir is not currently in PATH for this shell."
    Write-Host "You can add it with:"
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','User') + ';$InstallDir', 'User')"
  }
}
finally {
  if (Test-Path $tempRoot) {
    Remove-Item -Path $tempRoot -Recurse -Force
  }
}
