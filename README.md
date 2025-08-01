TUI base maplestory resource file unpack tool.

fast and lower memory usage.

## Build

```bash
cd horntail_cli
cargo build --release
```

## Usage

```bash
# detected file version generally use first result
horntail_cli probe --path <Base.wz>
# view file
horntail_cli view --path <Base.wz> --version <probe_ver> --key <ems|gms|nil>
```

## Example

### Simple load wz file

target `File.wz` is from client version `79` and cipher is `EUROPE`

```rust
fn main() {
    let file = WizetFile::new("File.wz", MapleVersion::from(79), false).expect("load");
    let opt = file.accessor_opt();
    let fd = file.source().open().expect("open");
    let mut accessor = BinaryAccessor::new(MapleTable::new(MAPLE_VECTOR_EUROPE), fd.as_ref());
    let dirs = Directories::from_accessor(opt, &mut accessor);
    dirs.into_inner().into_iter().for_each(|d| {
        println!("Directory: {}", d.name);
    });
}
```

### Simple load pack file

```rust
fn main() {
    let file = PackFile::new("File.ms").expect("load");
    let fd = file.source().open().expect("open");
    let mut accessor = BinaryAccessor::new(MapleTableNone, fd.as_ref());
    let mut entries = file.entries().expect("entries");
    for e in entries.iter() {
        println!("Entry: {:?}", e);
        let entry_image_data = e.decrypt_from(&mut accessor).expect("decrypt_from");
        let mut entry_accessor = BinaryAccessor::new(MapleTableNone, entry_image_data);
        let image = Image::from_accessor(AccessorOpt::default(), &mut entry_accessor);
        println!("{}", image.kind)
    }
}
```

## Shortcut

| Key        | Description                                  |
|------------|----------------------------------------------|
| `j`        | select next row                              |
| `k`        | select prev row                              |
| `gg`       | go to the first row of the view              |
| `G`        | go to the last row of the view               |
| `[`        | go to the prev expend row                    |
| `]`        | go to the next expend row                    |
| `Ctrl`+`f` | move screen down one page                    |
| `Ctrl`+`b` | move screen up one page                      |
| `/`        | search for pattern (case sensitive)          |
| `?`        | search backward for pattern (case sensitive) |
| `n`        | search next                                  |
| `N`        | search back                                  |
| `e`        | toggle selected row                          |
| `Ctrl`+`e` | recursive toggle selected row                |
| `esc`      | exit                                         |