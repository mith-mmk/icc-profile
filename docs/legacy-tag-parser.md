# Legacy tag parser hardening

This change hardens `iccprofile::Data` inspection, not the separate modern transform compiler. It preserves public signatures and decoded types. It is not a complete ICC conformance validator.

## Wire-layout corrections

Layout reference: ICC.1:2022, clause 10 tag type definitions (https://www.color.org/specification/ICC.1-2022-05.pdf), and the multi-process-element/curve segment definitions. Synthetic fixtures encode integers and float bit patterns directly, independently of the parser.

- `chrm`: y was read from x's fractional half and y's integer half; x=(1,2), y=(3,4) now retains those exact components instead of y=(2,3).
- `mluc`: one record-size field precedes the entire record array, not each record. Twelve-byte records advance independently. A two-record en-US / ja-JP fixture now yields A / あ instead of repeating the first record. Odd lengths, invalid UTF-16, and strings overlapping headers are rejected.
- `desc`: ASCII starts at byte 12, not 16. The historical high-version Descriptor branch now returns `old\0` instead of the language bytes `en\0\0`; Unicode count is in 16-bit code units. The ordinary legacy ASCII return type and embedded NUL behavior remain unchanged.
- `clro`: the count is followed directly by byte entries, not another u32 count.
- `clrt`: each record is 32 name bytes plus three u16 PCS values. Two fixture entries now yield [1,2,3] and [4,5,6]. The existing public type can retain only the first colorant name; additional names remain unavailable.
- `rcs2`: the offset table begins after both u16 fields; each channel declares its response count. A one-channel, two-response fixture now contains both responses (intervals 7 and 9), not just one response per channel.
- `mAB`/`mBA`: the header is 32 bytes including signature/reserved. B/M curves use output/input channel counts respectively; A curves use input/output counts. Embedded curves advance with four-byte alignment. Optional zero offsets remain absent stages; nonzero invalid offsets never mean absence.
- `samf`: sample count is u32, not the upper u16. `parf` needs a 12-byte header, and its serialized `as_size` includes that header.
- `curf`: one array of segment-minus-one breakpoints precedes actual formula/sampled segments. All segments are validated. The legacy public output exposes one breakpoint vector only, not full decoded segment objects.
- `pseq`: each 20-byte device record is followed by two embedded desc/mluc tag structures, without the old invented tag/length wrapper. Both descriptions advance the cursor.
- `view`, `mft1`, `mft2`, and `ncl2`: minimum headers no longer require bytes belonging to a first record that may be absent.

## Resource and compatibility boundaries

Declared tag length is enforced even when `Data::get` receives a larger backing slice. Count/stride ranges are checked before count-driven allocation. Scalar array payloads must contain whole elements. LUT powers/products are checked before use. There is no production panic-catching fallback.

Nested position records must be in-bounds, four-byte aligned, outside the table, and non-overlapping. Aliased position records are deliberately rejected rather than expanded repeatedly. Recursion is capped at 16 nested calls. Aggregate mluc string bytes and response-curve decoded wire bytes are capped at four times the containing tag length, permitting ordinary sharing while bounding amplification. These are legacy-reader resource policies, not ICC conformance rules; unusually alias-heavy profiles may now be rejected.

Unknown top-level signatures remain `Data::Raw`. Nested MPE currently validates only cvst and matf elements; CLUT/ACS/unknown MPE elements report an explicit unsupported error instead of pretending the processing chain was decoded. Some valid ICC profiles therefore remain unsupported. ASCII-only legacy desc inspection still does not require optional legacy Unicode/ScriptCode tails; embedded pseq desc requires the full structure. These changes do not claim semantic completeness, panic/OOM freedom for arbitrary enormous inputs, or validation of transforms/color science.
