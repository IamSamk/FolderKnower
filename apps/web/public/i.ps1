# FolderKnower CLI Installer for Windows
$ErrorActionPreference = "Stop"

$Repo = "IamSamk/FolderKnower"
$InstallDir = "$env:LOCALAPPDATA\FolderKnower\bin"

Write-Host "Installing FolderKnower CLI for Windows..." -ForegroundColor Cyan

# Fetch latest release tag
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Tag = $Release.tag_name
} catch {
    $Tag = "v0.7.0"
}

$Target = "x86_64-pc-windows-msvc"
$Url = "https://github.com/IamSamk/FolderKnower/releases/download/$Tag/fknow-cli-$Tag-$Target.zip"

$TempZip = [System.IO.Path]::GetTempFileName() + ".zip"
$TempExtract = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())

Write-Host "Downloading $Url..."
Invoke-WebRequest -Uri $Url -OutFile $TempZip

Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

$Exe = Get-ChildItem -Path $TempExtract -Recurse -Filter "fknow.exe" | Select-Object -First 1
if ($Exe) {
    Copy-Item -Path $Exe.FullName -Destination "$InstallDir\fknow.exe" -Force
}

# Cleanup
Remove-Item -Path $TempZip -Force -ErrorAction SilentlyContinue
Remove-Item -Path $TempExtract -Recurse -Force -ErrorAction SilentlyContinue

# Add to user PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "Added $InstallDir to user PATH." -ForegroundColor Green
}

Write-Host ""
Write-Host "Successfully installed FolderKnower CLI to $InstallDir\fknow.exe" -ForegroundColor Green
Write-Host "Open a new terminal and run: fknow --help" -ForegroundColor Yellow
