use num::{rational::BigRational, Zero};
use regex::Regex;
use serde::{
    de::{self, MapAccess, Unexpected, Visitor},
    Deserializer, Serializer,
};
use serde_derive::Serialize;
use std::{collections::BTreeMap, fmt, marker::PhantomData};

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

struct RationalVisitor;

pub fn parse_mixed_number(number: &str) -> Result<BigRational, String> {
    use std::str::FromStr;

    let mixed_number = Regex::new(r"^((-)?(\d+)( (\d+/\d+))?|(-?\d+/\d+))$").unwrap();
    match mixed_number.captures(number) {
        Some(groups) => {
            let mut result = BigRational::from_str("0").unwrap();
            if let Some(x) = groups.get(3) {
                result = result + BigRational::from_str(x.as_str()).unwrap();
            }
            if let Some(x) = groups.get(5) {
                result = result + BigRational::from_str(x.as_str()).unwrap();
            }
            if let Some(x) = groups.get(6) {
                result = result + BigRational::from_str(x.as_str()).unwrap();
            }
            if let Some(_) = groups.get(2) {
                result = -result;
            }
            Ok(result)
        }
        None => Err("Not a valid mixed number".to_string()),
    }
}

impl<'de> Visitor<'de> for RationalVisitor {
    type Value = BigRational;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a number or string containing a rational number")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        parse_mixed_number(s).map_err(|_| de::Error::invalid_value(Unexpected::Str(s), &self))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(BigRational::from_integer(v.into()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(BigRational::from_integer(v.into()))
    }
}

fn deserialize_rational<'de, D>(de: D) -> Result<BigRational, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_any(RationalVisitor)
}

fn serialize_rational<S>(x: &BigRational, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(&format!("{}/{}", x.numer(), x.denom()))
}

struct MyMapVisitor {
    marker: PhantomData<fn() -> BTreeMap<String, BigRational>>,
}

impl MyMapVisitor {
    fn new() -> Self {
        MyMapVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de> Visitor<'de> for MyMapVisitor {
    // The type that our Visitor is going to produce.
    type Value = Vec<(String, BigRational)>;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a very special map")
    }

    // Deserialize MyMap from an abstract "map" provided by the
    // Deserializer. The MapAccess input is a callback provided by
    // the Deserializer to let us see each entry in the map.
    fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        #[derive(serde_derive::Deserialize, Debug)]
        struct T(#[serde(deserialize_with = "deserialize_rational")] BigRational);

        let mut v = Vec::with_capacity(access.size_hint().unwrap_or(0));

        // While there are entries remaining in the input, add them
        // into our map.
        while let Some((key, value)) = access.next_entry()? {
            let value: T = value;
            v.push((key, value.0));
        }

        Ok(v)
    }
}

fn deserialize_mymap<'de, D>(de: D) -> Result<Vec<(String, BigRational)>, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_map(MyMapVisitor::new())
}

#[derive(serde_derive::Deserialize, Debug)]
struct Transaction {
    // deserialize_with et eller annet!!!
    #[serde(deserialize_with = "deserialize_mymap")]
    credits: Vec<(String, BigRational)>,
    // deserialize_with et eller annet!!!
    #[serde(deserialize_with = "deserialize_mymap", rename = "debets")]
    debits: Vec<(String, BigRational)>,
}

#[derive(serde_derive::Deserialize, Debug)]
struct TransactionDocument {
    // _id: String,
    // _rev: String,
    transaction: Transaction,
    // meta: Meta,
}

#[derive(serde_derive::Deserialize, Debug)]
struct Row {
    // id: String,
    // key: String,
    value: TransactionDocument,
}

#[derive(serde_derive::Deserialize, Debug)]
struct AllDocs {
    // total_rows: u32,
    // offset: u32,
    rows: Vec<Row>,
}

fn balances(docs: AllDocs) -> BTreeMap<String, BigRational> {
    let mut balances: BTreeMap<String, BigRational> = docs
        .rows
        .into_iter()
        .flat_map(|row| {
            let credits = row
                .value
                .transaction
                .credits
                .into_iter()
                .map(|(account, value)| (account, value));
            let debits = row
                .value
                .transaction
                .debits
                .into_iter()
                .map(|(account, value)| (account, -value));
            credits.chain(debits)
        })
        .fold(BTreeMap::default(), |mut balances, (account, value)| {
            *balances.entry(account).or_insert_with(num::Zero::zero) += value;
            balances
        });

    balances.retain(|_, v| !v.is_zero());

    balances
}

#[derive(Serialize)]
struct BalancesRow {
    key: String,
    #[serde(serialize_with = "serialize_rational")]
    value: BigRational,
}

#[derive(Serialize)]
struct BalancesView {
    rows: Vec<BalancesRow>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let all_docs: AllDocs = {
        let stdin = std::io::stdin();
        let stdin_lock = stdin.lock();
        serde_json::from_reader(stdin_lock)
    }?;

    let balances = balances(all_docs);
    println!(
        "{}",
        serde_json::to_string(&BalancesView {
            rows: balances
                .into_iter()
                .map(|(key, value)| BalancesRow { key, value })
                .collect()
        })
        .unwrap()
    );

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
    fn rational_from_number_large() {
        let s: T = serde_json::from_str("321").unwrap();
        assert_eq!(s.0, BigRational::from_integer(321.into()));
    }

    #[test]
    fn rational_from_string_whole() {
        let s: T = serde_json::from_str("\"321\"").unwrap();
        assert_eq!(s.0, BigRational::from_integer(321.into()));
    }

    #[test]
    fn rational_from_string_fractional() {
        let s: T = serde_json::from_str("\"5/9\"").unwrap();
        assert_eq!(s.0, BigRational::new(5.into(), 9.into()));
    }

    #[test]
    fn calculate_balances() {
        let all_docs: AllDocs = serde_json::from_str(include_str!("test1.json")).unwrap();

        let balances = balances(all_docs);

        assert_eq!(
            balances.get("EEE"),
            Some(&BigRational::from_integer(num::BigInt::from(-88)))
        );

        assert_eq!(
            balances.get("AAA"),
            Some(&BigRational::from_integer(num::BigInt::from(115 - 85)))
        );
    }
}
