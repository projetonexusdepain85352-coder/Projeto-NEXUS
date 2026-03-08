param(
    [string]$Label = "manual",
    [string]$ContainerName = "pg_copia",
    [switch]$SkipContainerCopy,
    [switch]$CommitGithubBackup,
    [switch]$PushGithubBackup
)

$ErrorActionPreference = "Stop"

function Write-Info([string]$Message) {
    Write-Host "[backup] $Message"
}

function Ensure-Dir([string]$Path) {
    New-Item -ItemType Directory -Path $Path -Force | Out-Null
}

function Save-Text([string]$Path, [string]$Content) {
    $enc = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $enc)
}

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$safeLabel = ($Label.ToLower() -replace "[^a-z0-9_-]", "_")

$localBackupRoot = Join-Path $repoRoot "backups\code_snapshots"
$githubBackupRoot = Join-Path $repoRoot "github_backups"
$snapshotName = "${timestamp}_${safeLabel}"
$archiveName = "nexus_code_${snapshotName}.tar.gz"
$archivePath = Join-Path $localBackupRoot $archiveName
$githubSnapshotDir = Join-Path $githubBackupRoot $snapshotName
$githubUntrackedDir = Join-Path $githubSnapshotDir "untracked_files"

Ensure-Dir $localBackupRoot
Ensure-Dir $githubSnapshotDir
Ensure-Dir $githubUntrackedDir

Write-Info "Creating local code archive: $archivePath"
Push-Location $repoRoot
try {
    & tar.exe `
        --exclude ".git" `
        --exclude "backups" `
        --exclude "models" `
        --exclude "dados" `
        --exclude "logs" `
        --exclude "target" `
        --exclude "github_backups" `
        -a -c -f $archivePath .
}
finally {
    Pop-Location
}

if (-not (Test-Path $archivePath)) {
    throw "Archive not created: $archivePath"
}

$archiveHash = (Get-FileHash -Algorithm SHA256 -Path $archivePath).Hash
$archiveSize = (Get-Item $archivePath).Length
Write-Info "Archive created ($archiveSize bytes, sha256=$archiveHash)"

Push-Location $repoRoot
try {
    $status = (& git status --short) -join [Environment]::NewLine
    $diff = (& git diff) -join [Environment]::NewLine
    $staged = (& git diff --cached) -join [Environment]::NewLine
    $untracked = (& git ls-files --others --exclude-standard) -join [Environment]::NewLine
    $head = (& git rev-parse HEAD).Trim()
    $branch = (& git branch --show-current).Trim()
}
finally {
    Pop-Location
}

Save-Text (Join-Path $githubSnapshotDir "git_status.txt") $status
Save-Text (Join-Path $githubSnapshotDir "working_tree.patch") $diff
Save-Text (Join-Path $githubSnapshotDir "staged.patch") $staged
Save-Text (Join-Path $githubSnapshotDir "untracked_list.txt") $untracked

if ($untracked.Trim().Length -gt 0) {
    foreach ($rel in ($untracked -split "(\r?\n)" | Where-Object { $_ -and $_ -notmatch "^\r?$" })) {
        $src = Join-Path $repoRoot $rel
        if (Test-Path $src -PathType Leaf) {
            $dst = Join-Path $githubUntrackedDir $rel
            Ensure-Dir (Split-Path $dst -Parent)
            Copy-Item $src $dst -Force
        }
    }
}

$meta = @"
{
  "timestamp": "$timestamp",
  "label": "$safeLabel",
  "branch": "$branch",
  "head": "$head",
  "archive_path": "$archivePath",
  "archive_sha256": "$archiveHash",
  "archive_size_bytes": $archiveSize
}
"@
Save-Text (Join-Path $githubSnapshotDir "metadata.json") $meta
Write-Info "GitHub backup snapshot generated at github_backups/$snapshotName"

if (-not $SkipContainerCopy) {
    try {
        & docker exec $ContainerName sh -lc "mkdir -p /var/backups/nexus_code" | Out-Null
        & docker cp $archivePath "${ContainerName}:/var/backups/nexus_code/$archiveName"
        Write-Info "Archive copied to container: ${ContainerName}:/var/backups/nexus_code/$archiveName"
    }
    catch {
        Write-Warning "Container copy failed: $($_.Exception.Message)"
    }
}
else {
    Write-Info "Container copy skipped."
}

if ($CommitGithubBackup -or $PushGithubBackup) {
    Push-Location $repoRoot
    try {
        & git add "github_backups/$snapshotName"
        & git commit -m "backup: $snapshotName"
        Write-Info "Git commit created for github_backups/$snapshotName"
        if ($PushGithubBackup) {
            & git push origin $branch
            Write-Info "Backup pushed to origin/$branch"
        }
    }
    finally {
        Pop-Location
    }
}

Write-Output "Local archive: $archivePath"
Write-Output "GitHub snapshot: github_backups/$snapshotName"
