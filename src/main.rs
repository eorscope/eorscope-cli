use eorscope_cli::{self as ec, Kind, SOURCE_LINE};
use serde_json::{json, Value};
use std::process::ExitCode;

const USAGE: &str = "\
eorscope - statutory employer cost of an employee in 76 countries (EOR Scope data)

Usage:
  eorscope countries [--json]
  eorscope cost <ISO> <ANNUAL_GROSS_SALARY_LOCAL> [--json]
  eorscope changes <ISO> [--json]

  countries  list the 76 countries (ISO 3166-1 alpha-2 code, name, currency)
  cost       employer cost = gross salary + statutory employer contributions and
             the statutory extras counted in the total, before any EOR fee;
             the salary is yearly, in the country's currency (60000 or 60,000)
  changes    dated changes to employer contributions already scheduled

Options:
  --json     machine-readable output
  -h, --help, -V, --version
";

fn num(s: String) -> Value {
    json!(s.parse::<f64>().unwrap_or(0.0))
}

fn rate_text(r: f64) -> String {
    let s = ec::fixed(r * 100.0, 4);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{s}%")
}

fn unknown(iso: &str) -> ExitCode {
    eprintln!("eorscope: unknown country code {iso:?} (run `eorscope countries`)");
    ExitCode::from(1)
}

fn countries(as_json: bool) -> ExitCode {
    let cs = ec::countries();
    if as_json {
        let list: Vec<Value> = cs
            .iter()
            .map(|c| json!({"iso": c.iso, "name": c.name, "region": c.region, "currency": c.currency}))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({"countries": list, "source": SOURCE_LINE})).unwrap());
    } else {
        for c in cs {
            println!("{}  {} ({})", c.iso, c.name, c.currency);
        }
        println!("\n{} countries.\n{SOURCE_LINE}", cs.len());
    }
    ExitCode::SUCCESS
}

fn cost(iso: &str, salary: &str, as_json: bool) -> ExitCode {
    let cleaned: String = salary.chars().filter(|c| *c != ',' && *c != '_').collect();
    let gross = match cleaned.parse::<f64>() {
        Ok(v) if v.is_finite() && v >= 0.0 => v,
        _ => {
            eprintln!("eorscope: salary must be a non-negative number, got {salary:?}");
            return ExitCode::from(2);
        }
    };
    let Some(r) = ec::cost_local(iso, gross) else { return unknown(iso) };
    let c = r.country;
    let cur = &c.currency;
    let total_local = r.gross_local + r.employer_cost_local;
    let not_in_total = ec::lines(&c.iso)
        .iter()
        .filter(|l| l.line_type == "statutory_extra" && !l.in_total)
        .count();

    if as_json {
        let lines: Vec<Value> = r
            .lines
            .iter()
            .map(|l| {
                json!({
                    "id": l.line.id,
                    "name": l.line.name,
                    "type": if l.kind == Kind::Contribution { "contribution" } else { "statutory_extra" },
                    "base": l.line.base,
                    "rate": l.line.rate,
                    "extra_kind": if l.line.extra_kind.is_empty() { Value::Null } else { json!(l.line.extra_kind) },
                    "extra_value": l.line.extra_value,
                    "annual_local": num(ec::money(l.annual_local)),
                    "skipped": l.skipped,
                    "capped": l.capped,
                    "note": l.note,
                })
            })
            .collect();
        let out = json!({
            "iso": c.iso,
            "country": c.name,
            "place": c.example_place,
            "currency": cur,
            "salary_annual_local": r.gross_local,
            "lines": lines,
            "employer_cost_annual_local": num(ec::money(r.employer_cost_local)),
            "employer_cost_monthly_local": num(ec::money(r.employer_cost_local / 12.0)),
            "employer_cost_pct": num(ec::pct(r.employer_rate, 4)),
            "employer_cost_pct_printed": num(ec::pct(r.employer_rate, 1)),
            "total_cost_annual_local": num(ec::money(total_local)),
            "total_cost_annual_usd": num(ec::money(r.gross_usd + r.employer_cost_usd)),
            "fx_local_per_usd": c.fx_local_per_usd,
            "fx_date": c.fx_date,
            "total_declared_floor": c.total_declared_floor,
            "total_declared_ceiling": c.total_declared_ceiling,
            "total_excludes": c.total_excludes,
            "statutory_extras_not_in_total": not_in_total,
            "last_reviewed": c.last_reviewed,
            "source": SOURCE_LINE,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return ExitCode::SUCCESS;
    }

    let amt = |v: f64| format!("{cur} {}", ec::group(&ec::money(v)));
    let place = if c.example_place.is_empty() { String::new() } else { format!(", {}", c.example_place) };
    println!("{} ({}{place}): gross salary {} a year\n", c.name, c.iso, amt(r.gross_local));
    for l in &r.lines {
        let how = match (l.kind, l.line.base.as_str()) {
            (_, _) if l.skipped => "skipped".to_string(),
            (Kind::Contribution, "flat") => "flat".to_string(),
            (Kind::Contribution, "band") => format!("{} of band", rate_text(l.line.rate.unwrap_or(0.0))),
            (Kind::Contribution, "basic") => format!("{} of basic", rate_text(l.line.rate.unwrap_or(0.0))),
            (Kind::Contribution, _) => rate_text(l.line.rate.unwrap_or(0.0)),
            (Kind::Extra, _) => {
                let v = l.line.extra_value.unwrap_or(0.0);
                let v = ec::fixed(v, 4);
                let v = v.trim_end_matches('0').trim_end_matches('.');
                match l.line.extra_kind.as_str() {
                    "percent" => format!("{v}%"),
                    "months" => format!("{v} month(s)"),
                    _ => format!("{v} day(s)"),
                }
            }
        };
        let cap = if l.capped { " (capped)" } else { "" };
        println!("  {:>20}  {:<16} {}{cap}", amt(l.annual_local), how, l.line.name);
        if l.skipped {
            println!("    {}", l.note);
        }
    }
    let at_least = if c.total_declared_floor { "at least " } else { "" };
    println!(
        "\nEmployer cost: {at_least}{} a year ({} a month), {at_least}{}% of gross",
        amt(r.employer_cost_local),
        amt(r.employer_cost_local / 12.0),
        ec::pct(r.employer_rate, 1)
    );
    println!(
        "Total cost before any EOR fee: {at_least}{} a year ({} a month)",
        amt(total_local),
        amt(total_local / 12.0)
    );
    if cur != "USD" {
        println!(
            "In USD: {at_least}${} a year at {} {cur} per USD ({})",
            ec::group(&ec::money(r.gross_usd + r.employer_cost_usd)),
            c.fx_local_per_usd,
            c.fx_date
        );
    }
    if c.total_declared_floor {
        println!("This total is a floor: some statutory costs are not modelled.");
    }
    if c.total_declared_ceiling {
        println!("Modelled on the highest contribution base the law allows.");
    }
    if !c.total_excludes.is_empty() {
        println!("Not included: {}", c.total_excludes);
    }
    if not_in_total > 0 {
        println!("{not_in_total} statutory item(s) listed for {} are not in this total.", c.name);
    }
    println!("Data last reviewed {}. Calculator: https://eorscope.com/eor-cost-calculator/", c.last_reviewed);
    println!("\n{SOURCE_LINE}");
    ExitCode::SUCCESS
}

fn changes(iso: &str, as_json: bool) -> ExitCode {
    let upper = iso.to_ascii_uppercase();
    let sched = eorscope::dataset().scheduled.get(&upper);
    let Some(c) = ec::country(iso) else { return unknown(iso) };
    let list = eorscope::changes_for(iso);
    if as_json {
        let items: Vec<Value> = list
            .iter()
            .map(|x| {
                json!({
                    "contribution_id": x.contribution_id,
                    "effective_from": x.effective_from,
                    "date_basis": x.date_basis,
                    "rate": x.change.rate,
                    "cap_annual_local": x.change.cap_annual_local,
                    "threshold_weekly_local": x.change.threshold_weekly_local,
                    "floor_annual_local": x.change.floor_annual_local,
                    "status": x.status,
                    "note": x.note,
                    "source": {"url": x.source.url, "quote": x.source.quote, "read_on": x.source.read_on, "kind": x.source.kind},
                })
            })
            .collect();
        let pending: Vec<Value> = sched
            .map(|s| {
                s.pending
                    .iter()
                    .map(|p| json!({"status": p.status, "description": p.description, "contribution_id": p.contribution_id, "source_url": p.source.url}))
                    .collect()
            })
            .unwrap_or_default();
        let out = json!({
            "iso": c.iso,
            "country": c.name,
            "researched_on": sched.map(|s| s.researched_on.clone()),
            "coverage": sched.map(|s| s.coverage.clone()),
            "scheduled_changes": items,
            "pending": pending,
            "snapshot_date": eorscope::dataset().snapshot_date,
            "source": SOURCE_LINE,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return ExitCode::SUCCESS;
    }
    println!("{} ({}): scheduled changes to employer contributions", c.name, c.iso);
    if let Some(s) = sched {
        println!("Researched {}. Coverage: {}\n", s.researched_on, s.coverage);
    }
    if list.is_empty() {
        println!("No scheduled change recorded.");
    }
    for x in &list {
        let mut what = Vec::new();
        if let Some(r) = x.change.rate {
            what.push(format!("rate {}", rate_text(r)));
        }
        if let Some(v) = x.change.cap_annual_local {
            what.push(format!("annual cap {} {}", c.currency, ec::group(&v.to_string())));
        }
        if let Some(v) = x.change.floor_annual_local {
            what.push(format!("annual floor {} {}", c.currency, ec::group(&v.to_string())));
        }
        if let Some(v) = x.change.threshold_weekly_local {
            what.push(format!("weekly threshold {} {}", c.currency, ec::group(&v.to_string())));
        }
        println!(
            "{}  {}: {} [{}]",
            x.effective_from,
            x.contribution_id.as_deref().unwrap_or("-"),
            what.join(", "),
            x.status
        );
        if !x.note.is_empty() {
            println!("    {}", x.note);
        }
        println!("    Source: {} (read {})", x.source.url, x.source.read_on);
    }
    if let Some(s) = sched.filter(|s| !s.pending.is_empty()) {
        println!("\nPending:");
        for p in &s.pending {
            println!("  [{}] {}\n    Source: {}", p.status, p.description, p.source.url);
        }
    }
    println!("\n{SOURCE_LINE}");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let as_json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("eorscope {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["countries"] => countries(as_json),
        ["cost", iso, salary] => cost(iso, salary, as_json),
        ["changes", iso] => changes(iso, as_json),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
