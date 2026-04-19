//! FIX 4.4 test-data generator — produces ~1,000,000 realistic FX trading messages.
//!
//! Run:  cargo run --release --bin gen_fix
//! Output: fix_test_1m.log  (~180-220 MB, SOH-delimited, one message per line)
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

const DEFAULT_TARGET:  usize = 1_000_000;
const LARGE_TARGET:    usize = 100_000_000;
const DEFAULT_DIR:     &str  = "fixtures";
const DEFAULT_OUTPUT:  &str  = "fix_test_1m.log";
const LARGE_OUTPUT:    &str  = "fix_test_100m.log";
const HEALTH_OUTPUT:   &str  = "fix_health_test_100k.log";

/// 65 trading dates covering Jan–Mar 2024.
/// Sessions cycle through these with `day_idx % TRADING_DATES.len()` so that
/// timestamps remain realistic even across hundreds of synthetic trading days.
const TRADING_DATES: &[&str] = &[
    "20240102","20240103","20240104","20240105",
    "20240108","20240109","20240110","20240111","20240112",
    "20240115","20240116","20240117","20240118","20240119",
    "20240122","20240123","20240124","20240125","20240126",
    "20240129","20240130","20240131",
    "20240201","20240202",
    "20240205","20240206","20240207","20240208","20240209",
    "20240212","20240213","20240214","20240215","20240216",
    "20240219","20240220","20240221","20240222","20240223",
    "20240226","20240227","20240228","20240229",
    "20240301",
    "20240304","20240305","20240306","20240307","20240308",
    "20240311","20240312","20240313","20240314","20240315",
    "20240318","20240319","20240320","20240321","20240322",
    "20240325","20240326","20240327","20240328",
];

/// T+2 calendar spot settlement date for each entry in TRADING_DATES.
const SETTL_DATES: &[&str] = &[
    "20240104","20240105","20240106","20240107",
    "20240110","20240111","20240112","20240113","20240114",
    "20240117","20240118","20240119","20240120","20240121",
    "20240124","20240125","20240126","20240127","20240128",
    "20240131","20240201","20240202",
    "20240203","20240204",
    "20240207","20240208","20240209","20240210","20240211",
    "20240214","20240215","20240216","20240217","20240218",
    "20240221","20240222","20240223","20240224","20240225",
    "20240228","20240229","20240301","20240302",
    "20240303",
    "20240306","20240307","20240308","20240309","20240310",
    "20240313","20240314","20240315","20240316","20240317",
    "20240320","20240321","20240322","20240323","20240324",
    "20240327","20240328","20240329","20240330",
];

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
fn fmt_ts(us: u64, date: &str) -> String {
    let s    = us / 1_000_000;
    let frac = us % 1_000_000;
    let h    = s / 3600;
    let m    = (s % 3600) / 60;
    let sec  = s % 60;
    format!("{}-{:02}:{:02}:{:02}.{:06}", date, h, m, sec, frac)
}

// ── FIX message builder ───────────────────────────────────────────────────────

/// Build a fully framed FIX 4.4 message from body fields.
///
/// `body_fields` contains everything from tag 35 onwards (with trailing `|`).
/// Pipe separators are replaced with the real FIX SOH delimiter (0x01).
/// Returns a complete SOH-delimited message with correct tag-9 and tag-10.
fn build(body_fields: &str) -> String {
    let body: String = body_fields.replace('|', "\x01");
    let body_len = body.len();
    let header   = format!("8=FIX.4.4\x019={}\x01", body_len);
    let checksum: u32 = header.bytes().chain(body.bytes())
        .map(|b| b as u32).sum::<u32>() % 256;
    format!("{}{}10={:03}\x01", header, body, checksum)
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
    day_idx:   usize, // index into TRADING_DATES / SETTL_DATES
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
            day_idx:   0,
        }
    }
    fn next_seq_c(&mut self) -> u64 { self.seq_c += 1; self.seq_c }
    fn next_seq_s(&mut self) -> u64 { self.seq_s += 1; self.seq_s }

    /// Current trading date string (cycles through TRADING_DATES).
    fn date(&self) -> &'static str {
        TRADING_DATES[self.day_idx % TRADING_DATES.len()]
    }

    /// Current T+2 settlement date string (cycles with day_idx).
    fn settl(&self) -> &'static str {
        SETTL_DATES[self.day_idx % SETTL_DATES.len()]
    }

    /// Format the session's current time_us as a FIX SendingTime.
    fn ts(&self) -> String {
        fmt_ts(self.time_us, self.date())
    }

    /// Format an arbitrary μs-since-midnight value using the session's current date.
    fn ts_at(&self, us: u64) -> String {
        fmt_ts(us, self.date())
    }

    /// Advance time by `delta_us` microseconds.
    /// When the clock crosses DAY_END_US the session rolls to the next trading day
    /// and the clock resets to DAY_START_US (preserving any small overflow).
    fn tick(&mut self, delta_us: u64) {
        let new_time = self.time_us + delta_us;
        if new_time >= DAY_END_US {
            self.day_idx += 1;
            // carry a small amount of overflow into the new day so timing stays smooth
            let overflow = new_time.saturating_sub(DAY_END_US);
            self.time_us   = DAY_START_US + overflow.min(60_000_000);
            self.hb_due_us = self.time_us + 30_000_000;
        } else {
            self.time_us = new_time;
        }
    }
}

// ── Message emitters ──────────────────────────────────────────────────────────

fn emit_logon(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    // client → server
    let ts_c = s.ts();
    let seq_c = s.next_seq_c();
    let mut b = String::with_capacity(128);
    write!(b, "35=A|34={}|49={}|52={}|56={}|98=0|108=30|",
        seq_c, s.client, ts_c, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    // server → client (12 ms later)
    s.tick(12_000);
    let ts_s = s.ts();
    let seq_s = s.next_seq_s();
    let mut b = String::with_capacity(128);
    write!(b, "35=A|34={}|49={}|52={}|56={}|98=0|108=30|",
        seq_s, s.server, ts_s, s.client).unwrap();
    emit(msgs, s.time_us, &b, total);
    s.tick(1_000_000); // 1 s quiet after logon
}

fn emit_logout(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    let ts = s.ts();
    let seq_c = s.next_seq_c();
    let mut b = String::with_capacity(80);
    write!(b, "35=5|34={}|49={}|52={}|56={}|58=End of trading day|",
        seq_c, s.client, ts, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(50_000);
    let ts = s.ts();
    let seq_s = s.next_seq_s();
    let mut b = String::with_capacity(80);
    write!(b, "35=5|34={}|49={}|52={}|56={}|58=Acknowledged|",
        seq_s, s.server, ts, s.client).unwrap();
    emit(msgs, s.time_us, &b, total);
}

fn emit_heartbeats(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize) {
    while s.time_us >= s.hb_due_us {
        let ts_c = s.ts_at(s.hb_due_us);
        let seq_c = s.next_seq_c();
        let mut b = String::with_capacity(80);
        write!(b, "35=0|34={}|49={}|52={}|56={}|",
            seq_c, s.client, ts_c, s.server).unwrap();
        emit(msgs, s.time_us, &b, total);

        let ts_s = s.ts_at(s.hb_due_us + 8_000);
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
    let ts = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b,
        "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, s.settl()
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── Quote (server → client, 80–450 μs later) ──
    let network_us = rng.urange(80, 450);
    s.tick(network_us);
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b,
        "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, s.settl()
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── NOS: client accepts quote (50 ms – 1 s decision time) ──
    let decision_us = rng.urange(50_000, 1_000_000);
    s.tick(decision_us);
    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let fill_px = if side == 1 { offer } else { bid };
    let mut b = String::with_capacity(256);
    write!(b,
        "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, s.settl(), ts_nos
    ).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ── ExecRpt: New (5–40 ms) ──
    let new_us = rng.urange(5_000, 40_000);
    s.tick(new_us);
    let ts = s.ts();
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
    let ts = s.ts();
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
    let ts = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b, "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, s.settl()).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Quote
    s.tick(rng.urange(80, 450));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, s.settl()).unwrap();
    emit(msgs, s.time_us, &b, total);

    // NOS
    s.tick(rng.urange(50_000, 800_000));
    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt New
    s.tick(rng.urange(5_000, 30_000));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt PartialFill
    s.tick(rng.urange(2_000, 20_000));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=1|39=1|55={}|54={}|38={}|14={}|151={}|31={:.5}|32={}|6={:.5}|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec2, sym, side, lots, partial, remain, fill_px, partial, fill_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Fill (remainder)
    s.tick(rng.urange(1_000, 10_000));
    let ts = s.ts();
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

    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=1|59=3|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(3_000, 25_000));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(500, 8_000));
    let ts = s.ts();
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

    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=2|44={:.5}|59=0|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, limit_px, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    s.tick(rng.urange(5_000, 40_000));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|44={:.5}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord, exec1, sym, side, lots, lots, limit_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Filled 70% of the time, expired 30%
    s.tick(rng.urange(200_000, 5_000_000));
    let ts = s.ts();
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
    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=2|44={:.5}|59=1|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord1, account, sym, side, lots, limit_px, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt New
    s.tick(rng.urange(5_000, 30_000));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54={}|38={}|14=0|151={}|44={:.5}|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl_ord1, exec1, sym, side, lots, lots, limit_px, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // OrderCancelRequest (client decides to cancel, 100ms – 2s later)
    s.tick(rng.urange(100_000, 2_000_000));
    let ts_cxl = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=F|34={}|49={}|52={}|56={}|11=CO{:08}|41=CO{:08}|37=OR{:08}|55={}|54={}|38={}|60={}|",
        seq, s.client, ts_cxl, s.server, cl_ord2, cl_ord1, ord_id, sym, side, lots, ts_cxl).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Cancelled
    s.tick(rng.urange(3_000, 20_000));
    let ts = s.ts();
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
    let ts = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(200);
    write!(b, "35=R|34={}|49={}|52={}|56={}|131=QR{:08}|146=1|55={}|38={}|54={}|64={}|",
        seq, s.client, ts, s.server, qreq_id, sym, lots, side, s.settl()).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Quote
    s.tick(rng.urange(80, 450));
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=S|34={}|49={}|52={}|56={}|117=QT{:08}|131=QR{:08}|55={}|132={:.5}|133={:.5}|134={}|135={}|64={}|",
        seq, s.server, ts, s.client, quot_id, qreq_id, sym, bid, offer, lots, lots, s.settl()).unwrap();
    emit(msgs, s.time_us, &b, total);

    // NOS
    s.tick(rng.urange(50_000, 600_000));
    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(256);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1={}|55={}|54={}|38={}|40=D|44={:.5}|117=QT{:08}|59=4|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl_ord, account, sym, side, lots, fill_px, quot_id, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Rejected
    s.tick(rng.urange(4_000, 15_000));
    let ts = s.ts();
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

// ── Validator test-fixture generator ─────────────────────────────────────────
//
// Produces two files:
//   fix_validator_test.log       — one FIX message per line (pipe-delimited)
//   fix_validator_test.manifest  — tab-separated: line | exp_errors | exp_warnings | category | description
//
// Each message is crafted to trigger exactly the documented violation (or none for VALID cases).
// Usage:  cargo run --release --bin gen_fix -- --mode validator

const VTEST_LOG:      &str = "fix_validator_test.log";
const VTEST_MANIFEST: &str = "fix_validator_test.manifest";

// Fixed header fields used in all test messages.
const VT_TS: &str  = "20240315-12:00:00.000000";
const VT_CLI: &str = "CITIFX";
const VT_SRV: &str = "FXECN";
const VT_SYM: &str = "EUR/USD";
/// Build a complete FIX message but override `9` (BodyLength) with `bl_override`
/// and `10` (CheckSum) with `cs_override`.  Pass `None` to compute the real values.
fn build_with_overrides(body_fields: &str, bl_override: Option<u32>, cs_override: Option<u32>) -> String {
    let body_len = bl_override.unwrap_or(body_fields.len() as u32);
    let header   = format!("8=FIX.4.4|9={}|", body_len);
    let checksum: u32 = if let Some(cs) = cs_override {
        cs
    } else {
        header.bytes().chain(body_fields.bytes()).map(|b| b as u32).sum::<u32>() % 256
    };
    format!("{}{}10={:03}|", header, body_fields, checksum)
}

/// A single validator test case.
struct VCase {
    msg:          String,
    exp_errors:   u32,
    exp_warnings: u32,
    category:     &'static str,
    description:  String,
}

impl VCase {
    fn new(msg: String, exp_errors: u32, exp_warnings: u32, category: &'static str, description: impl Into<String>) -> Self {
        Self { msg, exp_errors, exp_warnings, category, description: description.into() }
    }
}

fn gen_validator_test(log_path: &str, manifest_path: &str) {
    let mut cases: Vec<VCase> = Vec::new();
    let mut seq_id = 1u64;

    macro_rules! vcase {
        ($msg:expr, $errs:expr, $warns:expr, $cat:expr, $desc:expr) => {
            cases.push(VCase::new($msg, $errs, $warns, $cat, $desc));
        }
    }

    // Helper: body for a complete, valid NOS (Market order)
    let valid_nos_market = |seq: u64| -> String {
        build_with_overrides(
            &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|",
                seq, VT_CLI, VT_TS, VT_SRV, seq, VT_SYM, VT_TS),
            None, None,
        )
    };
    let valid_nos_limit = |seq: u64| -> String {
        build_with_overrides(
            &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=2|44=1.08500|54=1|55={}|60={}|",
                seq, VT_CLI, VT_TS, VT_SRV, seq, VT_SYM, VT_TS),
            None, None,
        )
    };
    let valid_er_fill = |seq: u64, cl: u64| -> String {
        build_with_overrides(
            &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=1000000|151=0|31=1.08500|32=1000000|6=1.08500|60={}|",
                seq, VT_SRV, VT_TS, VT_CLI, seq, cl, seq, VT_SYM, VT_TS),
            None, None,
        )
    };
    let valid_logon = |seq: u64| -> String {
        build_with_overrides(
            &format!("35=A|34={}|49={}|52={}|56={}|98=0|108=30|", seq, VT_CLI, VT_TS, VT_SRV),
            None, None,
        )
    };

    // ── Section 1: VALID messages — must produce 0 errors, 0 warnings ──────────

    {
        let s = seq_id; seq_id += 1;
        vcase!(valid_logon(s), 0, 0, "VALID", "Valid Logon (35=A)");
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=0|34={}|49={}|52={}|56={}|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            0, 0, "VALID", "Valid Heartbeat (35=0)"
        );
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(valid_nos_market(s), 0, 0, "VALID", "Valid NOS Market order (35=D, OrdType=1)");
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(valid_nos_limit(s), 0, 0, "VALID", "Valid NOS Limit order (35=D, OrdType=2, Price present)");
    }
    {
        let s = seq_id; seq_id += 1;
        // NOS with OrdType=D (Previously Quoted) — must have QuoteID
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=D|44=1.08500|117=QT00000001|54=2|55={}|59=4|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 0, "VALID", "Valid NOS PreviouslyQuoted RFQ fill (35=D, OrdType=D, QuoteID present)"
        );
    }
    {
        let s = seq_id; seq_id += 1;
        let cl = s - 1;
        vcase!(valid_er_fill(s, cl), 0, 0, "VALID", "Valid ExecutionReport Fill (35=8, ExecType=F)");
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=R|34={}|49={}|52={}|56={}|131=QR00000001|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            0, 0, "VALID", "Valid QuoteRequest (35=R)"
        );
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=S|34={}|49={}|52={}|56={}|117=QT00000001|55={}|132=1.08475|133=1.08525|",
                    s, VT_SRV, VT_TS, VT_CLI, VT_SYM),
                None, None,
            ),
            0, 0, "VALID", "Valid Quote (35=S)"
        );
    }
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=5|34={}|49={}|52={}|56={}|58=End of session|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            0, 0, "VALID", "Valid Logout (35=5)"
        );
    }
    // Valid ER New (not a fill — no LastPx/LastQty required)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 0, "VALID", "Valid ExecutionReport New (35=8, ExecType=0)"
        );
    }
    // Valid ER Canceled
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=4|39=4|55={}|54=1|38=1000000|14=0|151=0|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 0, "VALID", "Valid ExecutionReport Canceled (35=8, ExecType=4, OrdStatus=4)"
        );
    }

    // ── Section 2: MISSING required header tags ──────────────────────────────

    // Missing MsgSeqNum (34)
    {
        vcase!(
            build_with_overrides(
                &format!("35=0|49={}|52={}|56={}|", VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            1, 0, "MISSING_HEADER_TAG", "Missing MsgSeqNum (tag 34)"
        );
    }
    // Missing SenderCompID (49)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=0|34={}|52={}|56={}|", s, VT_TS, VT_SRV),
                None, None,
            ),
            1, 0, "MISSING_HEADER_TAG", "Missing SenderCompID (tag 49)"
        );
    }
    // Missing SendingTime (52)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=0|34={}|49={}|56={}|", s, VT_CLI, VT_SRV),
                None, None,
            ),
            1, 0, "MISSING_HEADER_TAG", "Missing SendingTime (tag 52)"
        );
    }
    // Missing TargetCompID (56)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=0|34={}|49={}|52={}|", s, VT_CLI, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_HEADER_TAG", "Missing TargetCompID (tag 56)"
        );
    }

    // ── Section 3: MISSING required body tags ────────────────────────────────

    // NOS missing ClOrdID (11)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|21=1|38=1000000|40=1|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing ClOrdID (tag 11)"
        );
    }
    // NOS missing HandlInst (21)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|38=1000000|40=1|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing HandlInst (tag 21)"
        );
    }
    // NOS missing OrderQty (38)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|40=1|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing OrderQty (tag 38)"
        );
    }
    // NOS missing OrdType (40)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing OrdType (tag 40)"
        );
    }
    // NOS missing Side (54)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing Side (tag 54)"
        );
    }
    // NOS missing Symbol (55)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing Symbol (tag 55)"
        );
    }
    // NOS missing TransactTime (60)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "NOS missing TransactTime (tag 60)"
        );
    }
    // ER missing ExecType (150)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|39=0|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "ER missing ExecType (tag 150)"
        );
    }
    // ER missing LeavesQty (151)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54=1|38=1000000|14=0|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "ER missing LeavesQty (tag 151)"
        );
    }
    // ER missing OrdStatus (39)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "ER missing OrdStatus (tag 39)"
        );
    }
    // Logon missing EncryptMethod (98)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=A|34={}|49={}|52={}|56={}|108=30|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "Logon missing EncryptMethod (tag 98)"
        );
    }
    // Logon missing HeartBtInt (108)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=A|34={}|49={}|52={}|56={}|98=0|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            1, 0, "MISSING_REQUIRED_TAG", "Logon missing HeartBtInt (tag 108)"
        );
    }

    // ── Section 4: INVALID enum values ──────────────────────────────────────

    // Side = 0 (0 is not a valid Side value; 1=Buy, 2=Sell, etc.)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=0|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "NOS Side=0 (invalid; valid: 1-9)"
        );
    }
    // Side = X
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=X|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "NOS Side=X (not a valid FIX Side value)"
        );
    }
    // OrdType = 0 (not in FIX 4.4 valid OrdType set)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=0|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "NOS OrdType=0 (invalid; FIX 4.4 valid: 1-9, A-P)"
        );
    }
    // TimeInForce = X
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|59=X|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "NOS TimeInForce=X (invalid)"
        );
    }
    // OrdStatus = X
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=X|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "ER OrdStatus=X (invalid)"
        );
    }
    // ExecType = Z
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=Z|39=0|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "ER ExecType=Z (invalid)"
        );
    }
    // EncryptMethod = 9 (only 0-6 valid)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=A|34={}|49={}|52={}|56={}|98=9|108=30|", s, VT_CLI, VT_TS, VT_SRV),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "Logon EncryptMethod=9 (invalid; valid: 0-6)"
        );
    }
    // HandlInst = 9 (only 1-3 valid)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=9|38=1000000|40=1|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "INVALID_ENUM", "NOS HandlInst=9 (invalid; valid: 1-3)"
        );
    }

    // ── Section 5: CONDITIONAL tags missing ─────────────────────────────────

    // Limit NOS missing Price (44)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=2|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "NOS OrdType=2 (Limit) but Price (tag 44) missing"
        );
    }
    // Stop NOS missing StopPx (99)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=3|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "NOS OrdType=3 (Stop) but StopPx (tag 99) missing"
        );
    }
    // GTD NOS missing ExpireDate (432)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=2|44=1.08500|54=1|55={}|59=6|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "NOS TimeInForce=6 (GTD) but ExpireDate (tag 432) missing"
        );
    }
    // ER ExecType=F missing LastPx (31)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=1000000|151=0|32=1000000|6=1.08500|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "ER ExecType=F (Trade) but LastPx (tag 31) missing"
        );
    }
    // ER ExecType=F missing LastQty (32)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=1000000|151=0|31=1.08500|6=1.08500|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "ER ExecType=F (Trade) but LastQty (tag 32) missing"
        );
    }
    // RFQ NOS (OrdType=D) missing QuoteID (117)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=D|44=1.08500|54=1|55={}|59=4|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "NOS OrdType=D (PreviouslyQuoted) but QuoteID (tag 117) missing"
        );
    }
    // ER ExecType=G (TradeCorrect) missing ExecRefID (19)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=G|39=2|55={}|54=1|38=1000000|14=1000000|151=0|31=1.08500|32=1000000|6=1.08500|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONDITIONAL_TAG_MISSING", "ER ExecType=G (TradeCorrect) but ExecRefID (tag 19) missing"
        );
    }

    // ── Section 6: CONSISTENCY violations ───────────────────────────────────

    // ER: LeavesQty(200000) + CumQty(700000) = 900000 ≠ OrderQty(1000000)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=700000|151=200000|31=1.08500|32=700000|6=1.08500|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            1, 0, "CONSISTENCY_FILL_QTY", "ER ExecType=F: LeavesQty(200000) + CumQty(700000) = 900000 ≠ OrderQty(1000000)"
        );
    }
    // ER: OrdStatus=2 (Filled) but LeavesQty = 50000 (must be 0)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=950000|151=50000|31=1.08500|32=950000|6=1.08500|60={}|",
                    s, VT_SRV, VT_TS, VT_CLI, s, s, s, VT_SYM, VT_TS),
                None, None,
            ),
            2, 0, "CONSISTENCY_FILLED_LEAVES", "ER OrdStatus=2 (Filled) but LeavesQty=50000 (must be 0); also fails fill qty check"
        );
    }

    // ── Section 7: DUPLICATE tags ────────────────────────────────────────────

    // Duplicate Symbol (55)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|55=GBP/USD|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 1, "DUPLICATE_TAG", "NOS with duplicate Symbol (tag 55) — second value GBP/USD"
        );
    }
    // Duplicate ClOrdID (11)
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|11=CO99999999|21=1|38=1000000|40=1|54=1|55={}|60={}|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 1, "DUPLICATE_TAG", "NOS with duplicate ClOrdID (tag 11)"
        );
    }

    // ── Section 8: CUSTOM / EXTENDED tags ───────────────────────────────────

    // Custom tag >= 5000 → Warning
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|9001=INTERNAL-REF-001|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 1, "CUSTOM_TAG", "NOS with proprietary tag 9001 (custom, >= 5000) — should warn only"
        );
    }
    // Multiple custom tags
    {
        let s = seq_id; seq_id += 1;
        vcase!(
            build_with_overrides(
                &format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|5001=VENUE-REF|5002=ALGO-STRAT|",
                    s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS),
                None, None,
            ),
            0, 2, "CUSTOM_TAG", "NOS with two proprietary tags 5001 and 5002 — two warnings"
        );
    }

    // ── Section 9: FIX VERSION violations ───────────────────────────────────

    // FIX.4.2 message containing tag 453 (NoPartyIDs, introduced in FIX.4.4)
    {
        let s = seq_id; seq_id += 1;
        let body = format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|453=1|448=CITIFX|447=D|452=1|",
            s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS);
        let body_len = body.len() as u32;
        let header = format!("8=FIX.4.2|9={}|", body_len);
        let checksum: u32 = header.bytes().chain(body.bytes()).map(|b| b as u32).sum::<u32>() % 256;
        vcase!(
            format!("{}{}10={:03}|", header, body, checksum),
            0, 3, "VERSION_VIOLATION", "FIX.4.2 message with tags 453/448/452 (introduced in FIX.4.4) — three version warnings"
        );
    }
    // FIX.4.2 message with tag 571 (TradeReportID, introduced in FIX.4.4)
    {
        let s = seq_id; seq_id += 1;
        let body = format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|571=TRD-001|",
            s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS);
        let body_len = body.len() as u32;
        let header = format!("8=FIX.4.2|9={}|", body_len);
        let checksum: u32 = header.bytes().chain(body.bytes()).map(|b| b as u32).sum::<u32>() % 256;
        vcase!(
            format!("{}{}10={:03}|", header, body, checksum),
            0, 1, "VERSION_VIOLATION", "FIX.4.2 message with tag 571 (TradeReportID, introduced in FIX.4.4)"
        );
    }

    // ── Section 10: CHECKSUM and BODYLENGTH ─────────────────────────────────

    // Wrong checksum (computed + 1)
    {
        let s = seq_id; seq_id += 1;
        let body = format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|",
            s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS);
        let body_len = body.len() as u32;
        let header = format!("8=FIX.4.4|9={}|", body_len);
        let real_cs: u32 = header.bytes().chain(body.bytes()).map(|b| b as u32).sum::<u32>() % 256;
        let bad_cs = (real_cs + 1) % 256;
        vcase!(
            format!("{}{}10={:03}|", header, body, bad_cs),
            1, 0, "CHECKSUM_MISMATCH", format!("Correct checksum is {:03} but tag 10 has {:03}", real_cs, bad_cs)
        );
    }
    // Wrong BodyLength (set to 5, real is much larger)
    {
        let s = seq_id; seq_id += 1;
        let body = format!("35=D|34={}|49={}|52={}|56={}|11=CO{:08}|21=1|38=1000000|40=1|54=1|55={}|60={}|",
            s, VT_CLI, VT_TS, VT_SRV, s, VT_SYM, VT_TS);
        vcase!(
            build_with_overrides(&body, Some(5), None),
            1, 0, "BODY_LENGTH_MISMATCH", format!("BodyLength set to 5 but actual body is {} bytes", body.len())
        );
    }
    // Correct checksum and BodyLength (control — must be clean)
    {
        let s = seq_id;
        vcase!(valid_nos_market(s), 0, 0, "VALID", "Control: valid NOS with correct checksum and BodyLength");
    }

    // ── Write output files ────────────────────────────────────────────────────

    let log_file = File::create(log_path).expect("Cannot create log file");
    let mut log  = BufWriter::new(log_file);

    let man_file  = File::create(manifest_path).expect("Cannot create manifest file");
    let mut manifest = BufWriter::new(man_file);

    writeln!(manifest, "# FIX Validator test fixture manifest").unwrap();
    writeln!(manifest, "# Generated by gen_fix --mode validator").unwrap();
    writeln!(manifest, "# Columns: line_number | expected_errors | expected_warnings | category | description").unwrap();
    writeln!(manifest, "#").unwrap();
    writeln!(manifest, "# Load {} into AiFIXParser, switch to Validate tab,", log_path).unwrap();
    writeln!(manifest, "# click 'Batch Validate', and compare results against this manifest.").unwrap();
    writeln!(manifest, "#").unwrap();

    let total = cases.len();
    for (i, case) in cases.iter().enumerate() {
        log.write_all(case.msg.as_bytes()).unwrap();
        log.write_all(b"\n").unwrap();
        writeln!(manifest, "{}|{}|{}|{}|{}",
            i + 1, case.exp_errors, case.exp_warnings, case.category, case.description
        ).unwrap();
    }
    log.flush().unwrap();
    manifest.flush().unwrap();

    eprintln!("Validator test fixture: {} messages written to {}", total, log_path);
    eprintln!("Manifest written to {}", manifest_path);
    eprintln!();
    eprintln!("Summary:");
    let valid_count = cases.iter().filter(|c| c.exp_errors == 0 && c.exp_warnings == 0).count();
    let error_count = cases.iter().filter(|c| c.exp_errors > 0).count();
    let warn_count  = cases.iter().filter(|c| c.exp_warnings > 0 && c.exp_errors == 0).count();
    eprintln!("  {:3} VALID (0 errors, 0 warnings)", valid_count);
    eprintln!("  {:3} should produce ERRORS", error_count);
    eprintln!("  {:3} should produce WARNINGS only", warn_count);
    eprintln!();
    eprintln!("To use: Load {} in AiFIXParser → Validate tab → Batch Validate", log_path);
}

// ── Session-health test generator ─────────────────────────────────────────────
//
// Usage:  cargo run --release --bin gen_fix -- --mode health
// Output: fixtures/fix_health_test_100k.log
//
// Produces ~100 k FIX messages on a single session (CITIFX/FXECN) with
// deliberate anomalies at known milestones to exercise all 7 detection rules.

fn ensure_dir(dir: &str) {
    std::fs::create_dir_all(dir)
        .unwrap_or_else(|e| eprintln!("Warning: cannot create dir '{}': {}", dir, e));
}

fn out_path(dir: &str, filename: &str) -> String {
    format!("{}/{}", dir.trim_end_matches('/'), filename)
}

fn find_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter().position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

// ── Health injection helpers ───────────────────────────────────────────────────

/// Skip `skip` client sequence numbers, leaving a gap the detector will catch.
fn inject_seq_gap(s: &mut Session, skip: u64) {
    s.seq_c += skip;
}

/// Emit a heartbeat, advance time by `gap_sec` seconds without emitting another,
/// then emit the next heartbeat — creating a gap that exceeds the 30 s threshold.
fn inject_heartbeat_gap(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize, gap_sec: u64) {
    // Anchor heartbeat
    let ts = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(80);
    write!(b, "35=0|34={}|49={}|52={}|56={}|", seq, s.client, ts, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Silence — advance without calling emit_heartbeats
    s.tick(gap_sec * 1_000_000);

    // Second heartbeat with large gap
    let ts = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(80);
    write!(b, "35=0|34={}|49={}|52={}|56={}|", seq, s.client, ts, s.server).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Reset heartbeat timer so normal cycle resumes
    s.hb_due_us = s.time_us + 30_000_000;
}

/// Emit Logout → 5 s pause → Logon, simulating a mid-session reconnect.
/// If `reset_seq` the sequence numbers are zeroed before the new Logon (seq=1 = reset).
fn inject_reconnect(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize, reset_seq: bool) {
    emit_logout(msgs, s, total);
    s.tick(5_000_000);
    if reset_seq {
        s.seq_c = 0;
        s.seq_s = 0;
    }
    emit_logon(msgs, s, total);
}

/// Emit `count` heartbeats within a single second — triggers message-rate burst detection.
fn inject_burst(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize, count: usize) {
    let start_us = s.time_us;
    for i in 0..count {
        s.time_us = (start_us + i as u64 * 1_000_000 / count as u64).min(DAY_END_US);
        let ts = s.ts();
        let seq = s.next_seq_c();
        let mut b = String::with_capacity(80);
        write!(b, "35=0|34={}|49={}|52={}|56={}|", seq, s.client, ts, s.server).unwrap();
        emit(msgs, s.time_us, &b, total);
    }
    s.time_us = (start_us + 1_100_000).min(DAY_END_US);
    s.hb_due_us = s.time_us + 30_000_000;
}

/// NOS → Fill → CancelRequest (arrives after fill — late cancel).
fn inject_late_cancel(msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids, total: &mut usize) {
    let sym    = "EUR/USD";
    let cl1    = ids.next_cord();
    let cl2    = ids.next_cord();
    let ord_id = ids.next_ord();
    let exec1  = ids.next_exec();
    let exec2  = ids.next_exec();

    // NOS Market
    let ts_nos = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=D|34={}|49={}|52={}|56={}|11=CO{:08}|1=CITI-FX-01|55={}|54=1|38=1000000|40=1|59=3|64={}|60={}|21=1|",
        seq, s.client, ts_nos, s.server, cl1, sym, s.settl(), ts_nos).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt New
    s.tick(5_000);
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=0|39=0|55={}|54=1|38=1000000|14=0|151=1000000|6=0|60={}|",
        seq, s.server, ts, s.client, ord_id, cl1, exec1, sym, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // ExecRpt Fill
    s.tick(3_000);
    let ts = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(256);
    write!(b, "35=8|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|17=EX{:08}|150=F|39=2|55={}|54=1|38=1000000|14=1000000|151=0|31=1.08500|32=1000000|6=1.08500|60={}|",
        seq, s.server, ts, s.client, ord_id, cl1, exec2, sym, ts).unwrap();
    emit(msgs, s.time_us, &b, total);

    // Cancel request after fill (45 ms later — too late)
    s.tick(45_000);
    let ts_cxl = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(220);
    write!(b, "35=F|34={}|49={}|52={}|56={}|11=CO{:08}|41=CO{:08}|37=OR{:08}|55={}|54=1|38=1000000|60={}|",
        seq, s.client, ts_cxl, s.server, cl2, cl1, ord_id, sym, ts_cxl).unwrap();
    emit(msgs, s.time_us, &b, total);
}

/// Emit an OrderCancelReject (35=9) with the given tag-102 reason code.
fn inject_rejected_cancel(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids, total: &mut usize,
    reason_code: &str,
) {
    let cl_ord      = ids.next_cord();
    let orig_cl_ord = ids.next_cord();
    let ord_id      = ids.next_ord();
    let ts  = s.ts();
    let seq = s.next_seq_s();
    let mut b = String::with_capacity(220);
    write!(b, "35=9|34={}|49={}|52={}|56={}|37=OR{:08}|11=CO{:08}|41=CO{:08}|102={}|58=Cancel rejected|",
        seq, s.server, ts, s.client, ord_id, cl_ord, orig_cl_ord, reason_code).unwrap();
    emit(msgs, s.time_us, &b, total);
    s.tick(500_000);
}

/// Emit a ResendRequest (35=2) for the given sequence range.
fn inject_resend_request(
    msgs: &mut MsgBuf, s: &mut Session, total: &mut usize,
    begin_seq: u64, end_seq: u64,
) {
    let ts  = s.ts();
    let seq = s.next_seq_c();
    let mut b = String::with_capacity(120);
    write!(b, "35=2|34={}|49={}|52={}|56={}|7={}|16={}|",
        seq, s.client, ts, s.server, begin_seq, end_seq).unwrap();
    emit(msgs, s.time_us, &b, total);
    s.tick(2_000_000);
}

/// Randomly emit a ResendRequest with probability 1-in-`interval`.
fn maybe_resend(msgs: &mut MsgBuf, s: &mut Session, total: &mut usize, rng: &mut Rng, interval: u64) {
    if rng.next() % interval == 0 {
        let begin = s.seq_c.saturating_sub(5).max(1);
        let end   = s.seq_c;
        inject_resend_request(msgs, s, total, begin, end);
    }
}

/// Run one random normal trading workflow on session `s`.
fn run_one_workflow(
    msgs: &mut MsgBuf, s: &mut Session, ids: &mut Ids,
    rng: &mut Rng, total: &mut usize,
) {
    let sym_idx = rng.next() as usize % SYMBOLS.len();
    match rng.next() % 5 {
        0 => workflow_rfq_fill(msgs, s, ids, rng, sym_idx, total),
        1 => workflow_rfq_partial(msgs, s, ids, rng, sym_idx, total),
        2 => workflow_market_fill(msgs, s, ids, rng, sym_idx, total),
        3 => workflow_limit_order(msgs, s, ids, rng, sym_idx, total),
        _ => workflow_cancel(msgs, s, ids, rng, sym_idx, total),
    }
}

/// Generate ~`target` messages on a single CITIFX/FXECN session with deliberate
/// anomalies injected at known milestones:
///
///   ~  8 000  →  sequence gap × 2  (skip 10 seqnums each)
///   ~ 12 000  →  heartbeat gap × 2  (90 s and 120 s silence)
///   ~ 20 000  →  reconnect #1  (mid-session, no seq reset)
///   ~ 35 000  →  sequence gap #3  (skip 15 seqnums)
///   ~ 50 000  →  message-rate burst × 2  (150 and 180 msgs in one second)
///   ~ 55 000  →  late cancels × 5
///   ~ 65 000  →  reconnect #2  (seq reset — new session)
///   ~ 75 000  →  rejected cancels × 5  (codes 0, 1, 3, 0, 1)
///   throughout  →  resend requests (≈1 per 30 workflows, ~600 total)
fn gen_health_test(output: &str, target: usize) {
    let mut msgs: MsgBuf = Vec::with_capacity(target + 8192);
    let mut total = 0usize;
    let mut ids   = Ids::new();
    let mut rng   = Rng(0xFEED_BABE_CAFE_DEAD);

    let mut s = Session::new(0, 0); // CITIFX / FXECN

    emit_logon(&mut msgs, &mut s, &mut total);

    // ── Phase 1 → milestone 1a ─────────────────────────────────────────────
    while total < 8_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_seq_gap(&mut s, 10);
    for _ in 0..100 { // a short block between the two gaps
        if s.time_us >= DAY_END_US { break; }
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
    }
    inject_seq_gap(&mut s, 10);

    // ── Phase 2 → milestone 2 ─────────────────────────────────────────────
    while total < 12_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_heartbeat_gap(&mut msgs, &mut s, &mut total, 90);
    s.tick(30_000_000);
    inject_heartbeat_gap(&mut msgs, &mut s, &mut total, 120);

    // ── Phase 3 → milestone 3 ─────────────────────────────────────────────
    while total < 20_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_reconnect(&mut msgs, &mut s, &mut total, false);

    // ── Phase 4 → milestone 4 ─────────────────────────────────────────────
    while total < 35_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_seq_gap(&mut s, 15);

    // ── Phase 5 → milestone 5 ─────────────────────────────────────────────
    while total < 50_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_burst(&mut msgs, &mut s, &mut total, 150);
    s.tick(5_000_000);
    inject_burst(&mut msgs, &mut s, &mut total, 180);

    // ── Phase 6 → milestone 6 ─────────────────────────────────────────────
    while total < 55_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    for _ in 0..5 {
        inject_late_cancel(&mut msgs, &mut s, &mut ids, &mut total);
        s.tick(2_000_000);
    }

    // ── Phase 7 → milestone 7 ─────────────────────────────────────────────
    while total < 65_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    inject_reconnect(&mut msgs, &mut s, &mut total, true); // seq reset

    // ── Phase 8 → milestone 8 ─────────────────────────────────────────────
    while total < 75_000 && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }
    for &code in &["0", "1", "3", "0", "1"] {
        inject_rejected_cancel(&mut msgs, &mut s, &mut ids, &mut total, code);
        s.tick(1_000_000);
    }

    // ── Phase 9 → target ──────────────────────────────────────────────────
    while total < target.saturating_sub(4) && s.time_us < DAY_END_US {
        emit_heartbeats(&mut msgs, &mut s, &mut total);
        run_one_workflow(&mut msgs, &mut s, &mut ids, &mut rng, &mut total);
        maybe_resend(&mut msgs, &mut s, &mut total, &mut rng, 30);
    }

    emit_heartbeats(&mut msgs, &mut s, &mut total);
    emit_logout(&mut msgs, &mut s, &mut total);

    msgs.sort_unstable_by_key(|(ts, _)| *ts);

    let file = File::create(output).expect("Cannot create health test output file");
    let mut w = BufWriter::with_capacity(4 * 1024 * 1024, file);
    for (_, msg) in &msgs {
        w.write_all(msg.as_bytes()).unwrap();
        w.write_all(b"\n").unwrap();
    }
    w.flush().unwrap();
    eprintln!("Health test: {} messages written to {}", total, output);
    eprintln!("Anomalies: 3× seq-gap, 2× hb-gap, 2× reconnect, 2× rate-burst, 5× late-cancel, 5× rejected-cancel, ~{}× resends",
        total / 30 / 5);
}

// ── Billion-message streaming generator ──────────────────────────────────────

/// Generate LARGE_TARGET (100 M) messages and write them directly to `output`
/// without holding more than `FLUSH_BATCH` messages in RAM at once.
///
/// Strategy: all 8 sessions run in round-robin order (one workflow each turn).
/// This keeps each session's clock advancing at a similar rate so the rolling
/// `FLUSH_BATCH`-sized sort produces output that is approximately chronological.
/// Day-boundary cycling in `Session::tick()` keeps timestamps realistic across
/// the ~1 700 synthetic trading days needed to generate 100 million messages.
///
/// Estimated file size: ~19–22 GB (SOH-delimited).
fn gen_large_streaming(output: &str) {
    const FLUSH_BATCH: usize = 100_000; // ~30 MB per batch, sort then flush

    let file = File::create(output).expect("Cannot create billion output file");
    let mut w = BufWriter::with_capacity(8 * 1024 * 1024, file);

    let mut total = 0usize;
    let mut ids   = Ids::new();
    let mut rng   = Rng(0xCAFE_BABE_1337_0000);
    let n_sessions = SESSIONS.len();

    let mut sessions: Vec<Session> = (0..n_sessions)
        .map(|i| Session::new(i, (i as u64) * 250_000))
        .collect();

    let mut buf: MsgBuf = Vec::with_capacity(FLUSH_BATCH + 256);

    // Logons for every session
    for s in &mut sessions {
        emit_logon(&mut buf, s, &mut total);
    }

    let sym_weights = [25u64, 20, 20, 10, 8, 8, 4, 8, 5, 5, 4, 3];
    let sym_total: u64 = sym_weights.iter().sum();

    let mut si = 0usize;
    while total < LARGE_TARGET {
        let s = &mut sessions[si % n_sessions];

        emit_heartbeats(&mut buf, s, &mut total);

        // Weighted symbol pick
        let mut r = rng.next() % sym_total;
        let mut sym_idx = 0;
        for (i, &w_) in sym_weights.iter().enumerate() {
            if r < w_ { sym_idx = i; break; }
            r -= w_;
        }

        // Weighted workflow pick (same distribution as 1 M generator)
        match rng.next() % 100 {
            0..=44  => workflow_rfq_fill(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
            45..=59 => workflow_rfq_partial(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
            60..=74 => workflow_market_fill(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
            75..=84 => workflow_limit_order(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
            85..=92 => workflow_cancel(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
            _       => workflow_rfq_reject(&mut buf, s, &mut ids, &mut rng, sym_idx, &mut total),
        }

        if rng.next() % 30 == 0 {
            s.tick(rng.urange(1_000_000, 8_000_000));
        }

        // Flush sorted batch to disk
        if buf.len() >= FLUSH_BATCH {
            buf.sort_unstable_by_key(|(ts, _)| *ts);
            for (_, msg) in &buf {
                w.write_all(msg.as_bytes()).unwrap();
                w.write_all(b"\n").unwrap();
            }
            buf.clear();
        }

        si += 1;

        if total % 10_000_000 == 0 && total > 0 {
            let pct = total * 100 / LARGE_TARGET;
            eprintln!("  {} M / 100 M  ({}%)", total / 1_000_000, pct);
        }
    }

    // Logouts
    for s in &mut sessions {
        emit_logout(&mut buf, s, &mut total);
    }

    // Final flush
    buf.sort_unstable_by_key(|(ts, _)| *ts);
    for (_, msg) in &buf {
        w.write_all(msg.as_bytes()).unwrap();
        w.write_all(b"\n").unwrap();
    }
    w.flush().unwrap();

    eprintln!("Done. {} messages written to {}", total, output);
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Usage:
    //   cargo run --release --bin gen_fix                              # 1 M msgs → fixtures/fix_test_1m.log
    //   cargo run --release --bin gen_fix -- --mode health             # 100 k health test
    //   cargo run --release --bin gen_fix -- --mode validator          # validator fixture
    //   cargo run --release --bin gen_fix -- --mode large              # 100 M msgs → fixtures/fix_test_100m.log
    //   cargo run --release --bin gen_fix -- --output-dir /tmp         # custom output directory
    //   cargo run --release --bin gen_fix -- 100000 fix_100k.log       # positional (legacy)
    let args: Vec<String> = std::env::args().collect();

    // Output directory (default: fixtures/)
    let dir = find_flag(&args, "--output-dir").unwrap_or(DEFAULT_DIR);
    ensure_dir(dir);

    // --mode dispatch
    if let Some(mode_idx) = args.iter().position(|a| a == "--mode") {
        match args.get(mode_idx + 1).map(|s| s.as_str()) {
            Some("validator") => {
                let log_path = out_path(dir, VTEST_LOG);
                let man_path = out_path(dir, VTEST_MANIFEST);
                gen_validator_test(&log_path, &man_path);
                return;
            }
            Some("health") => {
                let output = out_path(dir, HEALTH_OUTPUT);
                gen_health_test(&output, 100_000);
                return;
            }
            Some("large") => {
                let output = out_path(dir, LARGE_OUTPUT);
                eprintln!("Generating 100 million FIX messages → {}", output);
                eprintln!("Estimated size: ~20 GB.");
                gen_large_streaming(&output);
                return;
            }
            Some(m) => {
                eprintln!("Unknown mode '{}'. Valid modes: validator, health, large", m);
                std::process::exit(1);
            }
            None => {
                eprintln!("--mode requires an argument");
                std::process::exit(1);
            }
        }
    }

    // Default: 1 M message generator
    let target: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(DEFAULT_TARGET);
    let output = args.get(2)
        .map(|s| s.to_string())
        .unwrap_or_else(|| out_path(dir, DEFAULT_OUTPUT));

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

    let file = File::create(&output).expect("Cannot create output file");
    let mut w = BufWriter::with_capacity(4 * 1024 * 1024, file);
    for (_, msg) in &msgs {
        w.write_all(msg.as_bytes()).unwrap();
        w.write_all(b"\n").unwrap();
    }
    w.flush().unwrap();
    eprintln!("Done. {} messages written to {}", total, output);
}
