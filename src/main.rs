use num::rational::BigRational;
use serde::{
    de::{self, Unexpected, Visitor},
    Deserializer,
};
use std::{convert::TryInto, fmt};

struct RationalVisitor;

impl<'de> Visitor<'de> for RationalVisitor {
    type Value = BigRational;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a number or string containing a rational number")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
/*
pub fn parse_mixed_number(number : &str) -> Result<Rational, String> {
	let mixed_number = Regex::new(r"^((-)?(\d+)( (\d+/\d+))?|(-?\d+/\d+))$").unwrap();
	match mixed_number.captures(number) {
		Some(groups) => {
			let mut result = BR::from_str("0").unwrap();
			if let Some(x) = groups.at(3) { result = result + BR::from_str(x).unwrap(); }
			if let Some(x) = groups.at(5) { result = result + BR::from_str(x).unwrap(); }
			if let Some(x) = groups.at(6) { result = result + BR::from_str(x).unwrap(); }
			if let Some(_) = groups.at(2) { result = -result; }
			Ok(Rational(result))
		},
		None => Err("Not a valid mixed number".to_string())
	}
}
 */

        s.parse()
            .map_err(|_| de::Error::invalid_value(Unexpected::Str(s), &self))
            .map(|i| BigRational::from_integer(i))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.try_into()
            .map_err(|_| de::Error::invalid_value(Unexpected::Signed(v), &self))
            .map(|i: i64| BigRational::from_integer(i.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.try_into()
            .map_err(|_| de::Error::invalid_value(Unexpected::Unsigned(v), &self))
            .map(|i: u64| BigRational::from_integer(i.into()))
    }
}

fn deserialize_rational<'de, D>(de: D) -> Result<BigRational, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_any(RationalVisitor)
}

#[derive(serde_derive::Deserialize, Debug)]
struct Transaction {
    name: String,

    #[serde(deserialize_with = "deserialize_rational")]
    value: BigRational,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s: Transaction = serde_json::from_str("{\"name\":\"kake\",\"value\":\"3217897897897897897897897897897879789789789789789789789789\"}")?;
    println!("Hello, {:?}!", s);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(serde_derive::Deserialize, Debug)]
    struct T(#[serde(deserialize_with = "deserialize_rational")] BigRational);

    #[test]
    fn rational_from_number() {
        let s: T = serde_json::from_str("321").unwrap();
        assert_eq!(s.0, BigRational::from_integer(321.into()));
    }

    #[test]
    fn rational_from_string_whole() {
        let s: T = serde_json::from_str("\"321\"").unwrap();
        assert_eq!(s.0, BigRational::from_integer(321.into()));
    }

    // #[test]
    // fn rational_from_string_fractional() {
    //     let s: T = serde_json::from_str("\"5/9\"").unwrap();
    //     assert_eq!(s.0, Rational(5, 9));
    // }
}
