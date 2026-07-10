$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repoRoot "hosts\unity-host\Plugins\plugin-manifest.json"
$packagePath = Join-Path $repoRoot "hosts\unity-host\package.json"
$buildScriptPath = Join-Path $repoRoot "build.ps1"

if (-not (Test-Path -LiteralPath $manifestPath)) {
    throw "Plugin manifest not found: $manifestPath"
}

$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$unityRoot = Join-Path $repoRoot "hosts\unity-host"
$nativeScriptsPath = Join-Path $unityRoot "Scripts\Native"
$protocolScriptsPath = Join-Path $unityRoot "Scripts\Protocol"
$protocolAsmdefPath = Join-Path $protocolScriptsPath "KimoTech.LichoraHost.Protocol.asmdef"
if (-not (Test-Path -LiteralPath $protocolAsmdefPath -PathType Leaf)) {
    throw "Protocol assembly definition not found: $protocolAsmdefPath"
}
$protocolAsmdef = Get-Content -Raw -LiteralPath $protocolAsmdefPath | ConvertFrom-Json
if ($protocolAsmdef.name -ne "KimoTech.LichoraHost.Protocol" -or $protocolAsmdef.rootNamespace -ne "KimoTech.LichoraHost") {
    throw "Protocol assembly definition has unexpected name or root namespace."
}
if ($protocolAsmdef.autoReferenced -ne $false -or $protocolAsmdef.noEngineReferences -ne $true -or $protocolAsmdef.references.Count -ne 0) {
    throw "Protocol assembly must be explicit, Unity-free and dependency-free."
}
$runtimeAsmdefPath = Join-Path $unityRoot "Scripts\KimoTech.LichoraHost.asmdef"
$runtimeAsmdef = Get-Content -Raw -LiteralPath $runtimeAsmdefPath | ConvertFrom-Json
$nativeAsmdefPath = Join-Path $nativeScriptsPath "KimoTech.LichoraHost.Native.asmdef"
$nativeAsmdef = Get-Content -Raw -LiteralPath $nativeAsmdefPath | ConvertFrom-Json
$collectionsGuid = "3dcbb65762ae01440b7e7f4a08bf32bd"
foreach ($assembly in @($runtimeAsmdef, $nativeAsmdef)) {
    if ($assembly.references -notcontains "GUID:$collectionsGuid") {
        throw "Assembly '$($assembly.name)' does not reference the confirmed Unity.Collections assembly GUID."
    }
}
if ($nativeAsmdef.references.Count -ne 1) {
    throw "Native assembly must only reference the confirmed Unity.Collections assembly."
}
if ($runtimeAsmdef.allowUnsafeCode -ne $false) {
    throw "Runtime assembly must not allow unsafe code."
}
$protocolGuid = (Get-Content -Raw -LiteralPath "$protocolAsmdefPath.meta") | Select-String -Pattern 'guid:\s*(\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $protocolGuid -or $runtimeAsmdef.references -notcontains "GUID:$protocolGuid") {
    throw "Runtime assembly does not explicitly reference Protocol assembly."
}
if ($nativeAsmdef.references -contains "GUID:$protocolGuid") {
    throw "Native assembly must not reference Protocol assembly."
}
$modelScriptsPath = Join-Path $unityRoot "Scripts\Model"
$modelAsmdefPath = Join-Path $modelScriptsPath "KimoTech.LichoraHost.Model.asmdef"
if (-not (Test-Path -LiteralPath $modelAsmdefPath -PathType Leaf)) {
    throw "Model assembly definition not found: $modelAsmdefPath"
}
$modelAsmdef = Get-Content -Raw -LiteralPath $modelAsmdefPath | ConvertFrom-Json
if ($modelAsmdef.name -ne "KimoTech.LichoraHost.Model" -or $modelAsmdef.rootNamespace -ne "KimoTech.LichoraHost") {
    throw "Model assembly definition has unexpected name or root namespace."
}
if ($modelAsmdef.autoReferenced -ne $false -or $modelAsmdef.noEngineReferences -ne $true -or $modelAsmdef.references.Count -ne 0) {
    throw "Model assembly must be explicit, Unity-free and dependency-free."
}
$modelGuid = (Get-Content -Raw -LiteralPath "$modelAsmdefPath.meta") | Select-String -Pattern 'guid:\s*(\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $modelGuid -or $runtimeAsmdef.references -notcontains "GUID:$modelGuid") {
    throw "Runtime assembly does not explicitly reference Model assembly."
}
$modelFiles = @("InputOwner.cs", "InputRegionShape.cs")
foreach ($modelFile in $modelFiles) {
    $modelPath = Join-Path $modelScriptsPath $modelFile
    if (-not (Test-Path -LiteralPath $modelPath -PathType Leaf)) {
        throw "Model file is missing: $modelFile"
    }
    $modelSource = Get-Content -Raw -LiteralPath $modelPath
    if ($modelSource -match 'UnityEngine|DllImport\s*\(') {
        throw "Model file must not reference UnityEngine or DllImport: $modelFile"
    }
}
$invalidRuntimeReferences = @($runtimeAsmdef.references | Where-Object { $_ -notmatch '^GUID:[0-9a-fA-F]{32}$' })
if ($invalidRuntimeReferences.Count -gt 0) {
    throw "Runtime assembly contains invalid references: $($invalidRuntimeReferences -join ', ')"
}
$protocolFiles = @("BrowserIpcOutputPayload.cs")
foreach ($protocolFile in $protocolFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $protocolScriptsPath $protocolFile) -PathType Leaf)) {
        throw "Protocol file is missing: $protocolFile"
    }
}
$protocolUnityReferences = Get-ChildItem -LiteralPath $protocolScriptsPath -Filter *.cs -File | Select-String -Pattern "UnityEngine|Unity\.Collections|DllImport\s*\(" -AllMatches
if ($protocolUnityReferences) {
    throw "Protocol source contains Unity or native references: $($protocolUnityReferences.Path -join ', ')"
}
$runtimeAsmdefPath = Join-Path $unityRoot "Scripts\KimoTech.LichoraHost.asmdef"
$nativeAsmdefPath = Join-Path $nativeScriptsPath "KimoTech.LichoraHost.Native.asmdef"
if (-not (Test-Path -LiteralPath $nativeAsmdefPath -PathType Leaf)) {
    throw "Native assembly definition not found: $nativeAsmdefPath"
}

$runtimeAsmdef = Get-Content -Raw -LiteralPath $runtimeAsmdefPath | ConvertFrom-Json
$nativeAsmdef = Get-Content -Raw -LiteralPath $nativeAsmdefPath | ConvertFrom-Json
if ($nativeAsmdef.name -ne "KimoTech.LichoraHost.Native" -or $nativeAsmdef.rootNamespace -ne "KimoTech.LichoraHost") {
    throw "Native assembly definition has unexpected name or root namespace."
}
if ($nativeAsmdef.autoReferenced -ne $false) {
    throw "Native assembly must not be auto-referenced."
}
$nativeGuid = (Get-Content -Raw -LiteralPath "$nativeAsmdefPath.meta") | Select-String -Pattern 'guid:\s*(\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }
if (-not $nativeGuid -or $runtimeAsmdef.references -notcontains "GUID:$nativeGuid") {
    throw "Runtime assembly does not explicitly reference Native assembly."
}
$runtimeGuid = (Get-Content -Raw -LiteralPath "$unityRoot\Scripts\KimoTech.LichoraHost.asmdef.meta") | Select-String -Pattern '^guid:\s*(\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value }
if ($nativeAsmdef.references -contains "GUID:$runtimeGuid") {
    throw "Native assembly must not reference Runtime assembly."
}

$nativeBridgeFiles = @(
    "BrowserIpcNative.cs",
    "BrowserIpcInputPayload.cs",
    "BrowserIpcProtocolTypes.cs",
    "NativeIpcHandle.cs",
    "NativeIpcResult.cs",
    "NativeIpcTypes.cs",
    "NativeProcessSpawner.cs",
    "NativeImeBridge.cs",
    "NativeImeClient.cs"
)
foreach ($bridgeFile in $nativeBridgeFiles) {
    $nativePath = Join-Path $nativeScriptsPath $bridgeFile
    if (-not (Test-Path -LiteralPath $nativePath -PathType Leaf)) {
        throw "Native bridge file is outside Scripts/Native or missing: $bridgeFile"
    }
}
$nativeFacadeFiles = @(
    "BrowserIpcClient.cs",
    "BrowserIpcControlWriter.cs",
    "BrowserIpcInputWriter.cs",
    "BrowserIpcOutputReader.cs",
    "BrowserIpcFrameReader.cs",
    "BrowserIpcStatusReader.cs"
)
foreach ($facadeFile in $nativeFacadeFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $nativeScriptsPath $facadeFile) -PathType Leaf)) {
        throw "Native facade file is outside Scripts/Native or missing: $facadeFile"
    }
}
$runtimeScripts = Get-ChildItem -LiteralPath (Join-Path $unityRoot "Scripts") -Recurse -Filter *.cs -File | Where-Object { $_.FullName -notlike "$nativeScriptsPath\*" -and $_.FullName -notlike "$protocolScriptsPath\*" -and $_.FullName -notlike "$modelScriptsPath\*" }
$runtimeBoundaryViolations = $runtimeScripts | Select-String -Pattern "DllImport\s*\(|\bunsafe\b|\bIntPtr\b|\bNativeIpc(Handle|Result|ErrorCode|SessionOptions)\b|\bBrowserIpcNative\b|\bNativeImeBridge\b" -AllMatches
if ($runtimeBoundaryViolations) {
    throw "Runtime source crosses the Native implementation boundary: $($runtimeBoundaryViolations.Path -join ', ')"
}
$friendAssemblyDeclarations = Get-ChildItem -LiteralPath (Join-Path $unityRoot "Scripts") -Recurse -Filter *.cs -File | Select-String -Pattern "InternalsVisibleTo" -AllMatches
if ($friendAssemblyDeclarations) {
    throw "InternalsVisibleTo is not allowed for Unity package assembly boundaries."
}
$nativeRuntimeReferences = Get-ChildItem -LiteralPath $nativeScriptsPath -Filter *.cs -File | Select-String -Pattern "\bIImeInputSink\b|\bISurroundingTextSnapshotProvider\b|\bKeyEventType\b|\bKeyboardState\b|\bPageHandler\b|\bBrowserOutputState\b" -AllMatches
if ($nativeRuntimeReferences) {
    throw "Native assembly references Runtime business types: $($nativeRuntimeReferences.Path -join ', ')"
}
$requiredPublicFacades = @(
    @{ File = "BrowserIpcClient.cs"; Pattern = "public sealed class BrowserIpcClient" },
    @{ File = "BrowserIpcControlWriter.cs"; Pattern = "public sealed class BrowserIpcControlWriter" },
    @{ File = "BrowserIpcInputWriter.cs"; Pattern = "public sealed class BrowserIpcInputWriter" },
    @{ File = "BrowserIpcOutputReader.cs"; Pattern = "public sealed class BrowserIpcOutputReader" },
    @{ File = "BrowserIpcFrameReader.cs"; Pattern = "public sealed class BrowserIpcFrameReader" },
    @{ File = "BrowserIpcStatusReader.cs"; Pattern = "public sealed class BrowserIpcStatusReader" },
    @{ File = "NativeImeClient.cs"; Pattern = "public sealed class NativeImeClient" },
    @{ File = "NativeProcessSpawner.cs"; Pattern = "public static class NativeProcessSpawner" }
)
foreach ($facade in $requiredPublicFacades) {
    $source = Get-Content -Raw -LiteralPath (Join-Path $nativeScriptsPath $facade.File)
    if ($source -notmatch [regex]::Escape($facade.Pattern)) {
        throw "Native facade is not public: $($facade.File)"
    }
}
$requiredInternalImplementations = @(
    @{ File = "BrowserIpcNative.cs"; Pattern = "internal static class BrowserIpcNative" },
    @{ File = "NativeIpcHandle.cs"; Pattern = "internal sealed class NativeIpcHandle" },
    @{ File = "NativeIpcResult.cs"; Pattern = "internal readonly struct NativeIpcResult" },
    @{ File = "NativeImeBridge.cs"; Pattern = "internal sealed class NativeImeBridge" }
)
foreach ($implementation in $requiredInternalImplementations) {
    $source = Get-Content -Raw -LiteralPath (Join-Path $nativeScriptsPath $implementation.File)
    if ($source -notmatch [regex]::Escape($implementation.Pattern)) {
        throw "Native implementation is not internal: $($implementation.File)"
    }
}
$outsideNativeDllImports = Get-ChildItem -LiteralPath (Join-Path $unityRoot "Scripts") -Recurse -Filter *.cs -File | Where-Object { $_.FullName -notlike "$nativeScriptsPath\*" } | Select-String -Pattern "DllImport\s*\(" -AllMatches
if ($outsideNativeDllImports) {
    throw "DllImport found outside Scripts/Native: $($outsideNativeDllImports.Path -join ', ')"
}
$package = Get-Content -Raw -LiteralPath $packagePath | ConvertFrom-Json
$unityRoot = Join-Path $repoRoot "hosts\unity-host"
$unityScripts = (Get-ChildItem -LiteralPath (Join-Path $unityRoot "Scripts") -Recurse -Filter *.cs -File | ForEach-Object { Get-Content -Raw -LiteralPath $_.FullName }) -join "
"
$buildScript = Get-Content -Raw -LiteralPath $buildScriptPath

if ($manifest.schemaVersion -ne 1) {
    throw "Unsupported plugin manifest schema: $($manifest.schemaVersion)"
}

foreach ($plugin in $manifest.plugins) {
    foreach ($property in @("name", "path", "platform", "architecture", "dllImport", "release", "sourceCrate", "abiVersion")) {
        if (-not ($plugin.PSObject.Properties.Name -contains $property)) {
            throw "Plugin '$($plugin.name)' is missing manifest field '$property'."
        }
    }

    $pluginPath = Join-Path $unityRoot ($plugin.path -replace '/', '\')
    if (-not (Test-Path -LiteralPath $pluginPath -PathType Leaf)) {
        throw "Plugin '$($plugin.name)' is missing: $($plugin.path)"
    }

    $constantPattern = 'const\s+string\s+PLUGIN\s*=\s*"' + [regex]::Escape($plugin.dllImport) + '"'
    if ($unityScripts -notmatch $constantPattern) {
        throw "No Unity P/Invoke plugin constant found for '$($plugin.dllImport)'."
    }

    if ($plugin.platform -eq "Windows" -and $plugin.release) {
        $fileName = Split-Path -Leaf $plugin.path
        if ($buildScript -notmatch [regex]::Escape($fileName)) {
            throw "Windows release plugin '$fileName' is not copied by build.ps1."
        }
    }

    if ($plugin.path -match '^Plugins/Linux/.+\.so$') {
        $metaPath = "$pluginPath.meta"
        if (-not (Test-Path -LiteralPath $metaPath -PathType Leaf)) {
            throw "Unity importer metadata missing: $metaPath"
        }

        $meta = Get-Content -Raw -LiteralPath $metaPath
        $linuxEnabled = $meta -match '(?ms)Standalone: Linux64.*?second:\s+enabled:\s+1'
        if ([bool]$plugin.release -ne [bool]$linuxEnabled) {
            throw "Linux importer state does not match release flag for '$($plugin.name)'."
        }
    }
}

if ($manifest.package.name -ne $package.name -or $manifest.package.version -ne $package.version) {
    throw "Plugin manifest package metadata does not match hosts/unity-host/package.json."
}

Write-Host "Plugin contract check passed: $($manifest.plugins.Count) plugins." -ForegroundColor Green
