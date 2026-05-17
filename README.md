# AI FIX Parser

Native desktop app for loading, parsing, and analyzing FIX protocol log files.
Built in Rust — parses **1 million FIX messages in under 100 ms**.

- **Platforms**: macOS (aarch64 / x86_64), Windows (x86_64), Linux (x86_64)
- **Website**: https://aifixparser.com
- **Stack**: Rust 2021, Dioxus 0.7 (desktop), Rayon, NEON / AVX2 SIMD

---

## Features

- Load `.fix` / `.log` / `.txt` files or entire folders
- Inspect any message in Table or Raw view, with 150+ FIX tags decoded
- Timeline filter by time range, sender / target, message type, ClOrdID, or field value
- Side-by-side **compare two log files** with diff coloring on shared ClOrdIDs
- Validate against FIX 4.x rules (missing tags, bad enums, checksum errors)
- **Latency analysis** — reconstruct full order chains (RFQ → Quote → NOS → Fill), histogram by phase
- **Session report** — order stats, fill quality scorecard, per-counterparty breakdown
- CSV export from any view
- 100% offline — your FIX logs never leave the machine

---

## License — Personal-Use Free, Commercial License Required

This software ships under a **dual license**. See [`LICENSE`](./LICENSE) for full text.

| Use case | License |
|----------|---------|
| Individual / personal use | **Free** (MIT-style) |
| Academic, educational, research | **Free** (MIT-style) |
| Hobby projects, non-commercial evaluation | **Free** (MIT-style) |
| Banks, hedge funds, asset managers, prop / HFT firms | **Commercial license required** |
| Exchanges, ECNs, ATSs, dark pools, clearing houses | **Commercial license required** |
| Prime brokers, custodians, market-data vendors | **Commercial license required** |
| Fintech (trading / brokerage / custody / order routing) | **Commercial license required** |
| Crypto exchanges, market makers, trading firms | **Commercial license required** |
| Any subsidiary, affiliate, contractor, or consultant working for the above | **Commercial license required** |

> If you're not sure which track applies to you, ask before deploying — see
> the [`LICENSE`](./LICENSE) file for the full list and conditions.

**Commercial license inquiries** — please email:

📧 **andrew.cs328@gmail.com**

Subject: `AI FIX Parser Commercial License Inquiry`

Please include: legal entity name, country of incorporation, brief description
of intended use, approximate number of seats, and expected deployment scale.

---

## Privacy & Network Behavior

AI FIX Parser is **100% offline for your FIX log data** — log content, message
fields, ClOrdIDs and counterparty IDs **never leave your machine**.

The app makes exactly two outbound HTTP calls, both anonymous:

| Call | Endpoint | Purpose | Data sent |
|------|----------|---------|-----------|
| Update check (on startup) | `https://aifixparser.com/latest-version` | Show "new version available" banner | None — plain GET |
| Anonymous usage telemetry (Google Analytics 4) | `https://www.googletagmanager.com/gtag/js?id=…` | Aggregate counters: app starts, file parses, parse times | App version, OS, **message count**, **parse microseconds**, **delimiter (soh/pipe)**. **No filenames, no message content, no field values, no counterparty IDs.** |

### Opt out of telemetry

Launch the app with the environment variable set:

```bash
AIFIXPARSER_NO_TELEMETRY=1 open /Applications/AiFixParser.app
```

Or add it to your shell profile to make it permanent. When set, the Google
Analytics script is never injected and no events fire.

### Local storage

The app writes one file on your machine, never sent anywhere:

- `~/.aifixparser/recents.json` — last 8 file paths you opened, for the
  "Recent" list in the welcome screen. Delete the file to clear.

## Contributing

By submitting a pull request you agree your contribution is licensed under the
same dual-license terms and may be relicensed as part of any future commercial
offering. See the *Contributions* section in [`LICENSE`](./LICENSE).
