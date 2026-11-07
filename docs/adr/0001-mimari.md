# ADR-0001: Rust Çekirdek + Yerel Platform Arayüzleri

- **Durum:** Kabul edildi
- **Tarih:** 2026-11-04

## Karar
- Ortak mantık (döküman modeli, görüntüleme, düzenleme, OCR, dönüştürme) **Rust** çekirdekte (`core/`) yazılır.
- Arayüzler: **Windows → WPF (.NET 9, C#)** (WinUI 3 yerine kararlı masaüstü desteği ve düşük ek yük için WPF seçildi), **Android → Kotlin + Jetpack Compose**, ileride **Apple → SwiftUI**, **Linux → GTK4 (gtk-rs)**.
- Köprü: UniFFI (Kotlin/Swift) ve C-ABI (C# için `csbindgen` veya UniFFI C# üreticisi; Faz 0'da seçilecek).

## Elenen seçenekler
- **Tauri / React Native:** Büyük döküman tuvalinde performans ve kalem gecikmesi yetersiz. RN'in Linux desteği yok.
- **Flutter:** PencilKit / Windows Ink erişimi yok, masaüstü olgunluğu sınırlı.
- **Compose Multiplatform:** Masaüstünde JVM paketlemesi gerekiyor (+50 MB, yavaş açılış).

## Gerekçe
Sektör liderleri (PDF Expert, Xodo, Foxit, Drawboard) yerel çekirdek + yerel arayüz modelini kullanıyor. Kodu yapay zeka ajanları yazdığı için çoklu arayüz maliyeti tolere edilebilir.
