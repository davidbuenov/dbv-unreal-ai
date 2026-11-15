# =============================================================================
# DBV Unreal AI — Compila el plugin C++ para cada versión del motor (binarios para quien no tiene Visual Studio)
# Copyright (c) 2026 David Bueno Vallejo and David Bueno Carmona
# Licensed under the MIT License. See LICENSE for details.
# Built with dbv-specs-ops · https://github.com/davidbuenov/dbv-specs-ops
# =============================================================================
#
# @spec REQ-121
#
# Para cada motor del Launcher indicado, `RunUAT BuildPlugin` compila `unreal-plugin/DBVUnrealAICpp` y deja en
# `unreal-plugin-bin/<mayor.menor>/Win64/` solo lo que hace falta para cargarlo sin compilar: las DLL y
# `UnrealEditor.modules` (con su BuildId, que «Preparar» compara con el CompatibleChangelist del motor del proyecto).
# Sin `.pdb` (60 MB cada uno). Se ejecuta en el /ship de cada versión, en la máquina de desarrollo.
#
# Uso: powershell -File scripts/release/build-cpp-plugin.ps1 [-Engines "D:\Program Files\Epic Games\UE_5.8"]

param(
    [string[]]$Engines = @("D:\Program Files\Epic Games\UE_5.8")
)

$ErrorActionPreference = "Stop"
$Repo = Resolve-Path (Join-Path $PSScriptRoot "..\..")
# BuildPlugin compila en un proyecto anfitrión que solo tiene este plugin, y no encuentra `DBVUnrealAI` (el toolset de
# Python, una dependencia de CONTENIDO y no de compilación). Se compila una copia sin esa dependencia: las DLL son las
# mismas, porque ningún módulo C++ enlaza con DBVUnrealAI.
$Staging = Join-Path $env:TEMP "dbv-cpp-plugin-src"
if (Test-Path $Staging) { Remove-Item -Recurse -Force $Staging }
New-Item -ItemType Directory -Force $Staging | Out-Null
Copy-Item -Recurse (Join-Path $Repo "unreal-plugin\DBVUnrealAICpp\*") $Staging -Exclude "Binaries", "Intermediate"
$Plugin = Join-Path $Staging "DBVUnrealAICpp.uplugin"
$Descriptor = Get-Content $Plugin -Raw | ConvertFrom-Json
$Descriptor.Plugins = @($Descriptor.Plugins | Where-Object { $_.Name -ne "DBVUnrealAI" })
$Descriptor | ConvertTo-Json -Depth 10 | Set-Content -Encoding UTF8 $Plugin

foreach ($Engine in $Engines) {
    $Version = Get-Content (Join-Path $Engine "Engine\Build\Build.version") -Raw | ConvertFrom-Json
    $Minor = "$($Version.MajorVersion).$($Version.MinorVersion)"
    $Package = Join-Path $env:TEMP "dbv-cpp-plugin-$Minor"
    if (Test-Path $Package) { Remove-Item -Recurse -Force $Package }
    Write-Host "== Unreal $Minor ($Engine) -> $Package"
    & (Join-Path $Engine "Engine\Build\BatchFiles\RunUAT.bat") BuildPlugin "-Plugin=$Plugin" "-Package=$Package" -TargetPlatforms=Win64 -Rocket
    if ($LASTEXITCODE -ne 0) { throw "BuildPlugin failed for Unreal $Minor (exit $LASTEXITCODE)" }

    $Out = Join-Path $Repo "unreal-plugin-bin\$Minor\Win64"
    if (Test-Path $Out) { Remove-Item -Recurse -Force $Out }
    New-Item -ItemType Directory -Force $Out | Out-Null
    Copy-Item (Join-Path $Package "Binaries\Win64\*.dll") $Out
    Copy-Item (Join-Path $Package "Binaries\Win64\UnrealEditor.modules") $Out
    $Modules = Get-Content (Join-Path $Out "UnrealEditor.modules") -Raw | ConvertFrom-Json
    if ("$($Modules.BuildId)" -ne "$($Version.CompatibleChangelist)") {
        throw "BuildId $($Modules.BuildId) != CompatibleChangelist $($Version.CompatibleChangelist) for Unreal $Minor"
    }
    $Size = (Get-ChildItem $Out | Measure-Object -Sum Length).Sum
    Write-Host ("   OK: BuildId {0}, {1:N0} bytes in {2}" -f $Modules.BuildId, $Size, $Out)
}
