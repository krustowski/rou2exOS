# rou2rexOS Rusted

Second iteration of the RoureXOS operating system, rewritten in Rust.

## linker script (multiboot2)

Used with `multiboot2` for GRUB2.

```json
	"pre-link-args": {
		"ld.lld": [
			"-n",
			"-Tlink.ld"
		]
	},
```
