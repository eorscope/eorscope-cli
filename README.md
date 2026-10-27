# eorscope

Command-line tool that prints the statutory employer cost of an employee in 76 countries, before any Employer of Record (EOR) fee, and the dated changes to employer contributions already scheduled. Data from [EOR Scope](https://eorscope.com/data/), embedded in the binary: no network access, no account, no telemetry.

Cost comparison, not legal or tax advice.

## Usage

```
eorscope countries [--json]
eorscope cost <ISO> <ANNUAL_GROSS_SALARY_LOCAL> [--json]
eorscope changes <ISO> [--json]
```

- `countries`: the 76 countries (ISO 3166-1 alpha-2 code, name, currency).
- `cost`: gross salary + statutory employer contributions + the statutory extras counted in the total, line by line, per year and per month, in the country's currency, with the USD equivalent at the dataset's exchange rate. The salary is yearly and in local currency (`60000` or `60,000`).
- `changes`: scheduled changes to employer contributions (effective date, new rate, cap or floor, legal status, official source and the date it was read), from the [`eorscope`](https://crates.io/crates/eorscope) crate.

```
$ eorscope cost DE 66050
Germany (DE, outside Saxony): gross salary EUR 66,050.00 a year

          EUR 6,142.65  9.3%             Statutory pension insurance (Rentenversicherung), employer half
...
Employer cost: EUR 14,696.13 a year (EUR 1,224.68 a month), 22.3% of gross
```

Some countries publish a floor ("at least"): some statutory costs are not modelled there, and the output says so. Countries whose base is modelled at the legal maximum say so too. The figures use the dataset's assumptions (for example the basic share of gross in India, or CPF for a Singapore citizen or permanent resident); the [online calculator](https://eorscope.com/eor-cost-calculator/) lets you change them.

## Method

The cost engine follows the one that prints the figures on eorscope.com, operation by operation: each contribution is its rate times its base (gross, basic share of gross, or the slice of gross between a floor and a cap), times the base factor (1 unless the law builds the base from more than the salary, as Mexico's integrated base does), floored and capped, and gated by the gross-salary thresholds of the line; flat lines add their fixed amount; statutory extras flagged `in_total` add a percent of gross, extra months or paid days. Percentages are rounded half away from zero, as the site prints them.

`cargo test` recomputes every country at its example salary and checks the result against the published columns `employer_cost_pct` (4 decimals), `employer_cost_pct_printed` (1 decimal) and `employer_cost_usd_month`: 76 of 76 match. Sources and method: <https://eorscope.com/methodology/>.

## Data

`data/` holds the EOR Scope employer-cost dataset, export of 2026-10-13, byte-identical to the files published with the dataset (sha256):

| file | sha256 |
|---|---|
| country_summary.csv | `83e1cec0a902ff629be224e22e0c4c11b90adc2f77aaa1e2b2914a79e6075254` |
| country_contributions.csv | `5c55cd63e18a9b8e60813653b600cbe1f0e1bafb560482c949ea5c225900ec85` |
| country_assumptions.csv | `c204bf448d5aecc99968a5e37f71473233cc179b10238d18537e46eb573fd50a` |

## Build

```
cargo test
cargo build --release
snapcraft pack        # snap/snapcraft.yaml, core24, strict confinement
```

## Licence

Code: MIT (`LICENSE`). Data in `data/` and in the `eorscope` crate: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) (`data/LICENSE`). Attribution: EOR Scope, published by Trésor Kaya EI (Les Créavores), France, <https://eorscope.com/methodology/>.

Source: https://eorscope.com/data/ (CC BY 4.0). Cost comparison, not legal or tax advice.
