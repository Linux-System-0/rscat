#!/usr/bin/env python3
"""假终端 oracle：模拟 kitty，检验 nyacat -c fastfetch 是否保持排版。

用法: python3 emu_test.py [--direct|--proxy]
  --direct: 假终端直连 fastfetch（对照组，排版应该正确）
  --proxy : 假终端 -> nyacat -c fastfetch（实验组）
输出虚拟屏幕渲染 + 光标查询日志，对比两组是否一致。
"""
import os
import pty
import re
import select
import struct
import subprocess
import sys
import fcntl
import termios

COLS, ROWS = 160, 50
CELL_W, CELL_H = 10, 20  # 每字符格像素

LOGO_W, LOGO_H = 1080, 1080  # nyarch-logo.png 实际尺寸


class Screen:
    def __init__(self):
        self.grid = [[' '] * COLS for _ in range(ROWS)]
        self.r, self.c = 0, 0
        self.log = []

    def put(self, ch):
        if ch == '\n':
            self.r = min(self.r + 1, ROWS - 1)
            return
        if ch == '\r':
            self.c = 0
            return
        if 0 <= self.r < ROWS and 0 <= self.c < COLS:
            self.grid[self.r][self.c] = ch
        self.c += 1
        if self.c >= COLS:
            self.c = 0
            self.r = min(self.r + 1, ROWS - 1)

    def csi(self, params, final):
        p = params
        n = p[0] if p else 0
        if final == 'A':
            self.r = max(0, self.r - (n or 1))
        elif final == 'B':
            self.r = min(ROWS - 1, self.r + (n or 1))
        elif final == 'C':
            self.c = min(COLS - 1, self.c + (n or 1))
        elif final == 'D':
            self.c = max(0, self.c - (n or 1))
        elif final in ('H', 'f'):
            rr = (p[0] or 1) - 1 if len(p) >= 1 else 0
            cc = (p[1] or 1) - 1 if len(p) >= 2 else 0
            self.r = max(0, min(ROWS - 1, rr))
            self.c = max(0, min(COLS - 1, cc))
        elif final == 'J':
            if n == 2:
                self.grid = [[' '] * COLS for _ in range(ROWS)]
        elif final == 'K':
            if 0 <= self.r < ROWS:
                for i in range(self.c, COLS):
                    self.grid[self.r][i] = ' '
        elif final == 'm':
            pass  # 颜色忽略
        elif final in ('h', 'l'):
            pass

    def render(self):
        lines = [''.join(row).rstrip() for row in self.grid]
        while lines and not lines[-1]:
            lines.pop()
        return lines


class FakeTerm:
    """解析子进程输出，维护虚拟屏幕，自动应答 DSR 查询。"""

    def __init__(self, master):
        self.m = master
        self.scr = Screen()
        self.buf = bytearray()
        self.dsr_count = 0
        self.apc_count = 0
        self.queries = []

    def _answer(self, data: bytes):
        os.write(self.m, data)

    def feed(self, data: bytes):
        self.buf += data
        b = self.buf
        i = 0
        out_text = bytearray()
        while i < len(b):
            ch = b[i]
            if ch == 0x1B:
                if out_text:
                    self._text(bytes(out_text))
                    out_text = bytearray()
                # 尝试解析完整序列
                if i + 1 >= len(b):
                    break
                n1 = b[i + 1]
                if n1 == ord('['):
                    m = re.match(rb'\x1b\[([0-9;?]*)([@-~])', bytes(b[i:i + 32]))
                    if not m:
                        break  # 不完整，等更多数据
                    raw_params = m.group(1).lstrip(b'?')
                    params = [int(x) if x else 0 for x in raw_params.split(b';')] if raw_params else []
                    final = chr(m.group(2)[0])
                    seq = m.group(0)
                    i += len(seq)
                    if final == 'n' and params == [6]:
                        self.dsr_count += 1
                        self.queries.append(('DSR-6n', self.scr.r + 1, self.scr.c + 1))
                        self._answer(f'\x1b[{self.scr.r + 1};{self.scr.c + 1}R'.encode())
                    elif final == 't' and params == [14]:
                        self.queries.append(('WIN-14t', None, None))
                        self._answer(f'\x1b[4;{ROWS * CELL_H};{COLS * CELL_W}t'.encode())
                    elif final == 't' and params == [18]:
                        self.queries.append(('CELL-18t', None, None))
                        self._answer(f'\x1b[8;{CELL_H};{CELL_W}t'.encode())
                    else:
                        self.scr.csi(params, final)
                    continue
                elif n1 in (ord('P'), ord('X'), ord('^'), ord('_')):
                    # DCS/SOS/PM/APC：找 ST
                    j = b.find(b'\x1b\\', i + 2)
                    k = b.find(b'\x07', i + 2)
                    ends = [x for x in (j, k) if x != -1]
                    if not ends:
                        break
                    e = min(ends) + (2 if b[min(ends)] == 0x1B else 1)
                    seq = bytes(b[i:e])
                    i = e
                    if n1 == ord('_'):
                        self.apc_count += 1
                        self._kitty(seq)
                    continue
                elif n1 == ord(']'):
                    j = b.find(b'\x07', i + 2)
                    k = b.find(b'\x1b\\', i + 2)
                    ends = [x for x in (j, k) if x != -1]
                    if not ends:
                        break
                    e = min(ends) + (2 if b[min(ends)] == 0x1B else 1)
                    i = e
                    continue
                elif n1 in (ord('7'), ord('8'), ord('M'), ord('c'), ord('='), ord('>')):
                    i += 2
                    continue
                else:
                    i += 2
                    continue
            else:
                out_text.append(ch)
                i += 1
        if out_text:
            self._text(bytes(out_text))
        self.buf = b[i:]  # 保留未处理完的残缺序列，等下批数据

    def _text(self, data: bytes):
        s = data.decode('utf-8', 'replace')
        for ch in s:
            if ch == '\t':
                for _ in range(8 - (self.scr.c % 8)):
                    self.scr.put(' ')
            else:
                self.scr.put(ch)

    def _kitty(self, seq: bytes):
        # 解析 a=T 显示：占用行数 = 图片高/格高
        m = re.search(rb'c=(\d+)', seq)
        cols = int(m.group(1)) if m else 30
        # t=f 文件路径模式：用已知 logo 尺寸
        rows = max(1, round(LOGO_H * (cols * CELL_W / LOGO_W) / CELL_H))
        self.scr.log.append(f'kitty-image cols={cols} rows={rows} at r={self.scr.r}')
        for _ in range(rows):
            self.scr.put('\n')


def run_case(mode):
    mfd, sfd = pty.openpty()
    ws = struct.pack('HHHH', ROWS, COLS, COLS * CELL_W, ROWS * CELL_H)
    fcntl.ioctl(sfd, termios.TIOCSWINSZ, ws)
    env = dict(os.environ, TERM='xterm-kitty')
    if mode == 'direct':
        cmd = ['fastfetch']
    else:
        cmd = ['nyacat', '-S', '42', '-c', 'fastfetch']
    p = subprocess.Popen(cmd, stdin=sfd, stdout=sfd, stderr=sfd, env=env,
                         close_fds=True)
    os.close(sfd)
    ft = FakeTerm(mfd)
    import time
    deadline = time.time() + 25
    while time.time() < deadline:
        r, _, _ = select.select([mfd], [], [], 0.2)
        if r:
            try:
                data = os.read(mfd, 65536)
            except OSError:
                break
            if not data:
                break
            ft.feed(data)
        if p.poll() is not None:
            # 排空
            for _ in range(10):
                r2, _, _ = select.select([mfd], [], [], 0.1)
                if not r2:
                    break
                try:
                    data = os.read(mfd, 65536)
                except OSError:
                    break
                if not data:
                    break
                ft.feed(data)
            break
    p.wait()
    os.close(mfd)
    return ft


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else '--proxy'
    mode = 'direct' if mode == '--direct' else 'proxy'
    ft = run_case(mode)
    print(f'=== mode={mode} ===')
    print(f'DSR queries answered: {ft.dsr_count}, kitty images: {ft.apc_count}')
    for q in ft.queries:
        print('  query:', q)
    for l in ft.scr.log:
        print('  ' + l)
    print('--- screen ---')
    for line in ft.scr.render():
        print(line)


if __name__ == '__main__':
    main()
