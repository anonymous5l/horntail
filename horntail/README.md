Simple parse maplestory resource file lib supported `wz` & `ms` file extension.

## Extra

more extension feature

#### Example

use cache load `Curse.img` collection

```rust
fn main() {
    let entry = Entry::from_path(
        "<Your Resource Path>/Base",
        MapleTableNone.into_boxed(), // your resource cipher
        MapleVersion::from(79), // your resource version
        false,
    )
        .unwrap_or_else(|e| panic!("load: {e}"))
        .into_cache();

    let curse_strs = entry.get_by_path_exact("Etc/Curse.img").to::<Vec<String>>();
    println!("{:?}", curse_strs);
}
```
