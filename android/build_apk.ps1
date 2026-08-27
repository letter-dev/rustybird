$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$sdk = "$env:LOCALAPPDATA\Android\Sdk"
if (Test-Path "F:\Android\ndk-27.3.13750724") {
    $ndkPath = "F:\Android\ndk-27.3.13750724"
} else {
    $ndkPath = (Get-ChildItem "$sdk\ndk" | Sort-Object Name -Descending | Select-Object -First 1).FullName
}
$btPath = (Get-ChildItem "$sdk\build-tools" | Sort-Object Name -Descending | Select-Object -First 1).FullName
$androidJar = (Get-ChildItem "$sdk\platforms" | Sort-Object Name -Descending | Select-Object -First 1).FullName + "\android.jar"
$jbr = "C:\Program Files\Android\Android Studio\jbr"

$env:JAVA_HOME = $jbr
$env:PATH = "$jbr\bin;$env:PATH"
$env:ANDROID_NDK_HOME = $ndkPath
$env:ANDROID_NDK_ROOT = $ndkPath

Write-Host "== NDK: $ndkPath"
Write-Host "== build-tools: $btPath"
Write-Host "== platform: $androidJar"

$work = "$PSScriptRoot\build"
if (Test-Path $work) { Remove-Item $work -Recurse -Force }
New-Item -ItemType Directory -Force -Path "$work\classes", "$work\apk" | Out-Null

# 1. Rust .so (universal: all ABIs)
Write-Host "== cargo build (android targets)"
$abis = @(
    @{ target = "aarch64-linux-android"; dir = "arm64-v8a" },
    @{ target = "armv7-linux-androideabi"; dir = "armeabi-v7a" },
    @{ target = "i686-linux-android"; dir = "x86" },
    @{ target = "x86_64-linux-android"; dir = "x86_64" }
)
Push-Location $root
foreach ($abi in $abis) {
    cargo build --release --lib --target $abi.target
    if ($LASTEXITCODE -ne 0) { Pop-Location; throw "cargo build failed for $($abi.target)" }
}
Pop-Location

# 2. javac
Write-Host "== javac"
$javaSources = @(
    "$PSScriptRoot\java-src\com\rustybird\game\MainActivity.java",
    "$PSScriptRoot\java-src\quad_native\QuadNative.java"
)
& "$jbr\bin\javac.exe" -source 8 -target 8 -bootclasspath $androidJar -d "$work\classes" $javaSources
if ($LASTEXITCODE -ne 0) { throw "javac failed" }

# 3. d8 dex
Write-Host "== d8"
Push-Location "$work\classes"
& "$jbr\bin\jar.exe" cf "$work\classes.jar" .
Pop-Location
& "$btPath\d8.bat" --release --lib $androidJar --output "$work\apk" "$work\classes.jar"
if ($LASTEXITCODE -ne 0) { throw "d8 failed" }

# 4. assets
Write-Host "== assets"
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\assets\sprites" | Out-Null
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\assets\audio" | Out-Null
Copy-Item "$root\sprites\*" "$PSScriptRoot\assets\sprites\" -Recurse -Force
Copy-Item "$root\audio\*" "$PSScriptRoot\assets\audio\" -Recurse -Force

# 5. aapt2
Write-Host "== aapt2"
& "$btPath\aapt2.exe" compile --dir "$PSScriptRoot\res" -o "$work\res.zip"
if ($LASTEXITCODE -ne 0) { throw "aapt2 compile failed" }
& "$btPath\aapt2.exe" link -o "$work\unsigned.apk" -I $androidJar --manifest "$PSScriptRoot\AndroidManifest.xml" "$work\res.zip"
if ($LASTEXITCODE -ne 0) { throw "aapt2 link failed" }

# 6. pack dex + native lib + assets (manual, forward slashes)
Write-Host "== pack"
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::Open("$work\unsigned.apk", "Update")
[void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, "$work\apk\classes.dex", "classes.dex", "Optimal")
foreach ($abi in $abis) {
    $soPath = "$root\target\$($abi.target)\release\librustybird.so"
    [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $soPath, "lib/$($abi.dir)/librustybird.so", "Optimal")
}
Get-ChildItem "$PSScriptRoot\assets" -Recurse -File | ForEach-Object {
    $rel = $_.FullName.Substring("$PSScriptRoot\assets\".Length).Replace("\", "/")
    [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $_.FullName, "assets/$rel", "Optimal")
}
$zip.Dispose()

# 7. zipalign
Write-Host "== zipalign"
& "$btPath\zipalign.exe" -f 4 "$work\unsigned.apk" "$work\aligned.apk"
if ($LASTEXITCODE -ne 0) { throw "zipalign failed" }

# 8. sign
Write-Host "== sign"
$ks = "$PSScriptRoot\rustybird.keystore"
if (!(Test-Path $ks)) {
    & "$jbr\bin\keytool.exe" -genkeypair -keystore $ks -alias rustybird -keyalg RSA -keysize 2048 -validity 10000 -storepass rustybird -keypass rustybird -dname "CN=RustyBird"
    if ($LASTEXITCODE -ne 0) { throw "keytool failed" }
}
$outDir = "$root\androidgame"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
& "$btPath\apksigner.bat" sign --ks $ks --ks-pass pass:rustybird --out "$outDir\RustyBird.apk" "$work\aligned.apk"
if ($LASTEXITCODE -ne 0) { throw "apksigner failed" }

Write-Host "== DONE: $outDir\RustyBird.apk"
