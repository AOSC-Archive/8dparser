use std::collections::HashMap;

use nom::error::ParseError;
use thiserror::Error;

mod parser;

#[derive(Error, Debug)]
pub enum EightDParseError<E> {
    #[error(transparent)]
    ParseError(#[from] nom::Err<E>),
    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),
}

#[derive(Debug, PartialEq)]
pub enum Item {
    OneLine(String),
    MultiLine(Vec<String>),
}

type NomParseItem<'a> = Vec<(&'a [u8], (&'a [u8], Vec<u8>))>;

pub fn parse_one<'a, E: ParseError<&'a [u8]>>(
    s: &'a str,
) -> Result<HashMap<String, Item>, EightDParseError<E>>
where
    EightDParseError<E>: From<nom::Err<nom::error::Error<&'a [u8]>>>,
{
    let (_, parse_v) = parser::single_package(s.as_bytes())?;
    let result = to_map(parse_v)?;

    Ok(result)
}

pub fn parse_multi<'a, E: ParseError<&'a [u8]>>(
    s: &'a str,
) -> Result<Vec<HashMap<String, Item>>, EightDParseError<E>>
where
    EightDParseError<E>: From<nom::Err<nom::error::Error<&'a [u8]>>>,
{
    let (_, parse_v) = parser::multi_package(s.as_bytes())?;

    let mut result = vec![];

    for i in parse_v {
        result.push(to_map(i)?);
    }

    Ok(result)
}

fn to_map<'a, E: ParseError<&'a [u8]>>(
    parse_v: NomParseItem,
) -> Result<HashMap<String, Item>, EightDParseError<E>> {
    let mut result = HashMap::new();
    for (k, v) in parse_v {
        let (one, multi) = v;
        let k = std::str::from_utf8(k)?.to_string();

        if one.is_empty() {
            let multi = std::str::from_utf8(&multi)?;
            let multi = multi.split("\n").map(|x| x.to_string()).collect();

            result.insert(k, Item::MultiLine(multi));
            continue;
        }

        result.insert(k, Item::OneLine(std::str::from_utf8(&one)?.to_string()));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Read, process::Command};

    use crate::{parse_multi, parse_one, Item};

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
}
