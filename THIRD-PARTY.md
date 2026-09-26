# Third-party notices

`rscat` is distributed under the MIT License (see [`LICENSE`](LICENSE)).

It additionally builds on, and is compatible with, the work listed below. Those
parts remain under their own licenses, reproduced here in full as those licenses
require.

---

## lolcat

- **Project:** [lolcat](https://github.com/busyloop/lolcat) — the Ruby original
  that `rscat` is a compatible replacement for
- **Version referenced:** 100.0.1
- **Copyright:** Copyright (c) 2016, moe@busyloop.net
- **License:** BSD 3-Clause

### What `rscat` takes from lolcat

| Item | Nature | Where |
|---|---|---|
| Rainbow colour algorithm | Rust port of the formula `rainbow(freq, i)` (three sin waves, phase-shifted by 2π/3, truncated to 0–255) from lolcat 100.0.1 `lib/lolcat/lol.rb` | `src/rainbow.rs` |
| 256-colour quantisation | Rust port of the `Paint` gem's `rgb_to_256` (greyscale ramp + 6×6×6 cube), as used by lolcat | `src/rainbow.rs` |
| Colour-pairing model | Byte-for-byte compatible output — escape runs plus single characters, each pair reset with `\e[39m` | `src/filter.rs` |
| `ass/nom.jpg` | **Verbatim copy** of the artwork shipped in lolcat's repository (`ass/nom.jpg`), byte-identical (MD5 `124e2a2e993e412c8a0067ce6f48a6dc`) | `ass/nom.jpg` |

The colour code is therefore a **derivative work of lolcat** and remains subject
to the BSD 3-Clause terms below, as does the copied artwork. `rscat`'s own
additions — the escape-stream filter, image passthrough, PTY run/session modes,
shell integration and packaging — are original and covered by the MIT License.

### License text

```
Copyright (c) 2016, moe@busyloop.net
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of the lolcat nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

---

## Rust dependencies

`rscat` links only two crates directly; both are permissively licensed and
compatible with the MIT License. Their full license texts are available through
[crates.io](https://crates.io) and are reproduced in the packages published to
the [Releases page](../../releases) where the format allows.

| Crate | License |
|---|---|
| [`image`](https://crates.io/crates/image) | MIT OR Apache-2.0 |
| [`libc`](https://crates.io/crates/libc) | MIT OR Apache-2.0 |

---

## Notes for redistributors

If you redistribute `rscat` — in source or in binary form, modified or not —
the BSD 3-Clause terms above require that you carry:

1. the copyright notice from this file,
2. the two lists of conditions, and
3. the disclaimer,

along with the distribution. Keeping this file (`THIRD-PARTY.md`) next to the
binary or inside the package satisfies that requirement; so does including it in
your own documentation.

The non-endorsement clause additionally means you may not use lolcat's name (or
the names of its contributors) to promote a derived product without prior
written permission.
