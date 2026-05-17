# Changelog

## [0.1.1](https://github.com/cntm-labs/sentinel/compare/sntl-v0.1.0...sntl-v0.1.1) (2026-05-17)


### Features

* add .Typed() for query_typed() skip-prepare optimization ([44518a6](https://github.com/cntm-labs/sentinel/commit/44518a6110bef988de7cee7b7cbd7be8cd53d6a9))
* add COPY protocol support with copy_in_sql helper ([1135524](https://github.com/cntm-labs/sentinel/commit/1135524c5332c11c3693869e0bcc46b8437f087f))
* add CursorQuery for Portal-based incremental fetching ([753e09a](https://github.com/cntm-labs/sentinel/commit/753e09a5042213d344f2f3c898c357fad02c87d6))
* add sntl umbrella crate and README.md ([5359669](https://github.com/cntm-labs/sentinel/commit/53596699fdb32b865ec4768e2d00b93da6abaf82))
* add Value variants for v1.0.0 types (MacAddr8, TimeTz, LTree, LQuery, Cube, multiranges) ([28eee78](https://github.com/cntm-labs/sentinel/commit/28eee78eadc311a46da2bc5613859c9cff407b38))
* leverage sentinel-driver v0.1.1 features ([#5](https://github.com/cntm-labs/sentinel/issues/5)) ([e2c0ce0](https://github.com/cntm-labs/sentinel/commit/e2c0ce052e12e2fdd7db01954d38a146a7ceff3a))
* Phase 3 driver integration + integration test infrastructure ([#4](https://github.com/cntm-labs/sentinel/issues/4)) ([f762048](https://github.com/cntm-labs/sentinel/commit/f76204894f603b6644b626b264e601af8bdf68d8))
* Phase 4 — relation types, PascalCase API, batch loading ([#6](https://github.com/cntm-labs/sentinel/issues/6)) ([63143cf](https://github.com/cntm-labs/sentinel/commit/63143cf989d11324776bef5b5569cd254a2ed1a5))
* Phase 5A — full PostgreSQL type coverage ([#8](https://github.com/cntm-labs/sentinel/issues/8)) ([a734961](https://github.com/cntm-labs/sentinel/commit/a734961bc761b3c9d157995a11fb7dc82016382a))
* Phase 5B-1 — type-state pattern for compile-time relation safety ([#9](https://github.com/cntm-labs/sentinel/issues/9)) ([b994fe0](https://github.com/cntm-labs/sentinel/commit/b994fe000bcda6d9a531ec4f7bee81db35df2e95))
* re-export sentinel-driver v1.0.0 types ([b6e8b54](https://github.com/cntm-labs/sentinel/commit/b6e8b54445a3651b64b0d33b84ba7f3178ca3205))
* sntl::query!() compile-time SQL validation macro family ([#12](https://github.com/cntm-labs/sentinel/issues/12)) ([958bc18](https://github.com/cntm-labs/sentinel/commit/958bc181ba61efb7bd346e544cb1ae4c974092b2))
* **sntl:** cluster A — array element nullability + tuple FromRow + extension type re-exports ([#13](https://github.com/cntm-labs/sentinel/issues/13)) ([f80748b](https://github.com/cntm-labs/sentinel/commit/f80748b1f7a508e93e558eac8f78497e9ba54698))
* **sntl:** observability bridge — SntlTracing + macro/reducer/migrate events (v0.4) ([#22](https://github.com/cntm-labs/sentinel/issues/22)) ([bedf09e](https://github.com/cntm-labs/sentinel/commit/bedf09e80546dbf07dbb52213e20897540aa1f51))
* use GenericClient trait for all query execution methods ([ed769fb](https://github.com/cntm-labs/sentinel/commit/ed769fb376ec2c65b5f12cad53a32feb62a72cc0))


### Bug Fixes

* **ci:** resolve CI failures — deny advisory ignore, coverage tests ([871fc0d](https://github.com/cntm-labs/sentinel/commit/871fc0dcbc07fc9cf3c1da2270d0f33357fd66d5))
* **ci:** update workflows and deps for crates.io publishing ([68d0f8e](https://github.com/cntm-labs/sentinel/commit/68d0f8eaccbd1d45a9beb04d3cef78c891274532))
* **ci:** use explicit versions in Cargo.toml for release-please ([f1d7306](https://github.com/cntm-labs/sentinel/commit/f1d7306d858bddab9a1c94fc696b6db231d493a8))

## [0.1.1](https://github.com/cntm-labs/sentinel/compare/sntl-v0.1.0...sntl-v0.1.1) (2026-04-06)


### Features

* add sntl umbrella crate and README.md ([5359669](https://github.com/cntm-labs/sentinel/commit/53596699fdb32b865ec4768e2d00b93da6abaf82))


### Bug Fixes

* **ci:** use explicit versions in Cargo.toml for release-please ([f1d7306](https://github.com/cntm-labs/sentinel/commit/f1d7306d858bddab9a1c94fc696b6db231d493a8))
