# Jug (program manager)

Jug is implemented in the apps repository's `cpp/jug`, using libc++r2. Open the
**Jug** desktop icon to browse the CDN catalog, download ELF programs, inspect
their checksums and running PIDs, and restart their instances. The window is
hosted by `JUG.ELF` in a separate process, with Memento forwarding its input and
displaying its frames.

The default root is `https://cdn.vxn.dev/jug`, with a `sums.txt` catalog. The
ISO build installs `/mnt/tar/opt/jug/jug.cfg` (or `/mnt/iso/opt/jug/jug.cfg` on
the CD). An editable `/mnt/fat/JUG.CFG` takes precedence. Configuration keys
are `repo`, `list` and `insecure`; HTTPS checks certificates by default using
Memento's CA bundle. Run the `eth` driver before fetching packages.

On the console:

```text
fg jug update
fg jug list
fg jug install tnt
fg jug upgrade
fg jug restart tnt
fg jug remove tnt
```

Downloaded programs live at `/mnt/tmp/jug/TNT.ELF`. The kernel searches them
before the working directory and the boot medium's bin directories. Jug checks
the complete SHA-256, optional byte size and ELF structure, then stages and
reads back the download before replacing the old copy. A light registry records
each program's checksum; the catalog shows its publication timestamp, name,
size and checksum. The RAM disk and downloaded updates are reset at reboot.

Updates affect subsequent launches. **Restart** stops every running instance
and starts it with its original arguments, obtained through syscall `0x41`.
Jug leaves an instance running if its command line cannot be recovered and
protects Jug itself and its Memento host. **Remove** restores the shipped copy
for subsequent launches.

In the window, **Tab** cycles between the program list and the six bottom
buttons; **Shift+Tab** goes backward. The focused button is highlighted.
**Enter** or **Space** activates it, and **Left/Right** move between buttons.
**Up/Down**, **Page Up/Down**, **Home/End**, or clicking a row return focus to
the list, where **Enter** downloads the selected program. Restart requires
**Y** to confirm.

To publish over SSH from the developer environment, use
`utils/jug-publish.sh --host user@swarm-node --all` in the kernel repository.
It uploads the staged `iso/bin` binaries to immutable version URLs, generates
`sums.txt`, and atomically switches the catalog after verifying the files.
Pass a single ELF filename to update one program while retaining the others.
See [Publishing Jug programs](jug-publishing.md) for SSH-agent setup, Nginx
examples, recovery, and the default CephFS destination.

For a manually managed server, `make sums DIR=/path/to/jug` in `cpp/jug`
generates a catalog with `SHA-256 bytes path` lines; paths are relative to the
list. Plain `sha256sum` output is accepted too. Set `list = list.txt` to use
that filename instead. See the apps repository's `cpp/jug/README.md` for
configuration, window shortcuts and tests.
