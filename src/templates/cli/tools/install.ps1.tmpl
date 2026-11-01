# {{ name }} installer for Windows:
#   irm https://raw.githubusercontent.com/{{ repo }}/main/tools/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "{{ repo }}"

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP, and an installer
# that fails because someone else on the same address ran it is no installer.
$tag = $env:{{ env_prefix }}_VERSION
if (-not $tag) {
    $request = [Net.HttpWebRequest]::Create("https://github.com/$repo/releases/latest")
    $request.AllowAutoRedirect = $false
    $request.UserAgent = "{{ name }}-installer"
    try {
        $response = $request.GetResponse()
        $tag = ($response.Headers["Location"] -split "/")[-1]
        $response.Close()
    } catch {
        throw "Cannot resolve the latest release of ${repo}: $($_.Exception.Message)"
    }
}
if (-not $tag -or $tag -notmatch '^v\d') {
    throw "Cannot resolve the latest release of $repo - set `$env:{{ env_prefix }}_VERSION to a release tag (vX.Y.Z)"
}

$name = "{{ name }}-$tag-x86_64-pc-windows-msvc"
$url = "https://github.com/$repo/releases/download/$tag/$name.zip"
$dir = if ($env:{{ env_prefix }}_INSTALL_DIR) { $env:{{ env_prefix }}_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\{{ name }}" }
$tmp = Join-Path ([IO.Path]::GetTempPath()) "{{ name }}-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force $tmp | Out-Null

$target = Join-Path $dir "{{ name }}.exe"
try {
    Write-Host "Downloading $url"
    Invoke-WebRequest $url -OutFile (Join-Path $tmp "{{ name }}.zip")
    Expand-Archive (Join-Path $tmp "{{ name }}.zip") -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Force $dir | Out-Null
    # A running {{ name }}.exe cannot be overwritten, but it can be renamed: the
    # old one steps aside, so an update does not fail while the program runs.
    # It is deleted below, or by the next install once it no longer runs.
    if (Test-Path $target) {
        Move-Item $target "$target.old-$([guid]::NewGuid())" -Force
    }
    Copy-Item (Join-Path $tmp "$name\{{ name }}.exe") $target -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
Get-ChildItem $dir -Filter "{{ name }}.exe.old-*" -ErrorAction SilentlyContinue |
    Remove-Item -Force -ErrorAction SilentlyContinue

# $env:{{ env_prefix }}_NO_PATH leaves the PATH alone: an install into a scratch
# directory - the release check that proves the installer works - must not
# add that directory to the user's PATH.
if ($env:{{ env_prefix }}_NO_PATH) {
    Write-Host "Installed {{ name }} $tag to $target"
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

Write-Host "Installed {{ name }} $tag to $target"
