$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Repo = 'Joaopedrobs7/kindle-for-linux'
$Headers = @{
  'User-Agent' = 'kindle-for-linux-install'
  'Accept'    = 'application/vnd.github+json'
}

function Install-KindlePackage {
  param([Parameter(Mandatory = $true)][string]$Path)
  $ext = [System.IO.Path]::GetExtension($Path).ToLowerInvariant()
  if ($ext -eq '.msi') {
    $proc = Start-Process -FilePath 'msiexec.exe' -ArgumentList "/i `"$Path`"" -Wait -PassThru -Verb RunAs
  } else {
    $proc = Start-Process -FilePath $Path -Wait -PassThru -Verb RunAs
  }
  if ($null -ne $proc.ExitCode -and $proc.ExitCode -ne 0) {
    throw "erro: instalador saiu com $($proc.ExitCode)"
  }
}

function Get-LatestInstaller {
  try {
    $release = Invoke-RestMethod -Headers $Headers -Uri "https://api.github.com/repos/$Repo/releases/latest"
  } catch {
    return $null
  }
  $assets = @($release.assets)
  $msi = $assets | Where-Object { $_.name -like '*.msi' } | Select-Object -First 1
  if ($msi) { return $msi }
  return $assets | Where-Object { $_.name -like '*setup*.exe' -or $_.name -like '*.exe' } | Select-Object -First 1
}

function Install-FromSource {
  foreach ($cmd in @('git', 'npm', 'cargo')) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
      throw "erro: sem instalador e sem $cmd"
    }
  }
  $work = Join-Path $env:TEMP 'kindle-for-linux-build'
  if (Test-Path $work) {
    Remove-Item -Recurse -Force $work
  }
  git clone --depth 1 "https://github.com/$Repo.git" $work
  Push-Location $work
  try {
    npm install
    npx tauri build --bundles msi
  } finally {
    Pop-Location
  }
  $built = Get-ChildItem -Path (Join-Path $work 'src-tauri\target\release\bundle\msi') -Filter '*.msi' -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $built) {
    throw 'erro: build sem .msi'
  }
  Install-KindlePackage $built.FullName
}

$asset = Get-LatestInstaller
if ($asset) {
  $name = [System.IO.Path]::GetFileName(([Uri]$asset.browser_download_url).AbsolutePath)
  $name = [Uri]::UnescapeDataString($name)
  $dest = Join-Path $env:TEMP $name
  Write-Host "==> Baixando kindle-para-linux..."
  Invoke-WebRequest -Headers $Headers -Uri $asset.browser_download_url -OutFile $dest
  Install-KindlePackage $dest
} else {
  Write-Host "==> Compilando..."
  Install-FromSource
}

Write-Host "==> Instalado! Abra pelo menu Iniciar."
