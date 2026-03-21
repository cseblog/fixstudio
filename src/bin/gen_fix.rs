//! FIX 4.4 test-data generator — produces ~1,000,000 realistic FX trading messages.
//!
//! Run:  cargo run --release --bin gen_fix
//! Output: fix_test_1m.log  (~180-220 MB, pipe-delimited, one message per line)
//!
//! Workflows modelled (per session, across 8 client/server pairs):
//!   • RFQ → Quote → NOS(Previously-Quoted) → ExecRpt(New) → ExecRpt(Fill)
//!   • RFQ → Quote → NOS → ExecRpt(New) → ExecRpt(Partial) → ExecRpt(Fill)
//!   • RFQ → Quote → NOS → ExecRpt(Rejected)
//!   • Direct NOS(Market/Limit) → ExecRpt(New) → ExecRpt(Fill)
//!   • NOS → ExecRpt(New) → OrderCancelRequest → ExecRpt(Cancelled)
//!   • Periodic Heartbeats (every 30 s, both directions)
//!   • Logon / Logout per session

use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::{BufWriter, Write};

type MsgBuf = Vec<(u64, String)>;

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_TARGET: usize = 1_000_000;
const DEFAULT_OUTPUT: &str  = "fix_test_1m.log";
const DATE: &str = "20240315"; // Friday 15-Mar-2024

/// (client_comp_id, server_comp_id)
const SESSIONS: &[(&str, &str)] = &[
    ("CITIFX",   "FXECN"),
    ("MORGANFX", "FXECN"),
    ("BARCLAYS", "FXECN"),
    ("DBFX",     "FXECN"),
    ("BNPFX",    "FXECN"),
    ("HSBCFX",   "FXECN"),
    ("UBSFX",    "FXECN"),
    ("NOMURA",   "FXECN"),
];

/// (fix_symbol, mid_price, spread_decimal, lot_min, lot_step)
///   spread_decimal: bid = mid - spread/2, offer = mid + spread/2
const SYMBOLS: &[(&str, f64, f64, u64, u64)] = &[
    ("EUR/USD", 1.085_42, 0.000_05, 1_000_000, 1_000_000),
    ("GBP/USD", 1.268_34, 0.000_07, 1_000_000, 1_000_000),
    ("USD/JPY", 149.850,  0.005_0,  1_000_000, 1_000_000),
    ("USD/CHF", 0.892_34, 0.000_08, 500_000,   500_000),
    ("AUD/USD", 0.651_23, 0.000_08, 500_000,   500_000),
    ("USD/CAD", 1.356_78, 0.000_09, 500_000,   500_000),
    ("NZD/USD", 0.602_34, 0.000_12, 500_000,   500_000),
    ("EUR/GBP", 0.855_34, 0.000_06, 1_000_000, 1_000_000),
    ("EUR/JPY", 162.453,  0.006_0,  1_000_000, 1_000_000),
    ("GBP/JPY", 190.234,  0.010_0,  500_000,   500_000),
    ("USD/MXN", 17.023,   0.003_0,  500_000,   500_000),
    ("USD/ZAR", 18.654,   0.005_0,  500_000,   500_000),
];

/// Accounts per client (index matches SESSIONS)
const ACCOUNTS: &[&[&str]] = &[
    &["CITI-FX-01", "CITI-FX-02", "CITI-HF-01"],
    &["MS-FX-DESK", "MS-ALGO-01"],
    &["BARC-FX-01", "BARC-HF-02", "BARC-MM-01"],
    &["DB-FX-MAIN", "DB-STRATS-1"],
    &["BNP-FX-01",  "BNP-ALGO-02"],
    &["HSBC-FX-01", "HSBC-FX-02", "HSBC-HF-01"],
    &["UBS-FX-MAIN","UBS-HF-01"],
    &["NOM-FX-01",  "NOM-ALGO-01"],
];

/// Trading window: 07:00 – 23:59 UTC in microseconds since midnight
/// Extended so high-volume sessions don't run out of time before hitting TARGET.
const DAY_START_US: u64 = 7 * 3600 * 1_000_000;
const DAY_END_US:   u64 = 23 * 3600 * 1_000_000 + 59 * 60 * 1_000_000;

// ── XorShift64 RNG (no external deps) ────────────────────────────────────────

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn urange(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo)
    }
    fn f64_01(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn choice<T: Copy>(&mut self, slice: &[T]) -> T {
        slice[self.next() as usize % slice.len()]
    }
}

// ── Lot size picker (spreads orders across all 5 treemap size buckets) ────────

/// Pick a notional size distributed across FX size buckets:
///   10 %  →   < 1M   (retail / small hedge-fund tickets)
///   30 %  →  1M–5M   (standard institutional tickets)
///   30 %  →  5M–10M  (mid-size institutional)
///   20 %  → 10M–50M  (large institutional)
///   10 %  →  > 50M   (block / prime brokerage)
fn pick_lots(rng: &mut Rng) -> u64 {
    match rng.next() % 10 {
        0     => rng.urange(1, 10)   * 100_000,     //  100k – 900k
        1..=3 => rng.urange(1,  5)   * 1_000_000,   //   1M  –   4M
        4..=6 => rng.urange(5, 10)   * 1_000_000,   //   5M  –   9M
        7..=8 => rng.urange(10, 50)  * 1_000_000,   //  10M  –  49M
        _     => rng.urange(50, 201) * 1_000_000,   //  50M  – 200M
    }
}

// ── ID counters ───────────────────────────────────────────────────────────────

struct Ids {
    qreq: u64,
    quot: u64,
    cord: u64,
    ord:  u64,
    exec: u64,
}

impl Ids {
    fn new() -> Self { Self { qreq: 0, quot: 0, cord: 0, ord: 0, exec: 0 } }
    fn next_qreq(&mut self) -> u64 { self.qreq += 1; self.qreq }
    fn next_quot(&mut self) -> u64 { self.quot += 1; self.quot }
    fn next_cord(&mut self) -> u64 { self.cord += 1; self.cord }
    fn next_ord(&mut self)  -> u64 { self.ord  += 1; self.ord  }
    fn next_exec(&mut self) -> u64 { self.exec += 1; self.exec }
}

// ── Timestamp helpers ─────────────────────────────────────────────────────────

/// Format microseconds-since-midnight as FIX SendingTime: YYYYMMDD-HH:MM:SS.uuuuuu
fn fmt_ts(us: u64) -> String {
    let s    = us / 1_000_000;
    let frac = us % 1_000_000;
    let h    = s / 3600;
    let m    = (s % 3600) / 60;
    let sec  = s % 60;
    format!("{}-{:02}:{:02}:{:02}.{:06}", DATE, h, m, sec, frac)
}

/// SettlDate: spot = T+2 weekdays.  For 2024-03-15 (Fri) spot = 2024-03-19 (Mon+2)
const SETTL_DATE: &str = "20240319";

// ── FIX message builder ───────────────────────────────────────────────────────

/// Build a fully framed FIX 4.4 message from body fields.
///
/// `body_fields` contains everything from tag 35 onwards (with trailing `|`).
/// Returns the complete pipe-delimited message with correct tag-9 and tag-10.
fn build(body_fields: &str) -> String {
    let body_len = body_fields.len();
    let header   = format!("8=FIX.4.4|9={}|", body_len);
    let checksum: u32 = header.bytes().chain(body_fields.bytes())
        .map(|b| b as u32).sum::<u32>() % 256;
    format!("{}{}10={:03}|", header, body_fields, checksum)
}

/// Collect a complete message into the time-sorted buffer.
#[inline]
fn emit(msgs: &mut MsgBuf, ts: u64, body: &str, total: &mut usize) {
    msgs.push((ts, build(body)));
    *total += 1;
}

// ── Session state ─────────────────────────────────────────────────────────────

struct Session {
    client:    &'static str,
    server:    &'static str,
    accounts:  &'static [&'static str],
    seq_c:     u64,   // client → server seq
    seq_s:     u64,   // server → client seq
    time_us:   u64,   // current clock (μs since midnight)
    hb_due_us: u64,   // next heartbeat due
}

impl Session {
    fn new(idx: usize, start_offset_us: u64) -> Self {
        let start = DAY_START_US + start_offset_us;
        Session {
            client:    SESSIONS[idx].0,
            server:    SESSIONS[idx].1,
            accounts:  ACCOUNTS[idx],
            seq_c:     0,
            seq_s:     0,
            time_us:   start,
            hb_due_us: start + 30_000_000, // first heartbeat after 30 s
        }
    }
    fn next_seq_c(&mut self) -> u64 { self.seq_c += 1; self.seq_c }
    fn next_seq_s(&mut self) -> u64 { self.seq_s += 1; self.seq_s }

    /// Advance time by `delta_us` microseconds, clamped to day-end.
    fn tick(&mut self, delta_us: u64) {
        self.time_us = (self.time_us + delta_us).min(DAY_END_US);
    }
}

// ── Message emitters ──────────────────────────────────────────────────────────

fn emit_logon(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    // client → server
    let ts_c = fmt_ts(s.time_us);
    let seq_c = s.next_seq_c();
    let mut b = String::with_capacity(128);
    write!(b, "35=A|34={}|49={}|52={}|56={}|98=0|108=30|",
        seq_c, s.client, ts_c, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    // server → client (12 ms later)
    s.tick(12_000);
    let ts_s = fmt_ts(s.time_us);
    let seq_s = s.next_seq_s();
    let mut b = String::with_capacity(128);
    write!(b, "35=A|34={}|49={}|52={}|56={}|98=0|108=30|",
        seq_s, s.server, ts_s, s.client).unwrap();
    emit(msgs, s.time_us, &b, total);
    s.tick(1_000_000); // 1 s quiet after logon
}

fn emit_logout(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    let ts = fmt_ts(s.time_us);
    let seq_c = s.next_seq_c();
    let mut b = String::with_capacity(80);
    write!(b, "35=5|34={}|49={}|52={}|56={}|58=End of trading day|",
        seq_c, s.client, ts, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(50_000);
    let ts = fmt_ts(s.time_us);
    let seq_s = s.next_seq_s();
    let mut b = String::with_capacity(80);
    write!(b, "35=5|34={}|49={}|52={}|56={}|58=Acknowledged|",
        seq_s, s.server, ts, s.client).unwrap();
    emit(msgs, s.time_us, &b, total);
}

fn emit_heartbeats(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    while s.time_us >= s.hb_due_us {
        let ts_c = fmt_ts(s.hb_due_us);
        let seq_c = s.next_seq_c();
        let mut b = String::with_capacity(80);
        write!(b, "35=0|34={}|49={}|52={}|56={}|",
            seq_c, s.client, ts_c, s.server).unwrap();
        emit(msgs, s.time_us, &b, total);

        let ts_s = fmt_ts(s.hb_due_us + 8_000);
        let seq_s = s.next_seq_s();
        let mut b = String::with_capacity(80);
        write!(b, "35=0|34={}|49={}|52={}|56={}|",
            seq_s, s.server, ts_s, s.client).unwrap();
        emit(msgs, s.time_us, &b, total);

        s.hb_due_us += 30_000_000;
    }
}

// ── Workflow generators ───────────────────────────────────────────────────────

/// RFQ → Quote → NOS(Previously-Quoted) → ExecRpt(New) → ExecRpt(Fill)   [5 msgs]
fn workflow_rfq_fill(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, mid, spread, _, _) = SYMBOLS[sym_idx];
    let side       = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots       = pick_lots(rng);
    let account    = rng.choice(s.accounts);

    let mid_var = mid + (rng.f64_01() - 0.5) * spread * 4.0;
    let bid     = mid_var - spread / 2.0;
    let offer   = mid_var + spread / 2.0;

    let qreq_id = ids.next_qreq();
    let quot_id = ids.next_quot();
    let cl_ord  = ids.next_cord();
    let ord_id  = ids.next_ord();
    let exec1   = ids.next_exec();
    let exec2   = ids.next_exec();

    // ── QuoteRequest (client → server) ──
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b,
        "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, SETTL_DATE
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── Quote (server → client, 80–450 μs later) ──
    let network_us = rng.urange(80, 450);
    s.tick(network_us);
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b,
        "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, SETTL_DATE
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── NOS: client accepts quote (50 ms – 1 s decision time) ──
    let decision_us = rng.urange(50_000, 1_000_000);
    s.tick(decision_us);
    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let fill_px = if side == 1 { offer } else { bid };
    let mut b = String::with_capacity(256);
    write!(b,
        "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, SETTL_DATE, ts_nos
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── ExecRpt: New (5–40 ms) ──
    let new_us = rng.urange(5_000, 40_000);
    s.tick(new_us);
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b,
        "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, ts
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── ExecRpt: Fill (1–15 ms) ──
    let fill_us = rng.urange(1_000, 15_000);
    s.tick(fill_us);
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b,
        "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54={}|38={}|14={}|151=0|31={:.5}|32={}|6={:.5}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, lots, fill_px, lots, fill_px, ts
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // advance 10 – 500 ms before next workflow
    s.tick(rng.urange(10_000, 500_000));
}

/// RFQ → Quote → NOS → ExecRpt(New) → ExecRpt(Partial) → ExecRpt(Fill)   [6 msgs]
fn workflow_rfq_partial(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, mid, spread, _, _) = SYMBOLS[sym_idx];
    let side    = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots    = pick_lots(rng).max(200_000);
    let partial = (lots / 2).max(100_000);
    let remain  = lots - partial;
    let account = rng.choice(s.accounts);

    let mid_var = mid + (rng.f64_01() - 0.5) * spread * 4.0;
    let bid     = mid_var - spread / 2.0;
    let offer   = mid_var + spread / 2.0;
    let fill_px = if side == 1 { offer } else { bid };

    let qreq_id = ids.next_qreq();
    let quot_id = ids.next_quot();
    let cl_ord  = ids.next_cord();
    let ord_id  = ids.next_ord();
    let exec1   = ids.next_exec();
    let exec2   = ids.next_exec();
    let exec3   = ids.next_exec();

    // QuoteRequest
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b, "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, SETTL_DATE).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Quote
    s.tick(rng.urange(80, 450));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, SETTL_DATE).unwrap();
    emit(msgs, s.time_us, &b, total);

    // NOS
    s.tick(rng.urange(50_000, 800_000));
    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, SETTL_DATE, ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt New
    s.tick(rng.urange(5_000, 30_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt PartialFill
    s.tick(rng.urange(2_000, 20_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=1|39=1|55={}|54={}|38={}|14={}|151={}|31={:.5}|32={}|6={:.5}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, partial, remain, fill_px, partial, fill_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Fill (remainder)
    s.tick(rng.urange(1_000, 10_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54={}|38={}|14={}|151=0|31={:.5}|32={}|6={:.5}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec3, sym, side, lots, lots, fill_px, remain, fill_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(10_000, 300_000));
}

/// Direct NOS(Market) → ExecRpt(New) → ExecRpt(Fill)   [3 msgs]
fn workflow_market_fill(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, mid, spread, _, _) = SYMBOLS[sym_idx];
    let side    = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots    = pick_lots(rng);
    let account = rng.choice(s.accounts);
    let fill_px = mid + if side == 1 { spread / 2.0 } else { -spread / 2.0 };

    let cl_ord = ids.next_cord();
    let ord_id = ids.next_ord();
    let exec1  = ids.next_exec();
    let exec2  = ids.next_exec();

    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=1|59=3|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, SETTL_DATE, ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(3_000, 25_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(500, 8_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54={}|38={}|14={}|151=0|31={:.5}|32={}|6={:.5}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, lots, fill_px, lots, fill_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(10_000, 150_000));
}

/// Direct NOS(Limit) → ExecRpt(New) → [waits] → ExecRpt(Fill/Expired)   [3 msgs]
fn workflow_limit_order(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, mid, spread, _, _) = SYMBOLS[sym_idx];
    let side    = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots    = pick_lots(rng);
    let account = rng.choice(s.accounts);
    // Limit price slightly away from market
    let limit_px = mid + if side == 1 {
        -(spread * rng.f64_01() * 3.0)
    } else {
        spread * rng.f64_01() * 3.0
    };

    let cl_ord = ids.next_cord();
    let ord_id = ids.next_ord();
    let exec1  = ids.next_exec();
    let exec2  = ids.next_exec();

    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=2|44={:.5}|59=0|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, limit_px, SETTL_DATE, ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(5_000, 40_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|44={:.5}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, limit_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Filled 70% of the time, expired 30%
    s.tick(rng.urange(200_000, 5_000_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let filled = rng.next() % 10 < 7;
    let mut b = String::with_capacity(256);
    if filled {
        write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54={}|38={}|14={}|151=0|31={:.5}|32={}|6={:.5}|60={}|",
            seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, lots, limit_px, lots, limit_px, ts).unwrap();
    } else {
        write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=C|39=C|55={}|54={}|38={}|14=0|151=0|6=0|58=Order expired|60={}|",
            seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, ts).unwrap();
    }
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(10_000, 100_000));
}

/// NOS → ExecRpt(New) → OrderCancelRequest → ExecRpt(Cancelled)   [4 msgs]
fn workflow_cancel(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, _, spread, _, _) = SYMBOLS[sym_idx];
    let side    = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots    = pick_lots(rng);
    let account = rng.choice(s.accounts);
    let mid     = SYMBOLS[sym_idx].1;
    let limit_px = mid - spread * 2.0 * rng.f64_01();

    let cl_ord1 = ids.next_cord();
    let cl_ord2 = ids.next_cord(); // cancel uses new ClOrdID
    let ord_id  = ids.next_ord();
    let exec1   = ids.next_exec();
    let exec2   = ids.next_exec();

    // NOS
    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=2|44={:.5}|59=1|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord1, account, sym, side, lots, limit_px, SETTL_DATE, ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt New
    s.tick(rng.urange(5_000, 30_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|44={:.5}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord1, exec1, sym, side, lots, lots, limit_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // OrderCancelRequest (client decides to cancel, 100ms – 2s later)
    s.tick(rng.urange(100_000, 2_000_000));
    let ts_cxl = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=F|34={}|49={}|52={}|56={}|11=CO{:08}|41=CO{:08}|37=OR{:08}|55={}|54={}|38={}|60={}|",
        seq, s.client, ts_cxl, s.server, cl_ord2, cl_ord1, ord_id, sym, side, lots, ts_cxl).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Cancelled
    s.tick(rng.urange(3_000, 20_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(280);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|41=CO{:08}|17=EX{:08}|150=4|39=4|55={}|54={}|38={}|14=0|151=0|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord2, cl_ord1, exec2, sym, side, lots, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(10_000, 100_000));
}

/// RFQ → Quote → NOS → ExecRpt(Rejected)   [4 msgs]
fn workflow_rfq_reject(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, sym_idx: usize, total: &mut usize,
) {
    let (sym, mid, spread, _, _) = SYMBOLS[sym_idx];
    let side    = if rng.next() & 1 == 0 { 1u8 } else { 2u8 };
    let lots    = pick_lots(rng);
    let account = rng.choice(s.accounts);
    let mid_var = mid + (rng.f64_01() - 0.5) * spread * 4.0;
    let bid     = mid_var - spread / 2.0;
    let offer   = mid_var + spread / 2.0;
    let fill_px = if side == 1 { offer } else { bid };

    let qreq_id = ids.next_qreq();
    let quot_id = ids.next_quot();
    let cl_ord  = ids.next_cord();
    let ord_id  = ids.next_ord();
    let exec1   = ids.next_exec();

    // QuoteRequest
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b, "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, SETTL_DATE).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Quote
    s.tick(rng.urange(80, 450));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, SETTL_DATE).unwrap();
    emit(msgs, s.time_us, &b, total);

    // NOS
    s.tick(rng.urange(50_000, 600_000));
    let ts_nos = fmt_ts(s.time_us);
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, SETTL_DATE, ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Rejected
    s.tick(rng.urange(4_000, 15_000));
    let ts = fmt_ts(s.time_us);
    let seq = s.next_seq_s();
    let reject_reasons = &[
        "Quote expired",
        "Insufficient liquidity",
        "Outside risk limits",
        "Market moved",
        "Size too large",
    ];
    let reason = rng.choice(reject_reasons);
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=8|39=8|55={}|54={}|38={}|14=0|151=0|6=0|58={}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, reason, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(10_000, 80_000));
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Optional positional args: <target_count> <output_file>
    //   cargo run --release --bin gen_fix -- 100000 fix_test_100k.log
    let args: Vec<String> = std::env::args().collect();
    let target: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(DEFAULT_TARGET);
    let output: &str  = args.get(2).map(|s| s.as_str()).unwrap_or(DEFAULT_OUTPUT);

    let mut msgs: MsgBuf = Vec::with_capacity(target + 4096);
    let mut total = 0usize;
    let mut ids   = Ids::new();
    let mut rng   = Rng(0xDEAD_BEEF_1234_5678);

    // All 8 sessions run concurrently; each starts at 07:00 UTC with a small stagger.
    // Messages are collected with timestamps and sorted at the end.
    let n_sessions  = SESSIONS.len();
    let per_session = target / n_sessions;

    for idx in 0..n_sessions {
        let stagger_us = (idx as u64) * 250_000; // 250 ms between session logons
        let mut s = Session::new(idx, stagger_us);

        emit_logon(&mut msgs, &mut s, &mut total);
        emit_heartbeats(&mut msgs, &mut s, &mut total);

        // Session-level target: stop a few messages short to hit exactly target overall
        let session_target = if idx == n_sessions - 1 {
            target.saturating_sub(total).saturating_sub(2) // leave room for logout
        } else {
            per_session.saturating_sub(4) // -2 logon, -2 logout
        };

        while total < session_target + (idx * per_session) && s.time_us < DAY_END_US {
            // Emit any due heartbeats
            emit_heartbeats(&mut msgs, &mut s, &mut total);

            // Pick symbol (weight toward majors)
            let sym_weights = [25u64, 20, 20, 10, 8, 8, 4, 8, 5, 5, 4, 3];
            let sym_total: u64 = sym_weights.iter().sum();
            let mut r = rng.next() % sym_total;
            let mut sym_idx = 0;
            for (i, &w_) in sym_weights.iter().enumerate() {
                if r < w_ { sym_idx = i; break; }
                r -= w_;
            }

            // Pick workflow (weighted)
            match rng.next() % 100 {
                0..=44  => workflow_rfq_fill(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
                45..=59 => workflow_rfq_partial(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
                60..=74 => workflow_market_fill(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
                75..=84 => workflow_limit_order(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
                85..=92 => workflow_cancel(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
                _       => workflow_rfq_reject(&mut msgs, &mut s, &mut ids, &mut rng, sym_idx, &mut total),
            }

            // Occasional burst: short pause between clusters (simulates market activity bursts)
            if rng.next() % 30 == 0 {
                s.tick(rng.urange(1_000_000, 8_000_000)); // 1–8 s quiet period
            }
        }

        emit_heartbeats(&mut msgs, &mut s, &mut total);
        emit_logout(&mut msgs, &mut s, &mut total);
    }

    // Sort all sessions' messages by timestamp to produce a realistic interleaved log.
    msgs.sort_unstable_by_key(|(ts, _)| *ts);

    let file = File::create(output).expect("Cannot create output file");
    let mut w = BufWriter::with_capacity(4 * 1024 * 1024, file);
    for (_, msg) in &msgs {
        w.write_all(msg.as_bytes()).unwrap();
        w.write_all(b"\n").unwrap();
    }
    w.flush().unwrap();
    eprintln!("Done. {} messages written to {}", total, output);
}
