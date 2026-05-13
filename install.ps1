$ErrorActionPreference = 'Stop'

$repo = 'Azmekk/vimg'
$installDir = Join-Path $env:LOCALAPPDATA 'vimg'

$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
$asset = $release.assets | Where-Object { $_.name -like '*windows-x86_64*.zip' } | Select-Object -First 1
if (-not $asset) { throw 'No Windows asset found in latest release.' }

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
$zip = Join-Path $installDir 'vimg.zip'
Invoke-WebRequest $asset.browser_download_url -OutFile $zip
Expand-Archive -Force $zip $installDir
Remove-Item $zip

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$installDir", 'User')
}

Write-Host "vimg installed to $installDir"
Write-Host 'Restart your terminal, then optionally run: vimg --enable-context-menu'
