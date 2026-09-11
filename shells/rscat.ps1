# rscat: rainbow cat — https://github.com/anomalyco/rscat
# Added by `rscat --init powershell`. Idempotent; append to $PROFILE:
#   rscat --init powershell >> $PROFILE
$RscatBin = Join-Path $HOME ".local\bin"
if (($env:Path -split ';') -notcontains $RscatBin) { $env:Path = "$RscatBin;$env:Path" }
