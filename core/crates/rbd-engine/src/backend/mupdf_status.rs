//! MuPDF compilation status documentation and diagnosis.
//!
//! # MuPDF on Windows MSVC Evaluation
//!
//! During Phase 0 Spike 1 evaluation, building `mupdf` / `mupdf-sys 0.8.0` on Windows MSVC
//! (using MSVC toolset 14.50 / Visual Studio 2026 build tools) was tested.
//!
//! ## Root Cause of Failure
//! `mupdf-sys`'s `build.rs` invokes MSBuild on the bundled `platform/win32/mupdf.sln`.
//! That solution file hardcodes the Visual Studio 2019 platform toolset (`Platform Toolset = 'v142'`).
//!
//! When invoked on modern MSVC installations without Visual Studio 2019 installed, MSBuild fails with:
//! ```text
//! error MSB8020: The build tools for Visual Studio 2019 (Platform Toolset = 'v142') cannot be found.
//! To build using the v142 build tools, please install Visual Studio 2019 build tools.
//! Alternatively, you may upgrade to the current Visual Studio tools by selecting the Project menu
//! or right-click the solution, and then selecting "Retarget solution".
//! ```
//!
//! Because `mupdf-sys` has not updated its MSVC build scripts for newer Visual Studio toolchains,
//! RBD OFİS provides:
//! 1. A production-grade C/C++ backend using Google PDFium (`pdfium-render` + `bblanchon/pdfium-binaries`),
//!    which works out-of-the-box on Windows MSVC.
//! 2. A 100% pure-Rust zero-C-dependency reference backend using `lopdf` and `hayro`.

/// Returns the diagnostic reason why `mupdf-sys` fails on Windows MSVC without VS 2019 tools.
pub fn mupdf_msvc_failure_reason() -> &'static str {
    "mupdf-sys 0.8.0 hardcodes MSBuild Platform Toolset 'v142' (VS 2019). On MSVC 14.50+ / VS 2026, MSBuild fails with MSB8020."
}
