## Summary
- Align HWP3-origin sample16 page 3 vertical layout with Hancom 3mm grid references.
- Fix 2022 BCP tail-line handling and k-water 2024 RowBreak table cut position.
- Normalize legacy Latin font substitution so HCI Poppy resolves consistently across HWP3/HWP5 variants.
- Add final report and document that internal task PR creation requires separate approval.

## Verification
- cargo test --test issue_1116 -- --nocapture
- cargo test --test issue_1105 -- --nocapture
- cargo test --test issue_1086 -- --nocapture
- cargo test --test issue_1035_alignment -- --nocapture
- cargo test --test issue_713 -- --nocapture
- cargo fmt --all -- --check
- cargo build --bin rhwp
- git diff --check
- Regenerated 3mm SVGs for the sample16 variants and confirmed Latin glyphs now resolve as Palatino=6, HCI=0.

Related to #1116
