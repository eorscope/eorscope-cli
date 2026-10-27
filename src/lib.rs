//! Statutory employer cost of an employee, computed from the EOR Scope
//! employer-cost dataset (CC BY 4.0) embedded in `data/`.
//!
//! The engine reproduces the one that prints the figures on eorscope.com
//! (`site/src/lib/cost-engine.ts`, `computeScenario`) and its rounding
//! (`site/src/lib/format.ts`): at each country's example salary it gives back
//! the `employer_cost_pct`, `employer_cost_pct_printed` and
//! `employer_cost_usd_month` columns of the dataset, which the Go module
//! `github.com/eorscope/employercosts` exposes as published figures.
//!
//! Cost comparison, not legal or tax advice.

use std::collections::HashMap;
use std::sync::OnceLock;

pub const SUMMARY_CSV: &str = include_str!("../data/country_summary.csv");
pub const CONTRIBUTIONS_CSV: &str = include_str!("../data/country_contributions.csv");
pub const ASSUMPTIONS_CSV: &str = include_str!("../data/country_assumptions.csv");

pub const SOURCE_LINE: &str =
    "Source: https://eorscope.com/data/ (CC BY 4.0). Cost comparison, not legal or tax advice.";

/// One row of `country_summary.csv`.
#[derive(Debug, Clone)]
pub struct Country {
    pub iso: String,
    pub name: String,
    pub region: String,
    pub currency: String,
    pub example_salary_usd: f64,
    pub example_place: String,
    /// Published percent, 4 decimals, as written in the CSV.
    pub employer_cost_pct: String,
    /// Published percent, 1 decimal, as printed on eorscope.com.
    pub employer_cost_pct_printed: String,
    pub total_declared_floor: bool,
    pub total_declared_ceiling: bool,
    pub employer_cost_usd_month: String,
    pub total_excludes: String,
    pub fx_local_per_usd: f64,
    pub fx_date: String,
    pub last_reviewed: String,
}

/// One row of `country_contributions.csv`.
#[derive(Debug, Clone)]
pub struct Line {
    pub line_type: String,
    pub id: String,
    pub name: String,
    pub rate: Option<f64>,
    pub base: String,
    pub base_factor: Option<f64>,
    pub floor: Option<f64>,
    pub cap: Option<f64>,
    pub flat: Option<f64>,
    pub min_gross: Option<f64>,
    pub max_gross: Option<f64>,
    pub extra_kind: String,
    pub extra_value: Option<f64>,
    pub in_total: bool,
    pub applies: String,
    pub notes: String,
}

struct Data {
    countries: Vec<Country>,
    lines: HashMap<String, Vec<Line>>,
    assumptions: HashMap<String, Vec<(String, f64)>>,
}

fn rows(text: &'static str) -> (HashMap<String, usize>, Vec<csv::StringRecord>) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut r = csv::Reader::from_reader(text.as_bytes());
    let idx = r
        .headers()
        .expect("csv header")
        .iter()
        .enumerate()
        .map(|(i, h)| (h.to_string(), i))
        .collect();
    let recs = r.records().map(|x| x.expect("csv record")).collect();
    (idx, recs)
}

fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let s = |idx: &HashMap<String, usize>, r: &csv::StringRecord, col: &str| -> String {
            r[*idx.get(col).unwrap_or_else(|| panic!("missing column {col}"))].to_string()
        };
        let f = |v: String| -> Option<f64> {
            if v.is_empty() {
                None
            } else {
                Some(v.parse().unwrap_or_else(|_| panic!("bad number {v:?}")))
            }
        };
        let b = |v: String| v == "true";

        let (i, recs) = rows(SUMMARY_CSV);
        let mut countries: Vec<Country> = recs
            .iter()
            .map(|r| Country {
                iso: s(&i, r, "iso"),
                name: s(&i, r, "country"),
                region: s(&i, r, "region"),
                currency: s(&i, r, "currency"),
                example_salary_usd: f(s(&i, r, "example_salary_usd")).expect("example salary"),
                example_place: s(&i, r, "example_place"),
                employer_cost_pct: s(&i, r, "employer_cost_pct"),
                employer_cost_pct_printed: s(&i, r, "employer_cost_pct_printed"),
                total_declared_floor: b(s(&i, r, "total_declared_floor")),
                total_declared_ceiling: b(s(&i, r, "total_declared_ceiling")),
                employer_cost_usd_month: s(&i, r, "employer_cost_usd_month"),
                total_excludes: s(&i, r, "total_excludes"),
                fx_local_per_usd: f(s(&i, r, "fx_local_per_usd")).expect("fx"),
                fx_date: s(&i, r, "fx_date"),
                last_reviewed: s(&i, r, "last_reviewed"),
            })
            .collect();
        countries.sort_by(|a, b| a.iso.cmp(&b.iso));

        let (i, recs) = rows(CONTRIBUTIONS_CSV);
        let mut lines: HashMap<String, Vec<Line>> = HashMap::new();
        for r in &recs {
            lines.entry(s(&i, r, "iso")).or_default().push(Line {
                line_type: s(&i, r, "line_type"),
                id: s(&i, r, "line_id"),
                name: s(&i, r, "line_name"),
                rate: f(s(&i, r, "rate")),
                base: s(&i, r, "base"),
                base_factor: f(s(&i, r, "base_factor")),
                floor: f(s(&i, r, "base_floor_annual_local")),
                cap: f(s(&i, r, "base_cap_annual_local")),
                flat: f(s(&i, r, "flat_annual_local")),
                min_gross: f(s(&i, r, "min_gross_annual_local")),
                max_gross: f(s(&i, r, "max_gross_annual_local")),
                extra_kind: s(&i, r, "extra_kind"),
                extra_value: f(s(&i, r, "extra_value")),
                in_total: b(s(&i, r, "in_total")),
                applies: s(&i, r, "applies"),
                notes: s(&i, r, "notes"),
            });
        }

        let (i, recs) = rows(ASSUMPTIONS_CSV);
        let mut assumptions: HashMap<String, Vec<(String, f64)>> = HashMap::new();
        for r in &recs {
            assumptions
                .entry(s(&i, r, "iso"))
                .or_default()
                .push((s(&i, r, "id"), f(s(&i, r, "value")).expect("assumption value")));
        }
        Data { countries, lines, assumptions }
    })
}

/// Every country, sorted by ISO code.
pub fn countries() -> &'static [Country] {
    &data().countries
}

/// Country by ISO 3166-1 alpha-2 code (case-insensitive).
pub fn country(iso: &str) -> Option<&'static Country> {
    countries().iter().find(|c| c.iso.eq_ignore_ascii_case(iso))
}

/// Contribution and statutory-extra lines of a country, in dataset order.
pub fn lines(iso: &str) -> &'static [Line] {
    data().lines.get(&iso.to_ascii_uppercase()).map(Vec::as_slice).unwrap_or(&[])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Contribution,
    Extra,
}

#[derive(Debug, Clone)]
pub struct LedgerLine {
    pub line: &'static Line,
    pub kind: Kind,
    pub annual_local: f64,
    pub annual_usd: f64,
    pub skipped: bool,
    pub capped: bool,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct CostResult {
    pub country: &'static Country,
    pub gross_local: f64,
    pub gross_usd: f64,
    pub lines: Vec<LedgerLine>,
    /// Contributions plus statutory extras flagged `in_total`, per year.
    pub employer_cost_local: f64,
    pub employer_cost_usd: f64,
    /// Employer cost over gross salary (0.2225 = 22.25 %).
    pub employer_rate: f64,
}

fn clamp(v: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let mut x = v;
    if let Some(m) = min {
        if x < m {
            x = m;
        }
    }
    if let Some(m) = max {
        if x > m {
            x = m;
        }
    }
    x
}

/// Same operations, in the same order, as `computeScenario` of the site.
/// Contributions are priced in local currency and converted at
/// `fx_local_per_usd`; statutory extras are priced on the gross in USD.
pub fn compute(country: &'static Country, gross_local: f64, gross_usd: f64) -> CostResult {
    let fx = country.fx_local_per_usd;
    let assumption = |id: &str, fallback: f64| {
        data()
            .assumptions
            .get(&country.iso)
            .and_then(|v| v.iter().find(|(k, _)| k == id))
            .map(|(_, v)| *v)
            .unwrap_or(fallback)
    };
    let share = assumption("basic_share_of_gross", 1.0);
    let basic_share = if share.is_finite() { share.max(0.0).min(1.0) } else { 1.0 };
    let citizen_or_pr = if assumption("employee_is_citizen_or_pr", 1.0) >= 1.0 { 1.0 } else { 0.0 };

    let all = lines(&country.iso);
    let mut out = Vec::new();
    for c in all.iter().filter(|l| l.line_type == "contribution") {
        let gated = c.max_gross.is_some_and(|m| gross_local > m) || c.min_gross.is_some_and(|m| gross_local < m);
        let switched_off = c.id == "employer_cpf" && citizen_or_pr < 1.0;
        if gated || switched_off {
            out.push(LedgerLine {
                line: c,
                kind: Kind::Contribution,
                annual_local: 0.0,
                annual_usd: 0.0,
                skipped: true,
                capped: false,
                note: if switched_off {
                    "Not applicable: employee is not a citizen or permanent resident".into()
                } else if c.applies.is_empty() {
                    "Not applicable at this salary".into()
                } else {
                    c.applies.clone()
                },
            });
            continue;
        }
        let rate = c.rate.unwrap_or(0.0);
        let factor = c.base_factor.unwrap_or(1.0);
        let base_local = match c.base.as_str() {
            "flat" => c.flat.unwrap_or(0.0),
            "basic" => gross_local * basic_share * factor,
            _ => gross_local * factor,
        };
        let annual_local = match c.base.as_str() {
            "flat" => base_local,
            "band" => {
                let slice = base_local.min(c.cap.unwrap_or(f64::INFINITY)) - c.floor.unwrap_or(0.0);
                slice.max(0.0) * rate
            }
            _ => clamp(base_local, c.floor, c.cap) * rate,
        };
        out.push(LedgerLine {
            line: c,
            kind: Kind::Contribution,
            annual_local,
            annual_usd: annual_local / fx,
            skipped: false,
            capped: c.base != "flat" && c.cap.is_some_and(|m| base_local > m),
            note: c.notes.clone(),
        });
    }
    for e in all.iter().filter(|l| l.line_type == "statutory_extra" && l.in_total) {
        let v = e.extra_value.unwrap_or(0.0);
        let amount = |gross: f64| match e.extra_kind.as_str() {
            "percent" => gross * (if e.base == "basic" { basic_share } else { 1.0 }) * (v / 100.0),
            "months" => (gross / 12.0) * v,
            "days" => (gross / 260.0) * v,
            _ => 0.0,
        };
        out.push(LedgerLine {
            line: e,
            kind: Kind::Extra,
            annual_local: amount(gross_local),
            annual_usd: amount(gross_usd),
            skipped: false,
            capped: false,
            note: e.notes.clone(),
        });
    }
    let employer_cost_usd = out.iter().fold(0.0, |s, l| s + l.annual_usd);
    let employer_cost_local = out.iter().fold(0.0, |s, l| s + l.annual_local);
    CostResult {
        country,
        gross_local,
        gross_usd,
        lines: out,
        employer_cost_local,
        employer_cost_usd,
        employer_rate: if gross_usd > 0.0 { employer_cost_usd / gross_usd } else { 0.0 },
    }
}

/// Employer cost at a gross annual salary in local currency.
pub fn cost_local(iso: &str, salary_annual_local: f64) -> Option<CostResult> {
    let c = country(iso)?;
    Some(compute(c, salary_annual_local, salary_annual_local / c.fx_local_per_usd))
}

/// Employer cost at the country's example salary, as the site computes it.
pub fn cost_at_example(c: &'static Country) -> CostResult {
    compute(c, c.example_salary_usd * c.fx_local_per_usd, c.example_salary_usd)
}

/// Fixed-point string of `v`, rounded half away from zero on the shortest
/// decimal that round-trips to `v`, as `Intl.NumberFormat` (ICU) does on the
/// site: 32.15 (binary 32.1499999...) prints 32.2 there, while Rust's `{:.1}`,
/// which rounds the exact binary value, would print 32.1.
pub fn fixed(v: f64, digits: usize) -> String {
    let neg = v < 0.0;
    let short = format!("{}", v.abs());
    let (int, frac) = short.split_once('.').unwrap_or((&short, ""));
    let frac = format!("{frac:0<digits$}");
    let mut ds: Vec<u8> = int.bytes().chain(frac.bytes().take(digits)).collect();
    if frac.as_bytes().get(digits).is_some_and(|d| *d >= b'5') {
        let mut i = ds.len();
        loop {
            if i == 0 {
                ds.insert(0, b'1');
                break;
            }
            i -= 1;
            if ds[i] == b'9' {
                ds[i] = b'0';
            } else {
                ds[i] += 1;
                break;
            }
        }
    }
    let split = ds.len() - digits;
    let mut s = String::from_utf8(ds[..split].to_vec()).unwrap();
    if digits > 0 {
        s.push('.');
        s.push_str(std::str::from_utf8(&ds[split..]).unwrap());
    }
    if neg && s.bytes().any(|b| b.is_ascii_digit() && b != b'0') {
        s.insert(0, '-');
    }
    s
}

/// Percent of a rate as the site prints it (`pct` in format.ts), without
/// grouping or the % sign: 0.22249999 -> "22.25" at 2 digits.
pub fn pct(rate: f64, digits: usize) -> String {
    fixed((rate * 1e8).round() / 1e6, digits)
}

/// Amount with 2 decimals, half away from zero, after the 1e-6 guard the
/// dataset export applies to the monthly figure.
pub fn money(v: f64) -> String {
    fixed((v * 1e6).round() / 1e6, 2)
}

/// Thousands separators on a fixed-point string: "1234567.89" -> "1,234,567.89".
pub fn group(s: &str) -> String {
    let (sign, rest) = s.strip_prefix('-').map_or(("", s), |r| ("-", r));
    let (int, frac) = rest.split_once('.').map_or((rest, None), |(a, b)| (a, Some(b)));
    let mut g = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            g.push(',');
        }
        g.push(ch);
    }
    match frac {
        Some(f) => format!("{sign}{g}.{f}"),
        None => format!("{sign}{g}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn hex(s: &str) -> String {
        Sha256::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Byte-identical to dataset/eorscope-employer-costs (export 2026-10-13),
    /// the files of the Go module v0.1.4.
    #[test]
    fn embedded_data_is_the_published_export() {
        assert_eq!(hex(SUMMARY_CSV), "83e1cec0a902ff629be224e22e0c4c11b90adc2f77aaa1e2b2914a79e6075254");
        assert_eq!(hex(CONTRIBUTIONS_CSV), "5c55cd63e18a9b8e60813653b600cbe1f0e1bafb560482c949ea5c225900ec85");
        assert_eq!(hex(ASSUMPTIONS_CSV), "c204bf448d5aecc99968a5e37f71473233cc179b10238d18537e46eb573fd50a");
        assert_eq!(countries().len(), 76);
        assert_eq!(countries().iter().map(|c| lines(&c.iso).len()).sum::<usize>(), 544);
    }

    fn num(s: &str) -> f64 {
        s.parse().unwrap()
    }

    /// (iso, employer_cost_pct, employer_cost_pct_printed, employer_cost_usd_month)
    /// copied from country_summary.csv, i.e. what Go's employercosts.Country(iso)
    /// returns as EmployerCostPct, EmployerCostPctPrinted, EmployerCostUSDMonth.
    const VECTORS: &[(&str, f64, f64, f64)] = &[
        ("DE", 22.25, 22.3, 1390.63),   // half away from zero on 22.25 (Rust {:.1} would give 22.2)
        ("IN", 3.7615, 3.8, 94.04),     // basic share 0.5, declared floor
        ("MX", 30.9336, 30.9, 928.01),  // base_factor 1.049315 / 1.049867, band lines
        ("AE", 2.8767, 2.9, 155.82),    // pegged currency, basic share
        ("AM", 0.0, 0.0, 0.0),          // no employer contribution
        ("ES", 32.15, 32.2, 1205.63),   // 32.15 is binary 32.1499999...: shortest-decimal rounding
    ];

    #[test]
    fn named_vectors_match_the_go_module_figures() {
        for &(iso, p4, p1, m) in VECTORS {
            let r = cost_at_example(country(iso).unwrap());
            assert_eq!(num(&pct(r.employer_rate, 4)), p4, "{iso} pct4");
            assert_eq!(num(&pct(r.employer_rate, 1)), p1, "{iso} printed");
            assert_eq!(num(&money(r.employer_cost_usd / 12.0)), m, "{iso} usd/month");
        }
    }

    #[test]
    fn every_country_matches_its_published_figures() {
        let mut bad = Vec::new();
        for c in countries() {
            let r = cost_at_example(c);
            let got = (pct(r.employer_rate, 4), pct(r.employer_rate, 1), money(r.employer_cost_usd / 12.0));
            if num(&got.0) != num(&c.employer_cost_pct)
                || num(&got.1) != num(&c.employer_cost_pct_printed)
                || num(&got.2) != num(&c.employer_cost_usd_month)
            {
                bad.push(format!(
                    "{}: got {:?}, want ({}, {}, {})",
                    c.iso, got, c.employer_cost_pct, c.employer_cost_pct_printed, c.employer_cost_usd_month
                ));
            }
        }
        assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad.join("\n"));
    }

    #[test]
    fn local_salary_gives_the_same_rate() {
        let c = country("de").unwrap();
        let r = cost_local("de", c.example_salary_usd * c.fx_local_per_usd).unwrap();
        assert_eq!(pct(r.employer_rate, 4), "22.2500");
        assert!((r.employer_cost_local / r.gross_local - r.employer_rate).abs() < 1e-12);
    }

    #[test]
    fn rounding() {
        assert_eq!(fixed(22.25, 1), "22.3");
        assert_eq!(fixed(1390.625, 2), "1390.63");
        assert_eq!(fixed(0.0, 2), "0.00");
        assert_eq!(fixed(32.15, 1), "32.2"); // binary 32.1499999..., ES on the site
        assert_eq!(fixed(9.995, 2), "10.00");
        assert_eq!(fixed(99.95, 1), "100.0");
        assert_eq!(fixed(0.0000001, 2), "0.00");
        assert_eq!(group("1234567.89"), "1,234,567.89");
        assert_eq!(group("123"), "123");
    }

    #[test]
    fn unknown_country() {
        assert!(country("xx").is_none());
        assert!(cost_local("xx", 1.0).is_none());
        assert!(lines("xx").is_empty());
    }
}
