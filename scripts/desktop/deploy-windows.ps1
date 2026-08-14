[CmdletBinding()]
param(
    [ValidateSet("Publish", "InstallLatest", "PublishAndInstall")]
    [string]$Mode = "Publish",

    [switch]$NoSyncMain,
    [switch]$SilentInstall,
    [switch]$KeepInstaller,
    [string]$OutputDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SourceRepo = "zhouxingxing1279/LifeTrace"
$ReleaseRepo = "zhouxingxing1279/LifeTrace-Releases"
$ReleaseWorkflow = "release-windows.yml"
$ExpectedBranch = "main"

function Write-Step([string]$Message) {
    Write-Host "`n==> $Message" -ForegroundColor Cyan
}

function Write-Ok([string]$Message) {
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Fail([string]$Message) {
    throw $Message
}

function Require-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Fail "缺少命令 '$Name'。请先安装后重新运行。"
    }
}

function Invoke-Native([string]$FilePath, [string[]]$Arguments) {
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        Fail "命令执行失败（exit=$LASTEXITCODE）：$FilePath $($Arguments -join ' ')"
    }
}

function Invoke-NativeText([string]$FilePath, [string[]]$Arguments) {
    $output = & $FilePath @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        Fail "命令执行失败（exit=$LASTEXITCODE）：$FilePath $($Arguments -join ' ')`n$($output -join "`n")"
    }
    return (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
}

function Get-RepoRoot {
    $root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
    return $root.Path
}

function Get-DesktopVersion([string]$RepoRoot) {
    $packagePath = Join-Path $RepoRoot "apps\desktop\package.json"
    $tauriPath = Join-Path $RepoRoot "apps\desktop\src-tauri\tauri.conf.json"
    $cargoPath = Join-Path $RepoRoot "apps\desktop\src-tauri\Cargo.toml"

    $package = Get-Content $packagePath -Raw | ConvertFrom-Json
    $tauri = Get-Content $tauriPath -Raw | ConvertFrom-Json
    $cargo = Get-Content $cargoPath -Raw
    $cargoMatch = [regex]::Match($cargo, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $cargoMatch.Success) {
        Fail "无法从 $cargoPath 读取 version。"
    }

    $packageVersion = [string]$package.version
    $tauriVersion = [string]$tauri.version
    $cargoVersion = $cargoMatch.Groups[1].Value
    if ([string]::IsNullOrWhiteSpace($packageVersion) -or
        $packageVersion -ne $tauriVersion -or
        $packageVersion -ne $cargoVersion) {
        Fail "桌面端版本不一致：package.json=$packageVersion, tauri.conf.json=$tauriVersion, Cargo.toml=$cargoVersion"
    }
    return $packageVersion
}

function Assert-SourceRepository([string]$RepoRoot) {
    Push-Location $RepoRoot
    try {
        Require-Command "git"
        $inside = Invoke-NativeText "git" @("rev-parse", "--is-inside-work-tree")
        if ($inside -ne "true") {
            Fail "当前目录不是 Git 工作区：$RepoRoot"
        }

        $origin = Invoke-NativeText "git" @("remote", "get-url", "origin")
        if ($origin -notmatch 'zhouxingxing1279/LifeTrace(?:\.git)?$') {
            Fail "origin 不是 $SourceRepo：$origin"
        }
    }
    finally {
        Pop-Location
    }
}

function Sync-Main([string]$RepoRoot) {
    Push-Location $RepoRoot
    try {
        $dirty = Invoke-NativeText "git" @("status", "--porcelain")
        if (-not [string]::IsNullOrWhiteSpace($dirty)) {
            Fail "工作区存在未提交修改。为避免覆盖本地内容，部署已停止。请先提交或暂存改动。"
        }

        $branch = Invoke-NativeText "git" @("branch", "--show-current")
        if ($branch -ne $ExpectedBranch) {
            Write-Step "切换到 $ExpectedBranch"
            Invoke-Native "git" @("checkout", $ExpectedBranch)
        }

        Write-Step "同步 origin/$ExpectedBranch"
        Invoke-Native "git" @("fetch", "origin", $ExpectedBranch)
        Invoke-Native "git" @("pull", "--ff-only", "origin", $ExpectedBranch)

        $head = Invoke-NativeText "git" @("rev-parse", "HEAD")
        $remote = Invoke-NativeText "git" @("rev-parse", "origin/$ExpectedBranch")
        if ($head -ne $remote) {
            Fail "本地 $ExpectedBranch 与 origin/$ExpectedBranch 不一致，拒绝发布。HEAD=$head remote=$remote"
        }
        Write-Ok "main 已同步：$head"
        return $head
    }
    finally {
        Pop-Location
    }
}

function Get-MainHead([string]$RepoRoot) {
    Push-Location $RepoRoot
    try {
        $branch = Invoke-NativeText "git" @("branch", "--show-current")
        if ($branch -ne $ExpectedBranch) {
            Fail "当前分支是 '$branch'，使用 -NoSyncMain 时必须手动位于 main。"
        }
        $dirty = Invoke-NativeText "git" @("status", "--porcelain")
        if (-not [string]::IsNullOrWhiteSpace($dirty)) {
            Fail "工作区存在未提交修改，拒绝发布。"
        }
        return Invoke-NativeText "git" @("rev-parse", "HEAD")
    }
    finally {
        Pop-Location
    }
}

function Assert-GitHubAuth {
    Require-Command "gh"
    & gh auth status *> $null
    if ($LASTEXITCODE -ne 0) {
        Fail "GitHub CLI 尚未登录。请先执行：gh auth login"
    }
    Write-Ok "GitHub CLI 已登录"
}

function Test-ReleaseExists([string]$Tag) {
    & gh release view $Tag --repo $ReleaseRepo --json tagName *> $null
    return ($LASTEXITCODE -eq 0)
}

function Find-WorkflowRun([string]$HeadSha, [DateTimeOffset]$TriggeredAfter) {
    for ($attempt = 1; $attempt -le 40; $attempt++) {
        $json = Invoke-NativeText "gh" @(
            "run", "list",
            "--repo", $SourceRepo,
            "--workflow", $ReleaseWorkflow,
            "--branch", $ExpectedBranch,
            "--event", "workflow_dispatch",
            "--limit", "20",
            "--json", "databaseId,headSha,createdAt,status,conclusion"
        )
        $runs = @($json | ConvertFrom-Json)
        $run = $runs |
            Where-Object {
                $_.headSha -eq $HeadSha -and
                ([DateTimeOffset]::Parse([string]$_.createdAt)) -ge $TriggeredAfter
            } |
            Sort-Object { [DateTimeOffset]::Parse([string]$_.createdAt) } -Descending |
            Select-Object -First 1
        if ($null -ne $run) {
            return $run
        }
        Start-Sleep -Seconds 3
    }
    Fail "已触发发布工作流，但在等待窗口内没有找到对应的 GitHub Actions run。"
}

function Publish-Desktop([string]$RepoRoot) {
    Assert-SourceRepository $RepoRoot
    Assert-GitHubAuth

    $headSha = if ($NoSyncMain) { Get-MainHead $RepoRoot } else { Sync-Main $RepoRoot }
    $version = Get-DesktopVersion $RepoRoot
    $tag = "v$version"
    Write-Ok "桌面端版本一致：$tag"

    if (Test-ReleaseExists $tag) {
        Fail "Release $tag 已存在。桌面应用发布必须先提升版本号，避免覆盖已签名安装包和 updater manifest。"
    }

    Write-Step "触发 Windows 正式发布：$tag"
    $triggeredAfter = [DateTimeOffset]::UtcNow.AddSeconds(-5)
    Invoke-Native "gh" @(
        "workflow", "run", $ReleaseWorkflow,
        "--repo", $SourceRepo,
        "--ref", $ExpectedBranch,
        "-f", "version=$version"
    )

    $run = Find-WorkflowRun $headSha $triggeredAfter
    $runId = [string]$run.databaseId
    Write-Ok "已找到 Release Windows Installer run #$runId"
    Write-Step "等待 GitHub Actions 构建、签名并发布安装包"
    Invoke-Native "gh" @("run", "watch", $runId, "--repo", $SourceRepo, "--exit-status")

    Write-Step "校验公开 Release 与 updater manifest"
    $releaseJson = Invoke-NativeText "gh" @("release", "view", $tag, "--repo", $ReleaseRepo, "--json", "tagName,url,assets")
    $release = $releaseJson | ConvertFrom-Json
    if ([string]$release.tagName -ne $tag) {
        Fail "发布完成但 Release tag 不匹配：expected=$tag actual=$($release.tagName)"
    }

    $latestUrl = "https://github.com/$ReleaseRepo/releases/latest/download/latest.json"
    $manifest = Invoke-RestMethod -Uri $latestUrl -TimeoutSec 30
    if ([string]$manifest.version -ne $version) {
        Fail "latest.json 版本不匹配：expected=$version actual=$($manifest.version)"
    }
    $platform = $manifest.platforms."windows-x86_64"
    if ($null -eq $platform -or [string]::IsNullOrWhiteSpace([string]$platform.url) -or [string]::IsNullOrWhiteSpace([string]$platform.signature)) {
        Fail "latest.json 缺少 windows-x86_64 的 url/signature。"
    }

    Write-Ok "Windows 桌面应用发布完成：$tag"
    Write-Host "Release: $($release.url)"
    return $tag
}

function Get-LatestReleaseTag {
    Assert-GitHubAuth
    $json = Invoke-NativeText "gh" @("api", "repos/$ReleaseRepo/releases/latest")
    $release = $json | ConvertFrom-Json
    $tag = [string]$release.tag_name
    if ([string]::IsNullOrWhiteSpace($tag)) {
        Fail "无法读取最新桌面 Release tag。"
    }
    return $tag
}

function Download-Installer([string]$Tag, [string]$TargetDirectory) {
    $version = $Tag.TrimStart('v')
    if ([string]::IsNullOrWhiteSpace($TargetDirectory)) {
        $TargetDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "lifetrace-desktop-deploy"
    }
    New-Item -ItemType Directory -Force -Path $TargetDirectory | Out-Null

    Write-Step "下载 $Tag Windows 安装包"
    Invoke-Native "gh" @(
        "release", "download", $Tag,
        "--repo", $ReleaseRepo,
        "--pattern", "*_${version}_x64-setup.exe",
        "--dir", $TargetDirectory,
        "--clobber"
    )

    $installer = Get-ChildItem -Path $TargetDirectory -Filter "*_${version}_x64-setup.exe" -File |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1
    if ($null -eq $installer) {
        Fail "下载完成但没有找到版本 $version 的 NSIS 安装包。"
    }
    Write-Ok "安装包：$($installer.FullName)"
    return $installer.FullName
}

function Install-Desktop([string]$InstallerPath) {
    Write-Step "安装 LifeTrace Windows 桌面应用"
    $arguments = @()
    if ($SilentInstall) {
        $arguments += "/S"
    }
    $process = Start-Process -FilePath $InstallerPath -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        Fail "安装程序退出码为 $($process.ExitCode)。"
    }
    Write-Ok "LifeTrace 桌面应用安装完成"
}

if ($env:OS -ne "Windows_NT") {
    Fail "该脚本仅支持 Windows。"
}

$repoRoot = Get-RepoRoot
$tagToInstall = $null

switch ($Mode) {
    "Publish" {
        [void](Publish-Desktop $repoRoot)
    }
    "InstallLatest" {
        $tagToInstall = Get-LatestReleaseTag
        Write-Ok "最新正式版本：$tagToInstall"
        $installer = Download-Installer $tagToInstall $OutputDirectory
        Install-Desktop $installer
        if (-not $KeepInstaller -and [string]::IsNullOrWhiteSpace($OutputDirectory)) {
            Remove-Item -LiteralPath $installer -Force -ErrorAction SilentlyContinue
        }
    }
    "PublishAndInstall" {
        $tagToInstall = Publish-Desktop $repoRoot
        $installer = Download-Installer $tagToInstall $OutputDirectory
        Install-Desktop $installer
        if (-not $KeepInstaller -and [string]::IsNullOrWhiteSpace($OutputDirectory)) {
            Remove-Item -LiteralPath $installer -Force -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "`nDone." -ForegroundColor Green
