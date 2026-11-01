# lyrn installer for Windows:
#   irm https://raw.githubusercontent.com/lacodda/lyrn/main/tools/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "lacodda/lyrn"

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP, and an installer
# that fails because someone else on the same address ran it is no installer.
# $env:LYRN_VERSION pins a specific release.
$tag = $env:LYRN_VERSION
if (-not $tag) {
    $request = [Net.HttpWebRequest]::Create("https://github.com/$repo/releases/latest")
    $request.AllowAutoRedirect = $false
    $request.UserAgent = "lyrn-installer"
    try {
        $response = $request.GetResponse()
        $tag = ($response.Headers["Location"] -split "/")[-1]
        $response.Close()
    } catch {
        throw "Cannot resolve the latest release of ${repo}: $($_.Exception.Message)"
    }
}
if (-not $tag -or $tag -notmatch '^v\d') {
    throw "Cannot resolve the latest release of $repo - set `$env:LYRN_VERSION to a release tag (vX.Y.Z)"
}

$name = "lyrn-$tag-x86_64-pc-windows-msvc"
$url = "https://github.com/$repo/releases/download/$tag/$name.zip"
$dir = if ($env:LYRN_INSTALL_DIR) { $env:LYRN_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\lyrn" }
$tmp = Join-Path ([IO.Path]::GetTempPath()) "lyrn-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force $tmp | Out-Null

$target = Join-Path $dir "lyrn.exe"
try {
    Write-Host "Downloading $url"
    Invoke-WebRequest $url -OutFile (Join-Path $tmp "lyrn.zip")
    Expand-Archive (Join-Path $tmp "lyrn.zip") -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Force $dir | Out-Null
    # A running lyrn.exe cannot be overwritten, but it can be renamed: the old
    # one steps aside, so an update does not fail while lyrn runs. It is
    # deleted below, or by the next install once it no longer runs.
    if (Test-Path $target) {
        Move-Item $target "$target.old-$([guid]::NewGuid())" -Force
    }
    Copy-Item (Join-Path $tmp "$name\lyrn.exe") $target -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
Get-ChildItem $dir -Filter "lyrn.exe.old-*" -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue

# $env:LYRN_NO_PATH leaves the PATH alone: an install into a scratch
# directory - `lyrn check --release --install`, which proves the installer
# works - must not add that directory to the user's PATH.
if ($env:LYRN_NO_PATH) {
    Write-Host "Installed lyrn $tag to $target"
    return
}

# The PATH is edited through the registry rather than with
# [Environment]::SetEnvironmentVariable, which stores the value as a plain
# string. A user PATH is nearly always REG_EXPAND_SZ, because entries like
# `%JAVA_HOME%\bin` are written unexpanded - demoting the type would leave
# every one of them as literal text.
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
try {
    $userPath = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    if (($userPath -split ";" | Where-Object { $_ }) -notcontains $dir) {
        $updated = if ($userPath) { "$userPath;$dir" } else { $dir }
        # Absent or already expandable: the expandable form is what Windows
        # itself writes, and a string with nothing to expand expands to itself.
        # A user who never had a per-user PATH has no value to ask about, and
        # GetValueKind throws rather than reporting that.
        $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
        if ($userPath) {
            try {
                if ($key.GetValueKind("Path") -eq [Microsoft.Win32.RegistryValueKind]::String) {
                    $kind = [Microsoft.Win32.RegistryValueKind]::String
                }
            } catch {
                # Leave it expandable.
            }
        }
        $key.SetValue("Path", $updated, $kind)
        Write-Host "Added $dir to your user PATH - restart the terminal to pick it up."
    }
} finally {
    $key.Close()
}

Write-Host "Installed lyrn $tag to $target"
