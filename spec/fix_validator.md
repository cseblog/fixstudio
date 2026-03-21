## Feature 7: FIX Message Validator & Debugger

**Category:** Free (with AI explanation as Pro)

**my phyloshopy developing**
All code changes have to help our software: Robustness, simplest, and clean des  

### What it does

Validates any pasted FIX message against the spec and provides precise, actionable error messages.

### Validations (rule-based)
- use an existing open-source spec (FIX Orchestra / QuickFIX XML dictionaries)
- Required tags present for each MsgType (e.g., `35=D` requires 49, 56, 11, 54, 38, 40, 44, 60)
- Valid enum values per tag and version (e.g., `54` Side: 1/2/5/6 for FIX 4.4)
- Checksum (10) verification with correct value shown
- BodyLength (9) verification
- Sequence of tag groups (e.g., repeating groups must have delimiter tag first)
- FIX version consistency (tags introduced in 4.4 shouldn't appear in a 4.2 message)

you looks at FIX specification populars, using these specification as reference to help us validate FIX messages. 
Because we are building a general tool for eFX in sell sides,  buy sides, and other multi dealer broker platform such as ECN, Thomson Retuer, Flextrade, 306T, Bloomberg FXGO, FXall, BidFX..etc



### Research questions
- Whether to build a full FIX data dictionary in Rust or use an existing open-source spec (FIX Orchestra / QuickFIX XML dictionaries)
- How to handle custom tags (tag >= 5000) — user-defined schema upload?
- Performance: validating 1M messages — should validation run in parallel with parsing?


