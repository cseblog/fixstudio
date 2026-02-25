# FIX Parser Optimization

Summary of optimizations applied to the parser.

## Implemented Optimizations

### 1. Lazy `value_description` — highest impact

**Problem:** `value_description()` was called for every tag of every message during parse (e.g. ~30 tags × 1000 messages = 30,000 calls). Each does nested matches and allocates a `String`.

**Solution:**
- Removed `value_description` from `FixField`; only `tag`, `value`, and `tag_description` are stored.
- Parser no longer calls `value_description`.
- Detail panel computes `value_description(&field.tag, &field.value)` only when displaying a selected message’s fields.

**Files:** `src/model.rs`, `src/parser.rs`, `src/components/detail.rs`

---

### 2. Pre-allocation

**Problem:** `Vec` grew via repeated `push()` with reallocations.

**Solution:**
- **Messages:** `Vec::with_capacity(msg_count.max(1))` using `normalized.matches("8=FIX").count()`.
- **Fields:** `Vec::with_capacity(raw.matches('|').count() + 1)` per message.

**File:** `src/parser.rs`

---

### 3. Single-pass normalization

**Problem:** Three chained `.replace()` calls created three intermediate `String`s:

```rust
input.replace('\u{01}', "|").replace("\\x01", "|").replace("^A", "|")
```

**Solution:** New `normalize_delimiters()` does a single pass over the input, handling SOH (`\x01`), `\x01`, and `^A` without extra allocations.

**File:** `src/parser.rs`

---

### 4. Parallel parsing with Rayon

**Problem:** Messages were parsed sequentially.

**Solution:** Message parsing uses `raw_msgs.par_iter().map(parse_single).collect()` so each message is parsed in parallel on multiple cores.

**Dependency:** `rayon = "1.10"` in `Cargo.toml`

**File:** `src/parser.rs`

---

### 5. Message assembly tweaks

**Problem:** `current.clone()` and `format!()` caused unnecessary allocations when assembling messages.

**Solution:**
- Use `std::mem::take(&mut current)` instead of `current.clone()` when pushing completed messages.
- Use `current.reserve(6 + segment.len())` before `push_str` to reduce reallocations.

**File:** `src/parser.rs`

---

## Tests

Two parser tests verify correctness:

- `test_parse_sample` — basic FIX message parsing
- `test_normalize_soh` — SOH delimiter normalization

Run with: `cargo test`
