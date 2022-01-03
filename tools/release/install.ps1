$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($args.Length -eq 1) {
  $Version = $args.Get(0)
}

$3emInstall = $env:3EM_INSTALL
$BinDir = if ($3emInstall) {
  "$3emInstall\bin"
}
else {
  "$Home\.3em\bin"
}

$3emZip = "$BinDir\3em.zip"
$3emExe = "$BinDir\3em.exe"
$Target = 'x86_64-pc-windows-msvc'

# GitHub requires TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$3emUrl = if (!$Version) {
  "https://github.com/three-em/3em/releases/latest/download/three_em-${Target}.zip"
}
else {
  "https://github.com/three-em/3em/releases/download/${Version}/three_em-${Target}.zip"
}

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

Invoke-WebRequest $3emUrl -OutFile $3emZip -UseBasicParsing

if (Get-Command Expand-Archive -ErrorAction SilentlyContinue) {
  Expand-Archive $3emZip -Destination $BinDir -Force
}
else {
  if (Test-Path $3emExe) {
    Remove-Item $3emExe
  }
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  [IO.Compression.ZipFile]::ExtractToDirectory($3emZip, $BinDir)
}

Remove-Item $3emZip

$User = [EnvironmentVariableTarget]::User
$Path = [Environment]::GetEnvironmentVariable('Path', $User)
if (!(";$Path;".ToLower() -like "*;$BinDir;*".ToLower())) {
  [Environment]::SetEnvironmentVariable('Path', "$Path;$BinDir", $User)
  $Env:Path += ";$BinDir"
}

Write-Output "3em was installed successfully to $3emExe"
Write-Output "Head over to https://3em.dev to get started!"
