# RBD OFİS

**Ücretsiz. Reklamsız. Çevrimdışı (Offline). Açık Kaynak.**
PDF görüntüleme, düzenleme, sayfa organizasyonu, açıklama ekleme, dönüştürme ve hafif ofis dökümanı desteği sağlayan masaüstü ve mobil uygulama paketi.

> ℹ️ Durum: **Faz 0 – Faz 8 tamamlandı.** (v1.0.0 sürümüne ulaşıldı; bilinen sınırlamalar ve planlanan geliştirmeler aşağıda listelenmiştir).

## Platformlar ve Teknoloji

| Platform | Durum | Kullanıcı Arayüzü / Teknoloji |
|---|---|---|
| Windows 10/11 | Tamamlandı | **WPF (.NET 9, C#)** (WinUI 3 yerine WPF tercih edilmiştir) |
| Android | Tamamlandı | **Kotlin + Jetpack Compose** |
| macOS / iOS / iPadOS | Tamamlandı | **SwiftUI + Metal / PencilKit** |
| Linux | Tamamlandı | **Rust CLI (`rbd-cli`) & XDG Entegrasyonu** |

## Mimari

Uygulamanın iş mantığı platformlar arası ortak **Rust çekirdeğinde (`core/`)** çalışır. Arayüzler incedir; yalnızca çizim ve kullanıcı etkileşimini yönetir:
- **PDF Motoru (ADR-0002):** Birincil motor olarak **PDFium** (Chromium 8125, BSD-3-Clause) ve saf Rust yedek motor olarak **Hayro + lopdf** (Apache-2.0 / MIT) soyutlama katmanı (`rbd-engine`) arkasında çalışır. MuPDF veya qpdf doğrudan kullanılmamaktadır.
- **Ofis Desteği (ADR-0004):** Gömülü ONLYOFFICE veya WPS seviyesinde bir ofis paketi değildir. Platformun yerel WebView bileşeni (Windows WebView2, Android WebView, Apple WKWebView) üzerinde çalışan, harici ağa kapalı hafif HTML `contenteditable` editörü ile çekirdekte saf Rust OOXML (`zip` + XML ayrıştırma) metin çıkarma ve DOCX oluşturma mimarisi kullanılır.
- **Yapay Zeka / Metin Analitiği (Faz 8):** Ağır LLM modelleri (llama.cpp, RAG vb.) yerine, tamamen yerel ve çevrimdışı çalışan istatistiksel doğal dil işleme (TF-ISF cümle skorlama ve anahtar sözcük tabanlı arama) motoru kullanılır.

## Bilinen Sınırlamalar ve Yapılacaklar

1. **OCR Henüz Uygulanmadı:** Belge tarama ve PDF görüntüleme mevcuttur; optik karakter tanıma (OCR) özelliği henüz uygulanmamış olup gelecek sürümlerde planlanmaktadır.
2. **Ofis Düzenleme Kapsamı:** Ofis düzenleyici WPS Office veya Microsoft Office düzeyinde tam teşekküllü bir kelime işlemci değildir; hafif HTML tabanlı zengin metin düzenleme, saf Rust OOXML metin çıkarma ve temel DOCX dosyası üretimi ile sınırlıdır.
3. **Yapay Zeka Yetenekleri:** Yapay zeka özellikleri üretken LLM (Large Language Model) veya sinir ağı tabanlı RAG içermez; yerel istatistiksel (TF-ISF) çıkarımsal özetleme ve anahtar sözcük tabanlı soru-cevap ile sınırlıdır.
4. **PDF Reflow Türkçe Karakter Kısıtı:** PDF sayfa metin akışı düzenleme (reflow) ve paragraf değiştirme işlemlerinde Type 1 / WinAnsi karakter kodlaması kısıtları nedeniyle Türkçe karakterler (`ı, ş, ğ, İ, Ş, Ğ`) ASCII karşılıklarına (`i, s, g, I, S, G`) dönüştürülmektedir.
5. **core/benches Eksikliği:** Rust çekirdeğinde bağımsız `core/benches` performans kıyaslama test paketi henüz oluşturulmamıştır; performans regresyon testleri için bu paketin kurulması yapılacaklar arasındadır.

## Gizlilik ve Güvenlik

RBD OFİS hiçbir veri toplamaz, telemetri barındırmaz ve dosyalarınızı hiçbir yere göndermez. Uygulama sıfır ağ erişimi ilkesiyle tamamen yerel çalışır (Android sürümünde `INTERNET` izni yoktur).

## Lisans

[GNU AGPL-3.0-or-later](LICENSE) © Recep Baran Divatyalı
