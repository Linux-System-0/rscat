# rscat: rainbow cat — https://github.com/macOS-Terminal/rscat
# Added by `rscat --init powershell`. Idempotent; append to $PROFILE:
#   rscat --init powershell >> $PROFILE
$RscatBin = Join-Path $HOME ".local\bin"
if (($env:Path -split ';') -notcontains $RscatBin) { $env:Path = "$RscatBin;$env:Path" }

# --- fastfetch 管道兼容 -------------------------------------------------------
# fastfetch 在 stdout 非终端时会整个跳过图片输出(image.c 里 `display.pipe`
# 为真就直接 return false),于是 `fastfetch | rscat` 只剩文字、没有图。
# `--pipe false` 关掉这道检查,图片转义码照发,rscat 再把文字染彩虹、
# 把图片序列原样放行。
#
# 只在「管道读端确实是 rscat」时追加:
#   fastfetch | rscat      -> 加
#   fastfetch | lolcat     -> 不加(读端不是 rscat,不改人家的行为)
#   fastfetch > out.txt    -> 不加(否则图片码写进文件)
#   fastfetch              -> 不加(fastfetch 自己就出图)
# 用户已显式给过 --pipe 时不重复追加;绕过包装用 `fastfetch.exe`。
#
# 判定原理(POSIX 版的 pgid 思路在 Windows 上的等价物):
#   PowerShell 执行 `a | b` 时会把 a.exe 和 b.exe 都作为自己的**直接子进程**
#   启动,于是管道两端互为**兄弟进程**(ParentProcessId 相同)。
#   所以:stdout 是管道 + 进程表里存在一个与我同父的 rscat.exe => 读端是 rscat。
#   对照 POSIX 版用的是"同进程组",这里用的是"同父",语义一致。
#
#   `fastfetch | lolcat` 时根本没有 rscat.exe 在跑,自然不会命中。
function __Rscat-PipedIntoRscat {
    # stdout 必须是管道。重定向到文件时 CanSeek 为 true,据此排除。
    try {
        $stdout = [System.Console]::OpenStandardOutput()
        if ($stdout.CanSeek) { return $false }
    } catch {
        return $false
    }

    try {
        $me = Get-CimInstance Win32_Process -Filter "ProcessId = $PID" -ErrorAction Stop
        $myParent = $me.ParentProcessId
        if (-not $myParent) { return $false }

        # 同为该 shell 的子进程,且映像是 rscat.exe
        $sibs = Get-CimInstance Win32_Process `
                    -Filter "ParentProcessId = $myParent AND Name = 'rscat.exe'" `
                    -ErrorAction Stop
        foreach ($s in $sibs) {
            if ($s.ProcessId -ne $PID) { return $true }
        }
    } catch {
        # 拿不到进程信息时保守处理:不加参数(宁可少加,也不破坏
        # `fastfetch | lolcat` 之类的既有用法)。
        return $false
    }
    return $false
}

function fastfetch {
    $extra = @()
    if (__Rscat-PipedIntoRscat) {
        $hasPipe = $false
        foreach ($a in $args) {
            if ($a -eq '--pipe' -or $a -like '--pipe=*') { $hasPipe = $true }
        }
        if (-not $hasPipe) { $extra = @('--pipe', 'false') }
    }
    & fastfetch.exe @extra @args
}
