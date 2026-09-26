# rscat: rainbow cat — https://github.com/anomalyco/rscat
# Added by `rscat --init fish`. Idempotent, safe to re-source.
if not contains -- $HOME/.local/bin $PATH
    set -gx PATH $HOME/.local/bin $PATH
end

# --- fastfetch 管道兼容 -------------------------------------------------------
# fastfetch 在 stdout 非终端时会整个跳过图片输出(image.c 里 `display.pipe`
# 为真就直接 return false),于是 `fastfetch | rscat` 只剩文字、没有图。
# `--pipe false` 关掉这道检查,图片转义码照发,rscat 再把文字染彩虹、
# 把图片序列原样放行。
#
# 只在「管道读端确实是 rscat」时追加:
#   fastfetch | rscat      -> 加
#   fastfetch | lolcat     -> 不加(读端不是 rscat,不改人家的行为)
#   fastfetch | grep x     -> 不加
#   fastfetch > out.txt    -> 不加(否则图片码写进文件)
#   fastfetch              -> 不加(fastfetch 自己就出图)
# 用户已显式给过 --pipe 时不重复追加;绕过包装用 `command fastfetch`。
#
# 判定原理(fish 版,与 bash 版不同系):fish 把管线里的函数跑在**主进程**,
# 输出经内部缓冲喂给下游 —— 函数自己的 fd 1 看不到「谁是读端」(不像
# bash/zsh/sh 直接 readlink 管道 inode 就能找到)。但实测 fish 此时已把
# 右端进程 spawn 为主进程($fish_pid)的子进程,且它的 fd/0 就是喂它的那根
# 管道。所以:遍历主进程的子进程,exe 名(basename)为 rscat 且其 fd/0
# 是管道的,即管道读端是 rscat。加个有界重试(5×20ms)吸住极端载荷下的
# fork/exec 时序。exe 最准(comm 会被 rscat 会话代理伪装成 shell 名);
# 实在读不到 exe 才退回 ps comm。
# 不使用 pgid:交互式 shell 给每条管线分配独立进程组,主进程不在其中,
# 「同组有 rscat」必然落空。
function __rscat_piped_into_rscat
    test -d /proc; or return 1   # 仅 Linux 保障;无 /proc 的平台安全地不加
    for _try in (seq 1 5)
        set -l psout (ps -eo pid=,ppid= 2>/dev/null)
        test -n "$psout"; or set psout (ps -ax -o pid=,ppid= 2>/dev/null)
        test -n "$psout"; or return 1
        for kid in (printf '%s\n' $psout | awk -v m=$fish_pid '$2==m {print $1}')
            set -l fd0 (readlink /proc/$kid/fd/0 2>/dev/null)
            if not string match -q -- 'pipe:*' $fd0
                continue
            end
            set -l e (readlink /proc/$kid/exe 2>/dev/null)
            test -n "$e"; or set e (ps -o comm= -p $kid 2>/dev/null | string trim)
            test -n "$e"; or continue
            if string match -q -- '*/rscat' $e; or string match -q -- 'rscat' $e
                return 0
            end
        end
        sleep 0.02
    end
    return 1
end

function fastfetch
    set -l extra
    if __rscat_piped_into_rscat
        set -l has_pipe 0
        for a in $argv
            if string match -q -- '--pipe' $a; or string match -q -- '--pipe=*' $a
                set has_pipe 1
            end
        end
        if test $has_pipe -eq 0
            set extra --pipe false
        end
    end
    command fastfetch $extra $argv
end
