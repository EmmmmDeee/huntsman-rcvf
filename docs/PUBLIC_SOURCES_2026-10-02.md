# Public sources — 2026-10-02

Executed from this host. One call each. No fan-out.

| Source | Result | Use |
| --- | --- | --- |
| Wikidata wbsearchentities Brisbane | 200, Q34932, capital of Queensland | Fixture. Later lookup returned 429 and was refused |
| Wikidata Q34932 P625 | 200, -27.46778, 153.02778 | Independent of Nominatim. Separation under 2 km |
| Nominatim Brisbane | 200, relation 14027280, -27.46896, 153.02350 | Live lookup reproduced |
| ABS dataflow | 200 | Not a case input. Not wired |
| SeekNow | NOT APPLICABLE | Paid breach source. No client |
| Public SearXNG JSON | CLOSED | 403/429 on earlier single attempts. Not retried |

A 429 is not a hit. A challenge page is not a hit. These lookups do not enter Navigator.
