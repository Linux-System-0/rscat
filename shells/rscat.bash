# rscat: rainbow cat — https://github.com/macOS-Terminal/rscat
# Added by `rscat --init bash`. Idempotent, safe to re-source.
case ":$PATH:" in
  *":$HOME/.local/bin:"*) ;;
  *) export PATH="$HOME/.local/bin:$PATH" ;;
esac

# --- fastfetch 管道兼容 -------------------------------------------------------
# fastfetch 会检查 stdout 是否为终端,非终端时**整个跳过图片输出**
# (image.c: `if(instance.config.display.pipe) return false;`),于是
#   fastfetch | rscat
# 只剩文字、没有图。`--pipe false` 能关掉这道检查,让图片转义码照发,
# rscat 再把文字染彩虹、把图片序列原样放行。
#
# 只在「管道读端确实是 rscat」时追加:
#   fastfetch | rscat      -> 加
#   fastfetch | lolcat     -> 不加(读端不是 rscat,不改人家的行为)
#   fastfetch | grep x     -> 不加
#   fastfetch > out.txt    -> 不加(否则图片码写进文件)
#   fastfetch              -> 不加(fastfetch 自己就出图)
# 用户已显式给过 --pipe 时不重复追加;绕过包装用 `command fastfetch`。
#
# 判定原理(无竞态):管道在我 exec 之前就已建好、两端各持一端;谁的
# /proc/<pid>/fd/0 接着我 stdout 那根管道(pipe inode),谁就是读端。
# 絶不能放进 $(…):命令替换会把内部命令的 stdout 换成捕获管,读到错误的
# inode。因此探测整体交给一个「不带重定向」的外部 sh —— 它原样继承我们的
# fd 1,自报家门($PPID = 函数所在子 shell,其 fd 1 真是那根管道),从
# /proc/<$PPID>/fd/1 读到管道目标,再扫各进程的 fd/0 找读端。
# 不使用 $$/pgid:交互式 shell 给每条管道分配独立进程组,而函数在管道子
# shell 里 $$ 仍指向主 shell,"同组有 rscat"必然落空(后台那个真实竞态)。
# 只判断 `-p /dev/stdout` 也不够 —— 那对 `| lolcat`、`| grep` 也会误判。
__rscat_piped_into_rscat() {
    [ -p /dev/stdout ] || return 1
    [ -d /proc ] || return 1   # 仅 Linux 有保障;无 /proc 的平台安全地不加
    sh -c '
        me=$PPID    # 函数所在子 shell 的 pid(它的 fd 1 = 通往读端的管道)
        mypipe=$(readlink "/proc/$me/fd/1" 2>/dev/null) || exit 1
        case "$mypipe" in pipe:*) ;; *) exit 1;; esac
        # 扫所有进程,找 fd/0 链接到同一根管道的进程(读端)。exe basename 须为
        # rscat —— /proc/<pid>/exe 最准(comm 会被 rscat 会话代理伪装成 shell 名)。
        for d in /proc/[0-9]*/fd/0; do
            [ -L "$d" ] || continue
            [ "$(readlink "$d" 2>/dev/null)" = "$mypipe" ] || continue
            p=${d%/fd/0}; p=${p#/proc/}
            e=$(readlink "/proc/$p/exe" 2>/dev/null)
            [ -n "$e" ] || e=$(ps -o comm= -p "$p" 2>/dev/null)
            [ -n "$e" ] || continue
            case ${e##*/} in rscat) exit 0 ;; esac
        done
        exit 1
    '
}

fastfetch() {
    local _extra=() _a _has_pipe=0
    if __rscat_piped_into_rscat; then
        for _a in "$@"; do
            case "$_a" in
                --pipe|--pipe=*) _has_pipe=1 ;;
            esac
        done
        [ "$_has_pipe" = 1 ] || _extra=(--pipe false)
    fi
    command fastfetch "${_extra[@]}" "$@"
}
