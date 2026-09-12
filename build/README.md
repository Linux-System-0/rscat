# rscat 预编译包 / Prebuilt packages

## 本目录已有的包 (x86_64 Linux)

| 文件 | 安装命令 |
|---|---|
| `rscat-0.1.0-1-x86_64.pkg.tar.gz` | `sudo pacman -U rscat-0.1.0-1-x86_64.pkg.tar.gz` |
| `rscat_0.1.0-1_amd64.deb` | `sudo apt install ./rscat_0.1.0-1_amd64.deb` |
| `rscat-0.1.0-1.x86_64.rpm` | `sudo rpm -Uvh rscat-0.1.0-1.x86_64.rpm` |

## 目录结构 / Layout

```
build/
  x86_64/           Linux x86_64 三格式(pkg.tar.gz / deb / rpm)
  aarch64/          Linux aarch64 三格式(有 rustup musl 目标时本机可产)
  PROMPT-macos.txt      交给 macOS 机器上 Agent 的构建提示词
  PROMPT-windows.txt    交给 Windows 机器上 Agent 的构建提示词
  PROMPT-freebsd.txt    交给 FreeBSD 机器上 Agent 的构建提示词
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
- **FreeBSD**: `pkg install rust` 后 `cargo build --release`;
  打包用 `pkg create` + plist。详见 `PROMPT-freebsd.txt`。

## 打包配方速查 / Packaging quick reference

- Arch `.pkg.tar.gz`: `.PKGINFO` + `.MTREE` + `usr/bin/rscat`(本仓库用 makepkg 生成)
- Debian `.deb`: `ar rcs rscat_x.y.z_amd64.deb debian-binary control.tar.gz data.tar.xz`
- RPM: `rpmbuild -bb`(spec 需 `%define debug_package %{nil}`,二进制已 strip)
