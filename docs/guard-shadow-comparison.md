# Read-only shadow comparison

`guard_integration::shadow::compare(raw, projection)` compares the exact original
native bytes with an optional sealed SDK projection. It performs no filesystem,
process, publication, repair, or envelope operation. The native input budget is
1 MiB, checked before hashing. With a projection, exact raw equality is required,
and GuardEngine recomputes the contract and facts before the stored report is
returned. A mismatch is an error. Existing projection constructors enforce the
fact/contract budgets before this API can receive a projection.

With `None`, the result contains only the SHA-256 of the bytes. This deliberately
works for unsupported native formats too: it is a byte summary, not evidence of a
completed run, qualification, or an engine decision. `engine_report()` rejects
this weaker result. Do not treat successful digest computation as verification.

The capability boundary remains the locally registered readers:

| Profile | Capability |
| --- | --- |
| `run_report` 1.0 / descriptor `lint --format=json` | Strict readable legacy profile; unqualified, always partial. `InvocationDescriptor::capability()` declares this limitation. |
| `codeguard.ruff-f401/linux-x86_64/v1alpha1` | Exact selected-file Python feedback 0.13.0, pinned producer/Ruff, frozen source/config/tool/F401 settings and suppression audit. Complete only when the sealed reader proves every obligation. |
| Aggregate `check_feedback` 0.38.0, other schemas/tools/rules/platforms | Unsupported for complete engine projection. Native-only byte summaries do not change that. |

Caller metadata cannot add a capability. Ruff capture provenance is local trusted
executor evidence under its documented host profile, not external producer
authentication. Aggregate native exit 3 remains unchanged; its number is not an
engine approval. The old native commands and output ownership remain intact.
