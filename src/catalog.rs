use rusty_money::{Findable, FormattableCurrency, Locale, crypto, iso};
use std::sync::OnceLock;

const CRYPTO_CODES: [&str; 14] = [
    "BCH", "BSV", "BTC", "COMP", "DAI", "ETH", "LTC", "MKR", "TRX", "UNI", "USDC", "USDT", "XTZ",
    "ZEC",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CurrencyKind {
    Iso,
    Crypto,
}

impl CurrencyKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Iso => "iso",
            Self::Crypto => "crypto",
        }
    }
}

/// A process-static descriptor that lets one `PostgreSQL` type use both of
/// rusty-money's built-in currency sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Currency {
    code: &'static str,
    numeric_code: Option<&'static str>,
    exponent: u32,
    locale: Locale,
    minor_units: u64,
    name: &'static str,
    symbol: &'static str,
    symbol_first: bool,
    kind: CurrencyKind,
}

impl Currency {
    fn from_iso(currency: &'static iso::Currency) -> Self {
        Self {
            code: currency.code(),
            numeric_code: Some(currency.iso_numeric_code),
            exponent: currency.exponent,
            locale: currency.locale,
            minor_units: currency.minor_units,
            name: currency.name,
            symbol: currency.symbol,
            symbol_first: currency.symbol_first,
            kind: CurrencyKind::Iso,
        }
    }

    fn from_crypto(currency: &'static crypto::Currency) -> Self {
        Self {
            code: currency.code,
            numeric_code: None,
            exponent: currency.exponent,
            locale: currency.locale,
            minor_units: currency.minor_units,
            name: currency.name,
            symbol: currency.symbol,
            symbol_first: currency.symbol_first,
            kind: CurrencyKind::Crypto,
        }
    }

    pub(crate) const fn numeric_code(self) -> Option<&'static str> {
        self.numeric_code
    }

    pub(crate) const fn minor_units(self) -> u64 {
        self.minor_units
    }

    pub(crate) const fn name(self) -> &'static str {
        self.name
    }

    pub(crate) const fn kind(self) -> CurrencyKind {
        self.kind
    }
}

impl FormattableCurrency for Currency {
    fn to_string(&self) -> String {
        self.code.to_owned()
    }

    fn exponent(&self) -> u32 {
        self.exponent
    }

    fn code(&self) -> &'static str {
        self.code
    }

    fn locale(&self) -> Locale {
        self.locale
    }

    fn symbol(&self) -> &'static str {
        self.symbol
    }

    fn symbol_first(&self) -> bool {
        self.symbol_first
    }
}

impl Findable for Currency {
    fn find(code: &str) -> Option<&'static Self> {
        find(code)
    }
}

static CURRENCIES: OnceLock<Box<[Currency]>> = OnceLock::new();

pub(crate) fn currencies() -> &'static [Currency] {
    CURRENCIES.get_or_init(|| {
        let mut currencies = Vec::with_capacity(200);
        for first in b'A'..=b'Z' {
            for second in b'A'..=b'Z' {
                for third in b'A'..=b'Z' {
                    let bytes = [first, second, third];
                    // SAFETY: all bytes are uppercase ASCII and therefore valid UTF-8.
                    let code = unsafe { std::str::from_utf8_unchecked(&bytes) };
                    if let Some(currency) = iso::find(code) {
                        currencies.push(Currency::from_iso(currency));
                    }
                }
            }
        }
        currencies.extend(CRYPTO_CODES.into_iter().map(|code| {
            Currency::from_crypto(crypto::find(code).expect("compiled crypto code must exist"))
        }));
        currencies.sort_unstable_by_key(FormattableCurrency::code);
        assert!(
            currencies
                .windows(2)
                .all(|pair| pair[0].code() != pair[1].code()),
            "ISO and crypto catalogs must not contain duplicate codes"
        );
        currencies.into_boxed_slice()
    })
}

pub(crate) fn find(code: &str) -> Option<&'static Currency> {
    let currencies = currencies();
    currencies
        .binary_search_by(|currency| currency.code().cmp(code))
        .ok()
        .map(|index| &currencies[index])
}
