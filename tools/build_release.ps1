# RBD OFIS - Single-Command Multiplatform Signed Release Builder
# Builds Rust core (Windows static-CRT + Android NDK arm64-v8a/x86_64),
# signed Android Release APK + AAB, Windows Desktop Release, and syncs with Genel Mudurluk (HQ).

param(
    [switch]$BumpVersion,
    [switch]$SyncHq = $true
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

if (-not $env:JAVA_HOME -and (Test-Path "C:\Users\90536\.jdks\jbr-21.0.11")) {
    $env:JAVA_HOME = "C:\Users\90536\.jdks\jbr-21.0.11"
}

$GradleFile = Join-Path $RepoRoot "apps\android\app\build.gradle.kts"
$GradleText = Get-Content $GradleFile -Raw

if ($GradleText -match 'versionCode\s*=\s*(\d+)') {
    $VersionCode = [int]$Matches[1]
} else {
    $VersionCode = 100
}

if ($GradleText -match 'versionName\s*=\s*"([^"]+)"') {
    $VersionName = $Matches[1]
} else {
    $VersionName = "1.0.0"
}

if ($BumpVersion) {
    $VersionCode += 1
    $Parts = $VersionName.Split('.')
    $Parts[-1] = ([int]$Parts[-1] + 1).ToString()
    $VersionName = $Parts -join '.'
    $GradleText = $GradleText -replace 'versionCode\s*=\s*\d+', "versionCode = $VersionCode"
    $GradleText = $GradleText -replace 'versionName\s*=\s*"[^"]+"', "versionName = `"$VersionName`""
    Set-Content -Path $GradleFile -Value $GradleText -Encoding UTF8
    Write-Host "[1/5] Bumped Android version to $VersionName ($VersionCode)" -ForegroundColor Cyan
} else {
    Write-Host "[1/5] Using Android version $VersionName ($VersionCode)" -ForegroundColor Cyan
}

Write-Host "[2/5] Building Rust FFI DLL with Static CRT (Windows x64)..." -ForegroundColor Cyan
Push-Location (Join-Path $RepoRoot "core")
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release -p rbd-ffi --target x86_64-pc-windows-msvc
Copy-Item "target\x86_64-pc-windows-msvc\release\rbd_ffi.dll" -Destination "target\release\rbd_ffi.dll" -Force
Copy-Item "target\x86_64-pc-windows-msvc\release\rbd_ffi.dll" -Destination "..\apps\windows\RbdOfis.Interop\runtimes\win-x64\native\rbd_ffi.dll" -Force
Pop-Location

Write-Host "[3/5] Building Windows Desktop Application (.NET 9 Release)..." -ForegroundColor Cyan
Push-Location (Join-Path $RepoRoot "apps\windows")
dotnet build RbdOfis.sln -c Release
Pop-Location

Write-Host "[4/5] Building Signed Android Release APK & AAB..." -ForegroundColor Cyan
Push-Location (Join-Path $RepoRoot "apps\android")
.\gradlew.bat assembleRelease bundleRelease --no-daemon
Pop-Location

$ApkPath = Join-Path $RepoRoot "apps\android\app\build\outputs\apk\release\app-release.apk"
$AabPath = Join-Path $RepoRoot "apps\android\app\build\outputs\bundle\release\app-release.aab"
$DistAndroid = Join-Path $RepoRoot "dist\android"
New-Item -ItemType Directory -Force -Path $DistAndroid | Out-Null
Copy-Item $ApkPath -Destination (Join-Path $DistAndroid "rbd-ofis-v$VersionName-release.apk") -Force
Copy-Item $AabPath -Destination (Join-Path $DistAndroid "rbd-ofis-v$VersionName-release.aab") -Force

$HqScript = "C:\Users\90536\Desktop\Projeler\Projeler Sitesi\genel-mudurluk\hq.mjs"
if ($SyncHq -and (Test-Path $HqScript)) {
    Write-Host "[5/5] Syncing signed release with Genel Mudurluk (HQ)..." -ForegroundColor Cyan
    node $HqScript gonder --slug rbd-ofis --surum-ad $VersionName --surum-kod $VersionCode --apk $ApkPath
}

Write-Host "Release build complete! APK: $ApkPath | AAB: $AabPath" -ForegroundColor Green
