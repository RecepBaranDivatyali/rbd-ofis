# ADR-0002: PDF Motoru — `rbd-engine` Soyutlama Katmanı (PDFium + Hayro/lopdf + MuPDF)

- **Durum:** Kabul edildi ve Faz 0 Deney 1 ile doğrulandı
- **Tarih:** 2026-11-04

## Karar
- **Motor Soyutlama Katmanı (`rbd-engine`):** Çekirdek (`rbd-core`) ve FFI (`rbd-ffi`), PDF motorlarına yalnızca `rbd-engine` trait'leri (`PdfEngine`, `PdfDocument`, `PdfPage`) ve `rbd_engine::page_ops` üzerinden erişir.
- **Birincil C++ Motoru — PDFium (BSD-3-Clause, Chromium 8125):**
  - Windows, macOS, Linux, Android ve iOS için resmi/önceden derlenmiş ikilileri (`bblanchon/pdfium-binaries`) mevcuttur.
  - Apple App Store ve Google Play'de lisans çatışması riski sıfırdır.
  - Faz 0 hız ölçümlerinde 2000 sayfalık PDF'te **8.03 ms** soğuk açılış + ilk sayfa çizimi ve **0.214 ms** 256×256 karo çizimi; 1.192 sayfalık gerçek ABD Federal Mevzuat (CFR Title 12) PDF'inde **27.58 ms** açılış + ilk sayfa ve **1.312 ms** ortalama karo çizimi sağlamıştır (Faz 0 kapısı: `< 300 ms` ve `< 16.6 ms`).
- **Yedek Saf-Rust Motoru — Hayro + lopdf (Apache-2.0 / MIT):**
  - C/C++ bağımlılığı olmadan her platformda derlenen referans/yedek motor ve yapısal sayfa işlemleri (`merge`, `split`, `rotate`, `delete_pages`) motoru olarak `rbd-engine` içinde yer alır.
  - Hem PDFium hem Hayro motorları `PixelFormat::Bgra32` ve `PixelFormat::Rgba32` ile satır adımı (`stride`) sözleşmesini birebir aynı kanal dizilimiyle üretir (`test_pdfium_and_hayro_pixel_format_channel_consistency`).
- **MuPDF (AGPL-3.0) Durumu:**
  - `mupdf-sys` crate'i Windows üzerinde sabit kodlanmış Visual Studio 2019 (`v142`) MSBuild araç takımını aradığı için MSVC 14.50 (VS 2026) ortamında doğrudan `cargo build` ile derlenememektedir (`MSB8020`). İleriki fazlarda PAdES imza / gelişmiş redaksiyon için gerektiğinde CMake veya önceden derlenmiş kütüphane olarak `rbd-engine` arkasına bağlanabilir.

## Kısıt
Çekirdek (`rbd-core`), hiçbir PDF motoru kütüphanesine (`pdfium-render`, `hayro`, `lopdf`, `mupdf`) doğrudan bağımlı olamaz; tüm erişim `rbd-engine` üzerinden yapılır.
