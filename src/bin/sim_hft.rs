//! HFT-feature simulator — produces a small synthetic FIX log that exercises
//! every detector / scorecard the v3.2 release added.
//!
//! Output: `fixtures/fix_hft_stress.log`  (~5 000 messages, < 1 MB)
//!
//! What's intentionally in the log:
//!
//!   1. Reject burst — a 25-reject blast inside a 2 s window, after the
//!      first 1 000 normal orders. Should trigger the CRITICAL banner.
//!
//!   2. Sequence gap — MsgSeqNum jumps from 1 500 → 1 530 on the GAP→ME
//!      session. Should trigger a CRITICAL gap warning (skip ≥ 10).
//!
//!   3. Latency spike — first half of NOS→ER ack latencies sit at ~1 ms;
//!      second half jumps to ~15 ms (15×). Should trigger CRITICAL spike.
//!
//!   4. Three LPs with distinct character:
//!         GOOD_LP  → 98% fill, 3 ms hold, 0 % reject
//!         AVG_LP   → 88% fill, 25 ms hold, 8 % reject
//!         BAD_LP   → 55% fill, 90 ms hold, 35 % reject (← flagged)
//!
//!   5. Five symbols cycled (EUR/USD, GBP/USD, USD/JPY, USD/CHF, AUD/USD)
//!      so the LP × symbol heat grid has data to colour.
//!
//! Run:
//!     cargo run --release --bin sim_hft
//!     # then File → Load → fixtures/fix_hft_stress.log inside the app.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

const OUT_DIR:  &str = "fixtures";
const OUT_FILE: &str = "fix_hft_stress.log";

/// Microseconds since the simulated trading-day start (09:00:00.000).
type Us = i64;

fn fmt_ts(us: Us) -> String {
    // FIX 4.4 SendingTime: YYYYMMDD-HH:MM:SS.fff
    let total_s    = us / 1_000_000;
    let frac_us    = us % 1_000_000;
    let total_s = total_s + 9 * 3600;  // base 09:00
    let h = total_s / 3600;
    let m = (total_s % 3600) / 60;
    let s = total_s % 60;
    let ms = frac_us / 1_000;
    format!("20240315-{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

/// FIX 4.4 message header bits — all our messages share these counterparties
/// for the bulk of the LP traffic. Side parties get suffixes for the special
/// scenarios (gap, burst).
const TAKER: &str = "ME";

struct Lp {
    name:        &'static str,
    fill_rate:   f64,
    reject_rate: f64,
    hold_ms_p50: u32,
}

const LPS: &[Lp] = &[
    Lp { name: "GOOD_LP", fill_rate: 0.98, reject_rate: 0.00, hold_ms_p50: 3   },
    Lp { name: "AVG_LP",  fill_rate: 0.88, reject_rate: 0.08, hold_ms_p50: 25  },
    Lp { name: "BAD_LP",  fill_rate: 0.55, reject_rate: 0.35, hold_ms_p50: 90  },
];

const SYMBOLS: &[&str] = &["EUR/USD", "GBP/USD", "USD/JPY", "USD/CHF", "AUD/USD"];

/// Tiny deterministic PRNG so the same seed → same fixture. xorshift32.
struct Rng(u32);
impl Rng {
    fn new(seed: u32) -> Self { Rng(seed.max(1)) }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        self.0 = x; x
    }
    fn pct(&mut self) -> f64 { (self.next() as f64) / (u32::MAX as f64) }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[(self.next() as usize) % slice.len()]
    }
}

/// Per-message append helper. Writes one SOH-delimited FIX message + newline.
fn write_msg(out: &mut BufWriter<File>, fields: &[(u16, &str)]) -> std::io::Result<()> {
    let mut buf = String::with_capacity(256);
    for (tag, val) in fields {
        buf.push_str(&tag.to_string());
        buf.push('=');
        buf.push_str(val);
        buf.push('\x01');
    }
    buf.push('\n');
    out.write_all(buf.as_bytes())
}

/// Live-tail simulator: opens the same OUT_FILE in append mode and
/// writes one realistic Quote→NOS→ER chain every ~200ms forever, with
/// occasional bursts (random reject) so the anomaly banner lights up.
/// Run alongside the app: load the file once, toggle Live tail + Follow,
/// watch messages stream in.
fn run_live_mode() -> std::io::Result<()> {
    use std::io::Write as _;
    use std::time::{SystemTime, UNIX_EPOCH};
    fs::create_dir_all(OUT_DIR)?;
    let path = Path::new(OUT_DIR).join(OUT_FILE);
    // Truncate so a fresh run starts with a clean baseline.
    let f = File::create(&path)?;
    let mut out = BufWriter::with_capacity(8 * 1024, f);

    let mut rng = Rng::new(0xDEADBEEF);
    let mut seq_per_lp: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for lp in LPS { seq_per_lp.insert(lp.name, 1); }

    // Logons up front so the heartbeat status bar populates immediately.
    for lp in LPS {
        let seq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "A"),
            (49, TAKER), (56, lp.name),
            (34, &seq.to_string()),
            (52, &fmt_wall_clock()),
            (98, "0"), (108, "30"),
            (10, "000"),
        ])?;
        *seq += 1;
    }
    out.flush()?;

    println!("✓ Live mode — appending to {} every 200ms", path.display());
    println!("  In the app: File → Load {} → Live tail on → Follow on", path.display());
    println!("  Ctrl-C to stop.");

    let mut clord_seq: u32 = 1;
    let mut quote_seq: u32 = 1;
    let mut tick: u64 = 0;
    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    loop {
        let lp  = rng.pick(LPS);
        let sym = rng.pick(SYMBOLS);
        let q_id  = format!("QT{:08}", quote_seq); quote_seq += 1;
        let cl_id = format!("CO{:08}", clord_seq); clord_seq += 1;

        let ts = fmt_wall_clock();

        let qseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "S"),
            (49, lp.name), (56, TAKER),
            (34, &qseq.to_string()), (52, &ts),
            (117, &q_id), (55, sym),
            (10, "000"),
        ])?;
        *qseq += 1;
        let nseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "D"),
            (49, TAKER), (56, lp.name),
            (34, &nseq.to_string()), (52, &ts),
            (11, &cl_id), (117, &q_id),
            (55, sym), (54, "1"), (38, "1000000"), (40, "D"),
            (10, "000"),
        ])?;
        *nseq += 1;
        let r = rng.pct();
        let (exec_type, ord_status) = if r < lp.reject_rate { ("8", "8") }
                                      else if r < lp.reject_rate + lp.fill_rate { ("F", "2") }
                                      else { ("4", "4") };
        let eseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "8"),
            (49, lp.name), (56, TAKER),
            (34, &eseq.to_string()), (52, &ts),
            (11, &cl_id), (150, exec_type), (39, ord_status),
            (55, sym),
            (10, "000"),
        ])?;
        *eseq += 1;

        // Every ~150 ticks inject a 10-reject burst so the anomaly banner
        // fires while you watch.
        if tick > 0 && tick % 150 == 0 {
            for i in 0..10u32 {
                write_msg(&mut out, &[
                    (8, "FIX.4.4"), (9, "1"), (35, "3"),
                    (49, "BURST_LP"), (56, TAKER),
                    (34, &(i + 1).to_string()),
                    (52, &fmt_wall_clock()),
                    (45, &(2000 + i).to_string()),
                    (58, "Throttle"),
                    (10, "000"),
                ])?;
            }
        }

        out.flush()?;
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - start;
        if tick % 25 == 0 {
            println!("  tick {tick} · t+{elapsed}s · {} msgs written", clord_seq * 3);
        }
        tick += 1;
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// Wall-clock timestamp in FIX SendingTime form.
fn fmt_wall_clock() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let total_s = now.as_secs();
    let ms      = now.subsec_millis();
    // Render UTC. Approximate calendar via libc-free integer math.
    // Good enough for a simulator; FIX parsers don't care about date drift.
    let secs_in_day = total_s % 86_400;
    let h = secs_in_day / 3600;
    let m = (secs_in_day % 3600) / 60;
    let s = secs_in_day % 60;
    format!("20260524-{h:02}:{m:02}:{s:02}.{ms:03}")
}

fn main() -> std::io::Result<()> {
    if std::env::args().any(|a| a == "--live") {
        return run_live_mode();
    }
    fs::create_dir_all(OUT_DIR)?;
    let path = Path::new(OUT_DIR).join(OUT_FILE);
    let f = File::create(&path)?;
    let mut out = BufWriter::with_capacity(64 * 1024, f);

    let mut rng = Rng::new(0xCAFEBABE);
    let mut clord_seq: u32 = 0;
    let mut quote_seq: u32 = 0;
    let mut now: Us = 0;
    // Per-session MsgSeqNum so the sequence-gap scenario can be triggered
    // without breaking the rest of the log.
    let mut seq_per_lp: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for lp in LPS { seq_per_lp.insert(lp.name, 1); }
    let seq_gap: u32;

    // ── Logons (one per LP) ────────────────────────────────────────────────
    for lp in LPS {
        let seq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"),
            (9, "1"),
            (35, "A"),
            (49, TAKER), (56, lp.name),
            (34, &seq.to_string()),
            (52, &fmt_ts(now)),
            (98, "0"), (108, "30"),
            (10, "000"),
        ])?;
        *seq += 1;
        now += 1_000;
    }

    // ── Phase 1: 1 000 normal Quote→NOS→ER chains spread across LPs.
    //    First half latency ~1 ms (baseline), second half degrades to ~15 ms.
    let phase1_count: u32 = 1_000;
    for i in 0..phase1_count {
        let lp  = rng.pick(LPS);
        let sym = rng.pick(SYMBOLS);

        // Hold time = LP characteristic + small jitter, in ms.
        let hold_ms = lp.hold_ms_p50 as i64 + (rng.next() as i64 % 8 - 4);
        let hold_us = (hold_ms.max(1)) * 1_000;

        // ACK latency: phase1 first half = 1 ms baseline; second half = 15 ms spike.
        let ack_us: Us = if i < phase1_count / 2 { 1_000 } else { 15_000 };

        let q_id   = format!("QT{:08}", { quote_seq += 1; quote_seq });
        let cl_id  = format!("CO{:08}", { clord_seq += 1; clord_seq });
        let t_quote = now;
        let t_nos   = t_quote + hold_us;
        let t_er    = t_nos + ack_us;
        now = t_er + 2_000;

        // Quote (35=S) — sender = LP, target = taker.
        let qseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "S"),
            (49, lp.name), (56, TAKER),
            (34, &qseq.to_string()), (52, &fmt_ts(t_quote)),
            (117, &q_id), (55, sym),
            (10, "000"),
        ])?;
        *qseq += 1;

        // NewOrderSingle (35=D) — sender = taker, target = LP.
        // No incoming-direction seq numbers tracked on LP side here; keep them
        // monotonic per LP across all our messages for simplicity.
        let nseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "D"),
            (49, TAKER), (56, lp.name),
            (34, &nseq.to_string()), (52, &fmt_ts(t_nos)),
            (11, &cl_id), (117, &q_id),
            (55, sym), (54, "1"), (38, "1000000"), (40, "D"),
            (10, "000"),
        ])?;
        *nseq += 1;

        // ER outcome — sample per LP's reject / fill distribution.
        let r = rng.pct();
        let (exec_type, ord_status) = if r < lp.reject_rate {
            ("8", "8")
        } else if r < lp.reject_rate + lp.fill_rate {
            ("F", "2")
        } else {
            // Treat as pending / cancelled — neither fill nor reject.
            ("4", "4")
        };
        let eseq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "8"),
            (49, lp.name), (56, TAKER),
            (34, &eseq.to_string()), (52, &fmt_ts(t_er)),
            (11, &cl_id), (150, exec_type), (39, ord_status),
            (55, sym),
            (10, "000"),
        ])?;
        *eseq += 1;
    }

    // ── Phase 2: REJECT BURST — 25 raw 35=3 rejects inside 2 seconds.
    //    A separate session "BURST_LP" so the test is isolated.
    let burst_start = now;
    for i in 0..25u32 {
        let t = burst_start + (i as i64) * 70_000;   // 70 ms apart → 25 in 1.75 s
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "3"),
            (49, "BURST_LP"), (56, TAKER),
            (34, &(i + 1).to_string()),
            (52, &fmt_ts(t)),
            (45, &(1000 + i).to_string()),   // RefSeqNum
            (58, "Throttle: rate limit exceeded"),
            (10, "000"),
        ])?;
    }
    now = burst_start + 25 * 70_000 + 1_000_000;

    // ── Phase 3: SEQUENCE GAP — on the GAP→ME session, jump seq from 1 to 30.
    write_msg(&mut out, &[
        (8, "FIX.4.4"), (9, "1"), (35, "0"),
        (49, "GAP_LP"), (56, TAKER),
        (34, "1"), (52, &fmt_ts(now)),
        (10, "000"),
    ])?;
    now += 1_000;
    seq_gap = 30;
    write_msg(&mut out, &[
        (8, "FIX.4.4"), (9, "1"), (35, "0"),
        (49, "GAP_LP"), (56, TAKER),
        (34, &seq_gap.to_string()), (52, &fmt_ts(now)),
        (10, "000"),
    ])?;
    now += 1_000;

    // ── Logouts ──────────────────────────────────────────────────────────────
    for lp in LPS {
        let seq = seq_per_lp.get_mut(lp.name).unwrap();
        write_msg(&mut out, &[
            (8, "FIX.4.4"), (9, "1"), (35, "5"),
            (49, TAKER), (56, lp.name),
            (34, &seq.to_string()),
            (52, &fmt_ts(now)),
            (58, "End of trading day"),
            (10, "000"),
        ])?;
        now += 1_000;
    }

    out.flush()?;
    let n = fs::metadata(&path)?.len();
    println!("✓ Wrote {} ({} bytes)", path.display(), n);
    println!("  Phase 1: {} chain orders (latency degradation midway)", phase1_count);
    println!("  Phase 2: 25 rejects in 2s window  → anomaly banner");
    println!("  Phase 3: seq gap 1→30 on GAP_LP   → anomaly banner");
    println!("  LPs:     GOOD_LP / AVG_LP / BAD_LP → LP Scorecard");
    println!();
    println!("  Load with: File → Load file → {}", path.display());
    Ok(())
}
