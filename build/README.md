# rscat 预编译包 / Prebuilt packages

## 本目录已有的包 (x86_64 Linux)

| 文件 | 安装命令 |
|---|---|
| `rscat-1.1.6-5-2-x86_64.pkg.tar.gz` | `sudo pacman -U rscat-1.1.6-5-2-x86_64.pkg.tar.gz` |
| `rscat_1.1.6-5-2_amd64.deb` | `sudo apt install ./rscat_1.1.6-5-2_amd64.deb` |
| `rscat-1.1.6-5-2.x86_64.rpm` | `sudo rpm -Uvh rscat-1.1.6-5-2.x86_64.rpm` |
| `freebsd/rscat-1.1.6-5-freebsd-amd64.pkg` | FreeBSD: `pkg add ./rscat-1.1.6-5-freebsd-amd64.pkg` |
| `freebsd/rscat-1.1.6-5-freebsd-arm64.pkg` | FreeBSD arm64: `pkg add ./…` |

## FreeBSD 交叉编译配方 / FreeBSD cross-build recipe(无需 FreeBSD 实机)

```bash
rustup target add x86_64-unknown-freebsd        # std 组件(rustup 提供)
# 目标系统 libc/crt 从 base.txz 提取(amd64 约 205MB;arm64 换 arm64/aarch64 路径):
curl -LO https://download.freebsd.org/ftp/releases/amd64/14.3-RELEASE/base.txz
mkdir sysroot && tar xJf base.txz -C sysroot ./usr/lib ./lib
# 运行库在 /lib(libc.so.7),crt 在 /usr/lib(Scrt1.o 等)——两个目录都要
# 链接器包装:clang --target 走 FreeBSD startfiles,lld 链接:
cat > link.sh <<'EOT'
#!/bin/sh
exec clang --target=x86_64-unknown-freebsd14.3 --sysroot=<sysroot绝对路径> -fuse-ld=lld "$@"
EOT
chmod +x link.sh
cargo build --release --target x86_64-unknown-freebsd \
  --config 'target.x86_64-unknown-freebsd.linker="<绝对路径>/link.sh"'
# 打包:tar(--xz)内含 +MANIFEST(UCL,files 表带 sha256)与 usr/local 树
```

注意:必须用 rustup 的 rustc(确保 `~/.cargo/bin` 在 PATH 最前)。系统 rustc 的
sysroot 里没有 FreeBSD std,会报 E0463。

## 目录结构 / Layout

```
build/
  x86_64/           Linux x86_64 三格式(pkg.tar.gz / deb / rpm)
  aarch64/          Linux aarch64 三格式(有 rustup musl 目标时本机可产)
  freebsd/          FreeBSD 14.3 双架构 .pkg(amd64 / arm64,本机交叉编译)
  PROMPT-macos.txt      交给 macOS 机器上 Agent 的构建提示词
  PROMPT-windows.txt    交给 Windows 机器上 Agent 的构建提示词
```

## 从源码重建 / Build from source

```bash
# 任意平台(Linux/macOS/FreeBSD)
cargo build --release
install -Dm755 target/release/rscat ~/.local/bin/rscat

# Linux aarch64(在 x86_64 机器上交叉编译,静态二进制,免目标机依赖)
rustup target add aarch64-unknown-linux-musl
cargo build --release --target aarch64-unknown-linux-musl
# 产物: target/aarch64-unknown-linux-musl/release/rscat (静态链接,打包时
# Architecture 用 aarch64(deb) / aarch64(rpm) / aarch64(pkg))
```

## 各平台原生构建要点 / Native build notes

- **macOS**: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`;
  安装包用 `pkgbuild` + `productbuild` 出 `.pkg`。详见 `PROMPT-macos.txt`。
- **Windows**: 原生 `cargo build --release` 出 `rscat.exe`;
  安装器用 Inno Setup(`.exe`)与 WiX(`.msi`)。详见 `PROMPT-windows.txt`。
  注意:Windows 侧过滤/图片(--proto iterm,WezTerm)/--init 可用;
  `-e`/`-a` 需要 ConPTY,当前版本会多语言报错,属已知限制。
- **FreeBSD**: 已在本机交叉编译并打包(`freebsd/` 目录,14.3 amd64/arm64)。
  配方见上方 FreeBSD 交叉编译小节;也可在 FreeBSD 实机 `pkg install rust` 后直接 `cargo build --release`。

## 打包配方速查 / Packaging quick reference

- Arch `.pkg.tar.gz`: `.PKGINFO` + `.MTREE` + `usr/bin/rscat`(本仓库用 makepkg 生成)
- Debian `.deb`: `ar rcs rscat_x.y.z_amd64.deb debian-binary control.tar.gz data.tar.xz`
- RPM: `rpmbuild -bb`(spec 需 `%define debug_package %{nil}`,二进制已 strip)
