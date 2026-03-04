#!/usr/bin/env python3
"""Generate 1,000,000 SOH-delimited FIX 4.4 messages for benchmarking."""

import random
import sys
import time

SOH = "\x01"
OUT = "sample_1m_soh.fix"
TARGET = 1_000_000

SYMBOLS  = ["MSFT", "AAPL", "GOOG", "AMZN", "TSLA", "SPY", "QQQ", "NVDA", "META", "ORCL"]
SENDERS  = ["BANZAI", "CLIENT1", "ALGO01", "HFT_A", "MM_DESK"]
TARGETS  = ["EXEC", "VENUE1", "BROKER2", "DARK1", "LIT_A"]
SIDES    = ["1", "2"]  # Buy / Sell

def soh_join(fields):
    return SOH.join(fields) + SOH

def checksum(msg: str) -> str:
    return str(sum(msg.encode("latin-1")) % 256).zfill(3)

def logon(seq, sender, target, ts):
    body = soh_join([
        f"35=A", f"34={seq}", f"49={sender}", f"52={ts}",
        f"56={target}", "98=0", "108=30",
    ])
    hdr = f"8=FIX.4.4{SOH}9={len(body)}{SOH}"
    raw = hdr + body
    return raw + f"10={checksum(raw)}{SOH}"

def heartbeat(seq, sender, target, ts):
    body = soh_join([
        f"35=0", f"34={seq}", f"49={sender}", f"52={ts}", f"56={target}",
    ])
    hdr = f"8=FIX.4.4{SOH}9={len(body)}{SOH}"
    raw = hdr + body
    return raw + f"10={checksum(raw)}{SOH}"

def new_order(seq, sender, target, ts, cl_ord_id, symbol, side, qty, price):
    body = soh_join([
        f"35=D", f"34={seq}", f"49={sender}", f"52={ts}", f"56={target}",
        f"11={cl_ord_id}", "21=1", f"38={qty}", "40=2",
        f"44={price:.2f}", f"54={side}", f"55={symbol}", "59=0",
    ])
    hdr = f"8=FIX.4.4{SOH}9={len(body)}{SOH}"
    raw = hdr + body
    return raw + f"10={checksum(raw)}{SOH}"

def exec_report(seq, sender, target, ts, cl_ord_id, ord_id, exec_id,
                symbol, side, qty, price, exec_type, ord_status, leaves_qty, cum_qty, avg_px):
    body = soh_join([
        f"35=8", f"34={seq}", f"49={sender}", f"52={ts}", f"56={target}",
        f"6={avg_px:.2f}", f"11={cl_ord_id}", f"14={cum_qty}", f"17={exec_id}",
        "20=0", f"31={price:.2f}", f"32={qty}", f"37={ord_id}",
        f"38={qty}", f"39={ord_status}", f"54={side}", f"55={symbol}",
        f"150={exec_type}", f"151={leaves_qty}",
    ])
    hdr = f"8=FIX.4.4{SOH}9={len(body)}{SOH}"
    raw = hdr + body
    return raw + f"10={checksum(raw)}{SOH}"

def cancel_req(seq, sender, target, ts, cl_ord_id, orig_cl_ord_id, symbol, side, qty):
    body = soh_join([
        f"35=F", f"34={seq}", f"49={sender}", f"52={ts}", f"56={target}",
        f"11={cl_ord_id}", f"41={orig_cl_ord_id}", f"54={side}",
        f"55={symbol}", f"38={qty}",
    ])
    hdr = f"8=FIX.4.4{SOH}9={len(body)}{SOH}"
    raw = hdr + body
    return raw + f"10={checksum(raw)}{SOH}"

def main():
    t0 = time.time()
    rng = random.Random(42)  # deterministic

    # Base timestamp: 2024-01-02 09:30:00, increments by ~50ms per message
    base_ms  = int(time.mktime(time.strptime("20240102-09:30:00", "%Y%m%d-%H:%M:%S"))) * 1000

    sender = SENDERS[0]
    target = TARGETS[0]
    seq    = 1
    ord_counter = 1_352_000_000

    lines = []
    FLUSH = 50_000  # write in chunks

    def ts(offset_ms):
        ms = base_ms + offset_ms
        t  = time.gmtime(ms // 1000)
        return time.strftime("%Y%m%d-%H:%M:%S", t) + f".{ms % 1000:03d}"

    with open(OUT, "w", buffering=1 << 20, encoding="latin-1") as f:
        # Logon pair
        f.write(logon(seq, sender, target, ts(0))); seq += 1
        f.write(logon(seq, target, sender, ts(50))); seq += 1

        msg_offset = 100
        count = 2

        while count < TARGET:
            roll = rng.random()

            if roll < 0.05:
                # Heartbeat
                f.write(heartbeat(seq, sender, target, ts(msg_offset)))
                seq += 1; count += 1; msg_offset += 30_000

            elif roll < 0.75:
                # NewOrder → ER:New → ER:Fill cycle (3 messages)
                cl_ord_id  = ord_counter; ord_counter += 1
                ord_id     = 9_000_000 + cl_ord_id
                exec_id    = ord_id + 1
                sym        = rng.choice(SYMBOLS)
                side       = rng.choice(SIDES)
                qty        = rng.randint(1, 50) * 100
                price      = round(rng.uniform(10.0, 500.0), 2)

                f.write(new_order(seq, sender, target, ts(msg_offset),
                                  cl_ord_id, sym, side, qty, price))
                seq += 1; count += 1; msg_offset += rng.randint(1, 5)

                # ER New
                f.write(exec_report(seq, target, sender, ts(msg_offset),
                                    cl_ord_id, ord_id, exec_id,
                                    sym, side, qty, price,
                                    "0", "0", qty, 0, price))
                seq += 1; count += 1; msg_offset += rng.randint(1, 3)

                # ER Fill
                exec_id += 1
                f.write(exec_report(seq, target, sender, ts(msg_offset),
                                    cl_ord_id, ord_id, exec_id,
                                    sym, side, qty, price,
                                    "F", "2", 0, qty, price))
                seq += 1; count += 1; msg_offset += rng.randint(20, 200)

            else:
                # NewOrder → ER:New → Cancel → ER:Canceled (4 messages)
                cl_ord_id  = ord_counter; ord_counter += 1
                cxl_id     = ord_counter; ord_counter += 1
                ord_id     = 9_000_000 + cl_ord_id
                exec_id    = ord_id + 1
                sym        = rng.choice(SYMBOLS)
                side       = rng.choice(SIDES)
                qty        = rng.randint(1, 50) * 100
                price      = round(rng.uniform(10.0, 500.0), 2)

                f.write(new_order(seq, sender, target, ts(msg_offset),
                                  cl_ord_id, sym, side, qty, price))
                seq += 1; count += 1; msg_offset += rng.randint(1, 5)

                f.write(exec_report(seq, target, sender, ts(msg_offset),
                                    cl_ord_id, ord_id, exec_id,
                                    sym, side, qty, price,
                                    "0", "0", qty, 0, price))
                seq += 1; count += 1; msg_offset += rng.randint(1, 3)

                f.write(cancel_req(seq, sender, target, ts(msg_offset),
                                   cxl_id, cl_ord_id, sym, side, qty))
                seq += 1; count += 1; msg_offset += rng.randint(1, 5)

                exec_id += 1
                f.write(exec_report(seq, target, sender, ts(msg_offset),
                                    cxl_id, ord_id, exec_id,
                                    sym, side, qty, price,
                                    "4", "4", 0, 0, price))
                seq += 1; count += 1; msg_offset += rng.randint(20, 200)

            if count % 100_000 == 0:
                elapsed = time.time() - t0
                print(f"  {count:>9,} messages  {elapsed:.1f}s", flush=True)

    elapsed = time.time() - t0
    import os
    size_mb = os.path.getsize(OUT) / 1_048_576
    print(f"\nDone: {count:,} messages  {size_mb:.1f} MB  {elapsed:.1f}s  →  {OUT}")

if __name__ == "__main__":
    main()
