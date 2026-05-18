# Changelog

## [0.1.2](https://github.com/cntm-labs/sentinel/compare/sntl-macros-v0.1.1...sntl-macros-v0.1.2) (2026-05-18)


### Features

* **sntl:** #[sntl::test] fixture-isolated test harness (v0.5 Phase 3) ([#27](https://github.com/cntm-labs/sentinel/issues/27)) ([57404ff](https://github.com/cntm-labs/sentinel/commit/57404ff69fee7941f683cb912e52b0686740803b))
* **sntl:** fetch_stream() at the macro layer (v0.5 Phase 2) ([#26](https://github.com/cntm-labs/sentinel/issues/26)) ([e6cb6e0](https://github.com/cntm-labs/sentinel/commit/e6cb6e061104036aa26ee0cf0346452495145234))

## [0.1.1](https://github.com/cntm-labs/sentinel/compare/sntl-macros-v0.1.0...sntl-macros-v0.1.1) (2026-05-17)


### Features

* Phase 3 driver integration + integration test infrastructure ([#4](https://github.com/cntm-labs/sentinel/issues/4)) ([f762048](https://github.com/cntm-labs/sentinel/commit/f76204894f603b6644b626b264e601af8bdf68d8))
* Phase 4 — relation types, PascalCase API, batch loading ([#6](https://github.com/cntm-labs/sentinel/issues/6)) ([63143cf](https://github.com/cntm-labs/sentinel/commit/63143cf989d11324776bef5b5569cd254a2ed1a5))
* Phase 5B-1 — type-state pattern for compile-time relation safety ([#9](https://github.com/cntm-labs/sentinel/issues/9)) ([b994fe0](https://github.com/cntm-labs/sentinel/commit/b994fe000bcda6d9a531ec4f7bee81db35df2e95))
* **sntl-migrate:** PR-2 — diff + migrate!() macro + CLI subcommand ([#20](https://github.com/cntm-labs/sentinel/issues/20)) ([162a2e1](https://github.com/cntm-labs/sentinel/commit/162a2e18167268f26585c5ab0edd60bb59301ea0))
* sntl::query!() compile-time SQL validation macro family ([#12](https://github.com/cntm-labs/sentinel/issues/12)) ([958bc18](https://github.com/cntm-labs/sentinel/commit/958bc181ba61efb7bd346e544cb1ae4c974092b2))
* **sntl:** cluster A — array element nullability + tuple FromRow + extension type re-exports ([#13](https://github.com/cntm-labs/sentinel/issues/13)) ([f80748b](https://github.com/cntm-labs/sentinel/commit/f80748b1f7a508e93e558eac8f78497e9ba54698))
* **sntl:** observability bridge — SntlTracing + macro/reducer/migrate events (v0.4) ([#22](https://github.com/cntm-labs/sentinel/issues/22)) ([bedf09e](https://github.com/cntm-labs/sentinel/commit/bedf09e80546dbf07dbb52213e20897540aa1f51))
* use GenericClient trait for all query execution methods ([ed769fb](https://github.com/cntm-labs/sentinel/commit/ed769fb376ec2c65b5f12cad53a32feb62a72cc0))


### Bug Fixes

* cargo-deny — add CDLA-Permissive-2.0 license, license fields, relax wildcard ban ([0b486c6](https://github.com/cntm-labs/sentinel/commit/0b486c62a3fdead032eccb06c224d7ae48a8f853))
* **ci:** update workflows and deps for crates.io publishing ([68d0f8e](https://github.com/cntm-labs/sentinel/commit/68d0f8eaccbd1d45a9beb04d3cef78c891274532))
* **ci:** use explicit versions in Cargo.toml for release-please ([f1d7306](https://github.com/cntm-labs/sentinel/commit/f1d7306d858bddab9a1c94fc696b6db231d493a8))

## [0.1.1](https://github.com/cntm-labs/sentinel/compare/sntl-macros-v0.1.0...sntl-macros-v0.1.1) (2026-04-06)


### Bug Fixes

* cargo-deny — add CDLA-Permissive-2.0 license, license fields, relax wildcard ban ([0b486c6](https://github.com/cntm-labs/sentinel/commit/0b486c62a3fdead032eccb06c224d7ae48a8f853))
* **ci:** use explicit versions in Cargo.toml for release-please ([f1d7306](https://github.com/cntm-labs/sentinel/commit/f1d7306d858bddab9a1c94fc696b6db231d493a8))
