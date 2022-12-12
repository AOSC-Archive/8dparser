# 8dparser
Dpkg info parser 

## Usage

Add line to `dependencies` in `cargo.toml`:

```
eight-deep-parser = "0.1"
```

And try to parse one package:

```rust
use std::process::Command;
use eight_deep_parser::{parse_multi, parse_one, Item};

fn main() {
    let command = Command::new("dpkg")
        .arg("-s")
        .arg("plasma-workspace")
        .output()
        .unwrap();

    let stdout = command.stdout;
    let r = parse_one(std::str::from_utf8(&stdout).unwrap()).unwrap();

    assert_eq!(
        r.get("Package").unwrap(),
        &Item::OneLine("plasma-workspace".to_string())
    );
}
```

And try to parse multi package:

```rust
use std::{fs, io::Read, process::Command};
use eight_deep_parser::{parse_multi, Item};

fn main() {
    let dir = fs::read_dir("/var/lib/apt/lists").unwrap();

    for i in dir.flatten() {
      if !i.file_name().to_str().unwrap().ends_with("_Packages") {
          continue;
      }

      let mut f = std::fs::File::open(i.path()).unwrap();
      let mut buf = Vec::new();
      f.read_to_end(&mut buf).unwrap();

      let r = parse_multi(std::str::from_utf8(&buf).unwrap());

      assert!(r.is_ok())
    }
}
```
