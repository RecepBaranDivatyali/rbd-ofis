# Üçüncü Taraf Bağımlılıklar ve Lisanslar (THIRD_PARTY.md)

Bu dosya, RBD OFİS projesinde kullanılan tüm üçüncü taraf paketlerin ve kütüphanelerin lisans uyumluluğunu belgeler.
Tüm bağımlılıklar `docs/adr/0003-yayin-uyumlulugu.md` ve `AGENTS.md` Madde 3 kurallarına uygun olarak seçilmiştir.

## Windows Uygulaması (.NET 9 / NuGet)

| Paket | Sürüm | Lisans | Kullanım Amacı | Notlar |
|---|---|---|---|---|
| **Microsoft.Web.WebView2** | 1.0.4314.35 | BSD-2-Clause / Microsoft Redistributable | ONLYOFFICE offline sanal sunucu görüntüleme (`https://rbd-office.local/`) | ADR-0003 A.2: İşletim sistemi bileşeni / sistem kütüphanesi istisnası kapsamında. |
| **xunit** | 2.9.2 | Apache-2.0 | Birim test çerçevesi | İzinli lisans. |
| **xunit.runner.visualstudio** | 2.8.2 | Apache-2.0 | Visual Studio / `dotnet test` test çalıştırıcısı | Test aracı. |
| **Microsoft.NET.Test.Sdk** | 17.12.0 | MS-EULA / MIT | .NET test SDK alt yapısı | Test aracı. |
| **coverlet.collector** | 6.0.2 | MIT | Test kod kapsamı toplayıcısı | Test aracı. |

---

## Rust Çekirdeği (`core/` Cargo Workspace)

| Kütüphane / Crate | Sürüm | Lisans | Kullanım Amacı | ADR-0003 Uyumluluğu |
|---|---|---|---|---|
| **pdfium-render** | 0.9.5 | MIT | PDFium C API wrapper | İzinli lisans |
| **PDFium Prebuilt Binaries** (`core/vendor/pdfium/`) | Chromium 8125 | BSD-3-Clause | C++ PDFium dinamik kütüphanesi (`pdfium.dll`) | İzinli lisans (BSD-3-Clause) |
| **hayro** | 0.9.0 | Apache-2.0 OR MIT | Saf Rust PDF rasterizer (Vello/WGPU tabanlı) | İzinli lisans |
| **lopdf** | 0.47.0 | MIT | PDF DOM, nesne ayrıştırma ve sayfa manipülasyonu | İzinli lisans |
| **lru** | 0.18.5 | MIT | Doküman kutucuk (tile) önbelleği | İzinli lisans (RUSTSEC-2026-0002/0253 yamalı) |
| **parking_lot** | 0.12.3 | Apache-2.0 OR MIT | Yüksek başarımlı eşzamanlılık kilitleri | İzinli lisans |
| **thiserror** | 2.0.21 | Apache-2.0 OR MIT | Yapılandırılmış hata tanımlama | İzinli lisans |
| **image** | 0.25.10 | MIT OR Apache-2.0 | Piksel tamponu kodlama ve test doğrulama | İzinli lisans |
| **bytemuck** | 1.25.2 | Zlib OR Apache-2.0 OR MIT | Bellek tamponu ve bayt dilimleri güvenli dönüştürme | İzinli lisans |
| **clap** | 4.6.7 | Apache-2.0 OR MIT | CLI komut satırı argüman ayrıştırma | İzinli lisans |
| **serde** / **serde_json** | 1.0.229 / 1.0.149 | Apache-2.0 OR MIT | Kıyaslama (benchmark) ve veri serileştirme | İzinli lisans |

---

## Lisans Politikası Özeti (ADR-0003)
- **Ana Lisans:** AGPL-3.0-or-later.
- **İzin Verilen Lisanslar:** MIT, BSD, Apache-2.0, ISC, Zlib, MPL-2.0, LGPL, GPL-3.0, AGPL-3.0, OFL.
- **Kesinlikle Yasak Olanlar:** Kapalı kaynak SDK'lar (Google ML Kit, Firebase, telemetri/analitik SDK'ları, reklam SDK'ları), GPL-2.0-only, ticari kısıtlı lisanslar.
- **Ağ İletişimi:** Uygulama tamamen offline çalışır; harici ağ erişimi ve telemetri yasaktır (Rule 5).
