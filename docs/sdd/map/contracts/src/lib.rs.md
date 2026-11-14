# contracts/src/lib.rs

Stable ID: `contract.library` · Phase: `R1` · Kind: contract
Implementation wave: `I0-A`.

Owns the pure Rust contract package exports `config` and `operations`. No state,
I/O, parser, host authority or dependencies beyond data serialization. Consumers:
Core and trusted adapter implementations; runtime does not depend on this package.
Methods: none. Errors and versioned records are defined in `operations` and
configuration/composition records in `config`. This binding implements INTERFACES
sections 2–5 and 11–12; product tests remain deferred until I4 completes.
