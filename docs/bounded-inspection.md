# Bounded legacy inspection

The legacy public functions remain available with their existing signatures.
`load(String)` reads to EOF without an input cap. `decoded_print` and
`icc_profile_print` have no output cap and may allocate large intermediate
strings through `Data::as_string`. The latter also runs the legacy payload
decoder. Malformed caller-created payload structures may panic in the legacy
formatter. These APIs are compatibility paths, not bounded inspection APIs.

## Input bytes

```rust
use icc_profile::utils::{load_with_limit, read_with_limit};

let profile = load_with_limit("display.icc".to_owned(), 8 * 1024 * 1024)?;
let bytes = read_with_limit(reader, 8 * 1024 * 1024)?;
```

`read_with_limit` accepts any `Read` implementation, including a file, cursor,
pipe, or growing stream. It does not trust metadata. It grows its buffer
fallibly, never requesting a buffer capacity above the byte budget, and probes
at most one extra byte after reaching the limit. Exact EOF succeeds. Oversize
returns `std::io::ErrorKind::InvalidData`; it never returns a truncated success.
Zero accepts an empty stream only. `Interrupted` is retried and other I/O errors
are preserved. Buffer allocation failure returns `OutOfMemory`.

`load_with_limit` uses the same reader and then the existing `ICCProfile::new`
header constructor. It does not validate tag payloads. That constructor copies
the bounded input and makes its existing allocations: the file-size budget is
not a total-memory limit or an allocation-failure guarantee for the constructor.
A blocking `Read` can still block; the limit is not a timeout.

## Output bytes

```rust
use icc_profile::utils::decoded_print_with_limit;

let text = decoded_print_with_limit(&decoded, 64 * 1024)?;
```

`decoded_print_with_limit` takes an already-created `DecodedICCProfile` and
returns the full diagnostic report or `InvalidData` when it cannot fit in the
UTF-8 byte budget. It does not return partial/truncated text. The exact output
length succeeds, one byte less fails, and zero always fails. Bytes are counted
after escaping, not by Unicode scalar or character count.

This is a separate diagnostic format: numeric header identifiers and payloads
use derived Rust `Debug`, with all payload fields included, rather than the
legacy human-oriented `Data::as_string(verbose)` format. Debug text is not a
stable serialization format. No verbosity argument is needed. The report
prints public fields as they exist, without validating their semantic meaning
or indexing arrays according to caller-supplied counts.

Before rendering or sorting, a nonallocating preflight charges a lower bound
for every variable-size payload and key: string byte lengths, vector entry
counts, and nested contents. Outer lengths are charged before visiting nested
values. Oversized strings are rejected before Debug can scan their escapes;
large vectors are rejected before formatting. This lower bound cannot reject
an otherwise fitting report. Streaming `fmt::Write` then enforces the actual
byte limit before every append and propagates the first formatting failure.
No complete payload rendering is allocated first. The private mAB/mBA CLUT
grid vector has a fixed 16-entry parser bound; all publicly variable nested
vectors and strings are covered by the preflight.

Output and sorted-tag index allocations use `try_reserve_exact` and return
`OutOfMemory` on allocation failure. Requested output capacity never exceeds
the output budget. The borrowed index uses one pair of references per tag;
its entry count is admitted against the remaining report budget, and no tag
key or payload is cloned. Index storage is additional to the output buffer.
These are not caps on memory already held by the decoded input, its HashMap
capacity, or an elapsed-time guarantee.

Both printers sort tags lexicographically. Both label `chad` as
"Chromatic adaptation matrix", because a source white point of D65 cannot be
assumed for every profile.

## Parsing is a separate boundary

An output budget does not limit input reads, parsing allocations, payload
validation, or duplicate/overlapping tag decoding. A bounded file read does
not turn the legacy decoder into a fully resource-limited parser. Use the
checked `Profile` / `ParseLimits` API when that is the required contract. The
separate legacy parser hardening changes remain necessary for legacy raw-tag
inspection. This change does not add a bounded raw-profile printing wrapper
that would suggest otherwise.

## Regression coverage

The focused integration suite exercises exact, zero, one-byte-over, short,
chunk-boundary and growing input; a real bounded file load; interrupted/error
reads; malformed `Read` counts; exact multibyte/escaped output; insertion-order
independence; neutral `chad` labeling; huge strings, vectors, nested vectors,
keys and dates; and malformed public LUT/count values. Unit tests use the
existing allocation probe to verify no huge intermediate rendering, injected
allocation failures, and immediate abort of streaming Debug after writer error.
