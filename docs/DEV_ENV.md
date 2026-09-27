# Dev environment — building the engine in this container

> Purpose: kill the rebuild circles. Sessions repeatedly lost the Rust toolchain and
> rediscovered the FFmpeg/libav build recipe. This file is the canonical recipe
> (verified 2026-09-27, unprivileged user, Debian trixie container).

## 1. Rust toolchain (env resets wipe it)

```bash
curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
export PATH="$HOME/.cargo/bin:$PATH"
rustup component add clippy rustfmt
```

## 2. FFmpeg dev libraries WITHOUT root (ove-decode needs libav* headers)

Runtime libs exist (`/usr/bin/ffmpeg` 7.1.5, `libavutil.so.59`, `libavcodec.so.61`)
but headers/.pc do not, and `apt-get install` needs root. Unprivileged recipe:

```bash
mkdir -p ~/.local/debs && cd ~/.local/debs
apt-get download libavutil-dev libavcodec-dev libavformat-dev \
                libswscale-dev libswresample-dev libavfilter-dev
for d in *.deb; do dpkg -x "$d" ~/.local/; done          # -> ~/.local/usr/...
# .pc files hardcode prefix=/usr — repoint:
cd ~/.local/usr/lib/x86_64-linux-gnu/pkgconfig
sed -i 's|^prefix=/usr$|prefix='"$HOME"'/.local/usr|; s|/usr/include/x86_64-linux-gnu|'"$HOME"'/.local/usr/include/x86_64-linux-gnu|g; s|/usr/lib/x86_64-linux-gnu|'"$HOME"'/.local/usr/lib/x86_64-linux-gnu|g' *.pc
# linker must use SHARED system libs (static .a drag in libva symbols):
cd ~/.local/usr/lib/x86_64-linux-gnu
rm -f libav*.a libsw*.a
ln -sf /usr/lib/x86_64-linux-gnu/libavutil.so.59      libavutil.so
ln -sf /usr/lib/x86_64-linux-gnu/libavcodec.so.61     libavcodec.so
ln -sf /usr/lib/x86_64-linux-gnu/libavformat.so.61    libavformat.so
ln -sf /usr/lib/x86_64-linux-gnu/libswscale.so.8      libswscale.so
ln -sf /usr/lib/x86_64-linux-gnu/libswresample.so.5   libswresample.so
ln -sf /usr/lib/x86_64-linux-gnu/libavfilter.so.10    libavfilter.so
```

## 3. libclang for bindgen (ffmpeg-sys-next builds bindings)

```bash
cd ~/.local/debs
apt-get download libclang1-19 libclang-common-19-dev
dpkg -x libclang1-19*.deb x1 && dpkg -x libclang-common-19-dev*.deb x2
mkdir -p ~/.local/llvm/lib
cp x1/usr/lib/x86_64-linux-gnu/libclang-19.so.19.1.0 ~/.local/llvm/lib/libclang.so
cp -r x2/usr/lib/llvm-19/lib/19 ~/.local/llvm/lib/19          # resource headers
mkdir -p ~/.local/llvm/lib/clang && cp -r ~/.local/llvm/lib/19 ~/.local/llvm/lib/clang/19
```

No gcc in the container -> bindgen's `include_next <limits.h>` chain breaks.
Feed clang's own headers as a system include:

## 4. Environment block (copy-paste before any cargo command)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export PKG_CONFIG_PATH="$HOME/.local/usr/lib/x86_64-linux-gnu/pkgconfig"
export LIBCLANG_PATH="$HOME/.local/llvm/lib"
export BINDGEN_EXTRA_CLANG_ARGS="-isystem $HOME/.local/llvm/lib/clang/19/include -isystem /usr/include/x86_64-linux-gnu"
```

## 5. Verify (expect 60/60 green)

```bash
cd engine && cargo test --release
cargo fmt --check && cargo clippy --release --workspace
```

## Notes

* E-006a bench crate: `cargo run --release` from `scripts/experiments/E-006a_ipc_crate/`.
* GitHub CI (ubuntu-latest) installs these system deps natively — this file is only
  for THIS container's unprivileged workflow.
* Standing security rule: GitHub PATs are used for `git push` only, never written
  to any repo file, and must be rotated after appearing in any chat context.
