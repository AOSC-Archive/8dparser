use std::fmt::Display;

use error::Result;
pub use indexmap::IndexMap;
use thiserror::Error;

mod error;
mod parser;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Item {
    OneLine(String),
    MultiLine(Vec<String>),
}

#[derive(Debug, Error)]
pub struct NomErrorWrap {
    source: nom::Err<nom::error::Error<Vec<u8>>>,
}

impl Display for NomErrorWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(f)
    }
}

type NomParseItem<'a> = Vec<(&'a [u8], (&'a [u8], Vec<u8>))>;

/// Parse a single package:
///
/// ```rust
/// use std::process::Command;
/// use eight_deep_parser::{parse_multi, parse_one, Item};
///
/// let command = Command::new("dpkg")
///     .arg("-s")
///     .arg("plasma-workspace")
///     .output()
///     .unwrap();
///
/// let stdout = command.stdout;
///
/// let r = parse_one(std::str::from_utf8(&stdout).unwrap()).unwrap();
///
/// assert_eq!(
///     r.get("Package").unwrap(),
///     &Item::OneLine("plasma-workspace".to_string())
/// );
///```
pub fn parse_one(s: &str) -> Result<IndexMap<String, Item>> {
    let (_, parse_v) = parser::single_package(s.as_bytes())?;

    let result = to_map(parse_v)?;

    Ok(result)
}

/// Parse multi package:
/// (e.g: /var/lib/dpkg/status)
///
/// ```rust
/// use std::{fs, io::Read, process::Command};
/// use eight_deep_parser::{parse_multi, Item};
///
/// let dir = fs::read_dir("/var/lib/apt/lists").unwrap();
///
/// for i in dir.flatten() {
///     if !i.file_name().to_str().unwrap().ends_with("_Packages") {
///         continue;
///     }
///
///     let mut f = std::fs::File::open(i.path()).unwrap();
///     let mut buf = Vec::new();
///     f.read_to_end(&mut buf).unwrap();
///
///     let r = parse_multi(std::str::from_utf8(&buf).unwrap());
///
///     assert!(r.is_ok())
/// }
/// ```
pub fn parse_multi(s: &str) -> Result<Vec<IndexMap<String, Item>>> {
    if s.is_empty() {
        return Ok(Vec::new());
    }

    let (_, parse_v) = parser::multi_package(s.as_bytes())?;

    let mut result = vec![];

    for i in parse_v {
        result.push(to_map(i)?);
    }

    Ok(result)
}

fn to_map(parse_v: NomParseItem) -> Result<IndexMap<String, Item>> {
    let mut result = IndexMap::new();
    for (k, v) in parse_v {
        let (one, multi) = v;
        let k = std::str::from_utf8(k)?.to_string();

        if one.is_empty() {
            let multi = std::str::from_utf8(&multi)?;
            let multi = multi.split('\n').map(|x| x.to_string()).collect();

            result.insert(k, Item::MultiLine(multi));
            continue;
        }

        result.insert(k, Item::OneLine(std::str::from_utf8(one)?.to_string()));
    }

    Ok(result)
}

/// Parse back:
/// 
/// ```rust
/// use indexmap::IndexMap;
/// use eight_deep_parser::{parse_back, Item};
/// 
/// fn test_parse_back() {
///     let mut map = vec![];
///
///     let mut item1 = IndexMap::new();
///     item1.insert("a".to_string(), Item::OneLine("b".to_string()));
///     item1.insert(
///         "c".to_string(),
///         Item::MultiLine(vec!["a".to_string(), "b".to_string()]),
///     );
///     item1.insert("d".to_string(), Item::OneLine("e".to_string()));
///     map.push(item1);
///
///     let mut item2 = IndexMap::new();
///     item2.insert("a".to_string(), Item::OneLine("b".to_string()));
///     map.push(item2);
///
///     let s = parse_back(&map);
///
///     assert_eq!(
///         s,
///         r#"a: b
/// c:
///   a
///   b
/// d: e
///
/// a: b
///
/// "#
///     )
/// }

/// ```

pub fn parse_back(map: &[IndexMap<String, Item>]) -> String {
    let mut s = String::new();
    for i in map {
        for (k, v) in i {
            s += &format!("{}:", k);

            match v {
                Item::OneLine(v) => s += &format!(" {}\n", v),
                Item::MultiLine(v) => {
                    s += "\n";
                    for i in v {
                        s += &format!("  {}\n", i);
                    }
                }
            }
        }

        s += "\n";
    }

    s
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read, process::Command};

    use indexmap::IndexMap;

    use crate::{parse_back, parse_multi, parse_one, Item};

    #[test]
    fn parse_one_it_works() {
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

        let right = vec![
            "/etc/pam.d/kde a33459447160292012baca99cb9820b3",
            "/etc/xdg/autostart/gmenudbusmenuproxy.desktop 4bf33ab6a937c4991c0ec418bfff11a0",
            "/etc/xdg/autostart/klipper.desktop cc58958cfa37d7f4001e24e3de34abbd",
            "/etc/xdg/autostart/org.kde.plasmashell.desktop 9552c32cf4e0c3a56b2884f6b08d7c72",
            "/etc/xdg/autostart/xembedsniproxy.desktop 76011e12682833a1b4b3a01c7faac001",
            "/etc/xdg/plasmanotifyrc f9713a8fb2a4abb43e592f0c12f3fab5",
            "/etc/xdg/taskmanagerrulesrc 9df6c5d4530892fac71c219f27892f5b",
        ];

        let right = right.iter().map(|x| x.to_string()).collect::<Vec<_>>();

        assert_eq!(r.get("Conffiles").unwrap(), &Item::MultiLine(right));

        assert_eq!(
            r.get("Description").unwrap(),
            &Item::OneLine("The KDE Plasma Workspace, API and runtime libraries".to_string())
        );
    }

    #[test]
    fn parse_multi_it_works() {
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

    #[test]
    fn test_parse_back() {
        let mut map = vec![];

        let mut item1 = IndexMap::new();
        item1.insert("a".to_string(), Item::OneLine("b".to_string()));
        item1.insert(
            "c".to_string(),
            Item::MultiLine(vec!["a".to_string(), "b".to_string()]),
        );
        item1.insert("d".to_string(), Item::OneLine("e".to_string()));
        map.push(item1);

        let mut item2 = IndexMap::new();
        item2.insert("a".to_string(), Item::OneLine("b".to_string()));
        map.push(item2);

        let s = parse_back(&map);

        assert_eq!(
            s,
            r#"a: b
c:
  a
  b
d: e

a: b

"#
        )
    }
}
