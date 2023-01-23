use nom::{
    bytes::complete::{tag, take_until},
    character::complete::{char, multispace0, space0},
    combinator::{map, verify},
    multi::{many0, many1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

type KeyValueResult<'a> = IResult<&'a [u8], (&'a [u8], (&'a [u8], Vec<u8>))>;
type SinglePackageResult<'a> = IResult<&'a [u8], Vec<(&'a [u8], (&'a [u8], Vec<u8>))>>;
type MultiPackageResult<'a> = IResult<&'a [u8], Vec<Vec<(&'a [u8], (&'a [u8], Vec<u8>))>>>;

#[inline]
fn key_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    verify(handle_key, |input: &[u8]| {
        if !input.is_empty() {
            input[0] != b'\n'
        } else {
            false
        }
    })(input)
}

#[inline]
fn handle_key(input: &[u8]) -> IResult<&[u8], &[u8]> {
    preceded(handle_key_name, take_until(":"))(input)
}

#[inline]
fn handle_key_name(input: &[u8]) -> IResult<&[u8], ()> {
    map(
        many0(delimited(tag(" "), take_until("\n"), tag("\n"))),
        |_| (),
    )(input)
}

#[inline]
fn separator(input: &[u8]) -> IResult<&[u8], ()> {
    map(tuple((char(':'), space0)), |_| ())(input)
}

#[inline]
fn key_value(input: &[u8]) -> KeyValueResult {
    separated_pair(key_name, separator, value_field)(input)
}

#[inline]
fn value_field(input: &[u8]) -> IResult<&[u8], (&[u8], Vec<u8>)> {
    tuple((single_line, multi_to_one))(input)
}

#[inline]
fn single_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_until("\n"), tag("\n"))(input)
}

#[inline]
fn multi_line_single(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag(" "), take_until("\n"), tag("\n"))(input)
}

#[inline]
fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    map(
        many0(delimited(tag("--"), take_until("-\n"), tag("-\n"))),
        |_| (),
    )(input)
}

#[inline]
fn multi_line(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    many0(multi_line_single)(input)
}

fn multi_to_one(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let ctx = multi_line(input)?;

    let mut s = String::new();
    for (i, c) in ctx.1.iter().enumerate() {
        s += std::str::from_utf8(c).unwrap();
        if i != ctx.1.len() - 1 {
            s += "\n";
        }
    }

    let s = s.as_bytes().to_vec();

    Ok((input, s))
}

#[inline]
pub fn single_package(input: &[u8]) -> SinglePackageResult {
    terminated(many1(key_value), multispace0)(input)
}

#[inline]
pub fn multi_package(input: &[u8]) -> MultiPackageResult {
    many1(preceded(comment, single_package))(input)
}

#[test]
fn test_single_line() {
    let test = b"zsync\n";

    let r = single_line(test);

    assert_eq!(r, Ok((&b""[..], &b"zsync"[..])));
}

#[test]
fn test_value_field() {
    let test = b"zsync\n";
    let r = value_field(test);

    assert_eq!(r, Ok((&b""[..], (&b"zsync"[..], b"".to_vec()))));

    let test = b"\n a\n b\n c\n";
    let r = value_field(test);

    assert_eq!(
        r,
        Ok((&b" a\n b\n c\n"[..], (&b""[..], b"a\nb\nc".to_vec())))
    );
}

#[test]
fn test_multi_line() {
    let test = b" a\n b\n c\nD: E";
    let r = multi_line(test);

    assert_eq!(r, Ok((&b"D: E"[..], vec![&b"a"[..], &b"b"[..], &b"c"[..]])))
}

#[test]
fn test_multi_line_to_one() {
    let test = b" c\n d\n e\n";

    let r = multi_to_one(test);

    assert_eq!(r, Ok((&b" c\n d\n e\n"[..], b"c\nd\ne".to_vec())))
}

#[test]
fn test_handle_key() {
    let test = b" b\n c\nD: E";

    let r = handle_key(test);

    assert_eq!(r, Ok((&b": E"[..], &b"D"[..])))
}

#[test]
fn test_key_name() {
    let test = b"Package: zsync\n";

    let r = key_name(test);

    assert_eq!(r, Ok((&b": zsync\n"[..], &b"Package"[..])))
}

#[test]
fn test_key_value() {
    let test = b"Package: zsync\n";
    let r = key_value(test);

    assert_eq!(
        r,
        Ok((&b""[..], (&b"Package"[..], (&b"zsync"[..], b"".to_vec()))))
    );

    let test = b"c:\n d\n e\n";

    let r = key_value(test);

    assert_eq!(
        r,
        Ok((&b" d\n e\n"[..], (&b"c"[..], (&b""[..], b"d\ne".to_vec()))))
    );
}

#[test]
fn test_single_package() {
    let test = b"Package: a\nMulti:\n a\n b\n c\nD: E\n";

    let r = single_package(test);

    assert_eq!(
        r,
        Ok((
            &b""[..],
            vec![
                (&b"Package"[..], (&b"a"[..], b"".to_vec())),
                (&b"Multi"[..], (&b""[..], b"a\nb\nc".to_vec())),
                (&b"D"[..], (&b"E"[..], b"".to_vec())),
            ]
        ))
    )
}

#[test]
fn test_comment() {
    let test = b"---abc---\n";

    let c = comment(test);

    assert!(c.is_ok())
}

