param(
    [string]$ContainerName = "pg_copia",
    [switch]$Push
)

$ErrorActionPreference = "Stop"

function Write-Info([string]$Message) {
    Write-Host "[container-sync] $Message"
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
$branch = (& git -C $repoRoot branch --show-current).Trim()
$outRoot = Join-Path $repoRoot "github_backups\container_sync"
$outDir = Join-Path $outRoot $timestamp
Ensure-Dir $outDir

Write-Info "Collecting container metadata from $ContainerName"
$inspect = (& docker inspect $ContainerName) -join [Environment]::NewLine
Save-Text (Join-Path $outDir "container_inspect.json") $inspect

$imageName = (& docker inspect -f "{{.Config.Image}}" $ContainerName).Trim()
$imageInspect = (& docker image inspect $imageName) -join [Environment]::NewLine
Save-Text (Join-Path $outDir "image_inspect.json") $imageInspect

$envDump = (& docker inspect -f "{{range .Config.Env}}{{println .}}{{end}}" $ContainerName) -join [Environment]::NewLine
Save-Text (Join-Path $outDir "container_env.txt") $envDump

Write-Info "Dumping PostgreSQL database"
$tmpDump = "/tmp/knowledge_base_${timestamp}.dump"
$tmpSchema = "/tmp/knowledge_base_${timestamp}_schema.sql"
$tmpStats = "/tmp/knowledge_base_${timestamp}_stats.txt"

$dumpCmd = 'PGPASSWORD="$POSTGRES_PASSWORD" pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -F c -f "' + $tmpDump + '"'
$schemaCmd = 'PGPASSWORD="$POSTGRES_PASSWORD" pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -s -f "' + $tmpSchema + '"'
$statsCmd = 'PGPASSWORD="$POSTGRES_PASSWORD" psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c ''SELECT status, COUNT(*) FROM validation GROUP BY status ORDER BY status;'' > "' + $tmpStats + '"'

& docker exec $ContainerName sh -lc $dumpCmd
& docker exec $ContainerName sh -lc $schemaCmd
& docker exec $ContainerName sh -lc $statsCmd

$dumpPath = Join-Path $outDir "knowledge_base.dump"
$schemaPath = Join-Path $outDir "knowledge_base_schema.sql"
$statsPath = Join-Path $outDir "validation_stats.txt"

& docker cp "${ContainerName}:$tmpDump" $dumpPath
& docker cp "${ContainerName}:$tmpSchema" $schemaPath
& docker cp "${ContainerName}:$tmpStats" $statsPath

& docker exec $ContainerName sh -lc "rm -f $tmpDump $tmpSchema $tmpStats"

$dumpHash = (Get-FileHash -Algorithm SHA256 -Path $dumpPath).Hash
$schemaHash = (Get-FileHash -Algorithm SHA256 -Path $schemaPath).Hash
$statsHash = (Get-FileHash -Algorithm SHA256 -Path $statsPath).Hash

$manifest = @"
{
  "timestamp": "$timestamp",
  "container": "$ContainerName",
  "image": "$imageName",
  "branch": "$branch",
  "files": {
    "knowledge_base.dump.sha256": "$dumpHash",
    "knowledge_base_schema.sql.sha256": "$schemaHash",
    "validation_stats.txt.sha256": "$statsHash"
  }
}
"@
Save-Text (Join-Path $outDir "manifest.json") $manifest

Write-Info "Container sync snapshot generated: github_backups/container_sync/$timestamp"

Push-Location $repoRoot
try {
    & git add "github_backups/container_sync/$timestamp"
    & git add "scripts/sync_container_to_github.ps1"

    & git diff --cached --quiet
    if ($LASTEXITCODE -ne 0) {
        & git commit -m "backup(container): sync $ContainerName at $timestamp"
        Write-Info "Commit created on branch $branch"

        if ($Push) {
            & git push origin $branch
            Write-Info "Pushed to origin/$branch"
        }
    }
    else {
        Write-Info "No staged changes to commit."
    }
}
finally {
    Pop-Location
}

Write-Output "Snapshot: github_backups/container_sync/$timestamp"
