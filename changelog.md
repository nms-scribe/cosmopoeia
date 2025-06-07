# Changelog

All notable changes to this project should be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased] - (Someday)

## [0.2.5] - (2025-06-07)

* Upgraded to gdal crate 0.18.0
* Fixed error caused by compiling in rust 1.81.0

## [0.2.4] - (2024-09-01)

* Lowered the water fill cycle limit
* Allow water flow across flat tiles
* Allow override of glacier and wetland biome criteria

## [0.2.3] - (2024-08-31)

* Fixed problem with importing partial-world heightmaps
* Deferred fix to water fill cycle
* Added all commands to gen-water and gen-biome.

## [0.2.2] - (2024-08-31)

* Updated code to compile in rust 1.78.0 without warnings
* Updated code to use gdal crate 0.17.0
* Updated to work with gdal 3.9.0

## [0.2.1] - (2023-12-28)

* Added `--world-shape` argument to terrain generation commands. Default value is `cylinder` which defines traditional behavior.
* Added `sphere` option for `--world-shape` which generates worlds which behave more like they are on a sphere.

## [0.2.0] - 2023-10-18

Initial public release.

<!-- next-url -->
[Unreleased]: https://github.com/nms-scribe/cosmopoeia/v0.2.5...HEAD
[0.2.5]: https://github.com/nms-scribe/cosmopoeia/v0.2.4...v0.2.5
[0.2.4]: https://github.com/nms-scribe/cosmopoeia/v0.2.3...v0.2.4
[0.2.3]: https://github.com/nms-scribe/cosmopoeia/v0.2.2...v0.2.3
[0.2.2]: https://github.com/assert-rs/predicates-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/assert-rs/predicates-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/nms-scribe/cosmopoeia/v0.1.2...v0.2.0
