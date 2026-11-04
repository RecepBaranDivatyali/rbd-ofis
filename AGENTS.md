# AGENTS.md — RBD OFİS Ajan Anayasası

Bu dosya, bu depoda çalışan **her yapay zeka ajanı** için bağlayıcıdır. Görevine başlamadan önce baştan sona oku.

## 1. Proje
- **RBD OFİS**: Açık kaynak (AGPL-3.0-or-later), ücretsiz, reklamsız, offline ofis + PDF paketi.
- Platform sırası: **Windows → Android** → (ileride) macOS/iOS → Linux.
- Yol haritası: `docs/YOL_HARITASI.md`. Kararlar: `docs/adr/`.

## 2. Mimari kuralları (değişmez)
1. **Mantık Rust çekirdekte (`core/`) durur.** Platform arayüzleri (`apps/*`) "incedir": sadece çizer ve olayları iletir. İş mantığı arayüze yazılmaz.
2. Çekirdek PDF motorunu doğrudan çağırmaz; **`rbd-engine` soyutlama katmanı** üzerinden çağırır (MuPDF ↔ PDFium değiştirilebilir olmalı).
3. FFI sınırından büyük piksel tamponları kopyalanmaz; paylaşımlı tampon / tutamaç (handle) kullanılır.
4. Çekirdek panik ile çökmez: tüm genel API'ler `Result` döner, FFI sınırında `catch_unwind` kullanılır.
5. Ağ erişimi yoktur. Çekirdek ve uygulamalar **hiçbir veriyi dışarı göndermez** (telemetri yasak).

## 3. Lisans kuralları (yayın güvenliği — ihlal = görev reddi)
`docs/adr/0003-yayin-uyumlulugu.md` dosyası esastır. Özet:
- Yeni bağımlılık eklemeden önce lisansını kontrol et. **İzinli:** MIT, BSD, Apache-2.0, ISC, Zlib, MPL-2.0, LGPL, GPL-3.0, AGPL-3.0, OFL (yazı tipleri).
- **Yasak:** Kapalı kaynak SDK'lar (Google ML Kit, Firebase, Play Services kütüphaneleri, reklam/analitik SDK'ları), GPL-2.0-only, ticari kısıtlı lisanslar (CC-BY-NC, OpenRAIL-M ticari kısıtlı modeller), Microsoft/Adobe yazı tipleri.
- Her yeni bağımlılık `THIRD_PARTY.md`'ye eklenir. Rust tarafında `cargo deny check licenses` geçmek zorundadır.
- Yapay zeka model ağırlıkları için lisans ayrıca doğrulanır ve ADR'ye yazılır.

## 4. İş akışı
1. Müdür görevi **yazılı kabul kriterleriyle** verir.
2. Mühendis ajan uygular, testleri yazar ve yerelde çalıştırır.
3. Test ajanı bağımsız doğrular (testler, korpus, performans).
4. Mimar-Eleştirmen inceler. **Kodu yazan kendi kodunu onaylayamaz.**
5. Müdür son kontrolü yapar ve birleştirir.

## 5. Kod kalitesi
- Rust: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` temiz olmalı.
- Testi olmayan özellik "bitmiş" sayılmaz. Hata düzeltmesine regresyon testi eklenir.
- Performans gerilemesi hata sayılır (benchmark'lar `core/benches`).
- Kod içi yorumlar ve tanımlayıcılar **İngilizce**. Kullanıcıya görünen metinler çeviri dosyalarından gelir (TR + EN zorunlu).
- Dosya/klasör/kimlik adlarında Türkçe karakter (İ, ı, ş, ğ, ç, ö, ü) **kullanılmaz** (`rbd-ofis`, `rbdofis`).

## 6. Rapor formatı (her görev sonunda müdüre)
- Ne yapıldı (dosya listesi)
- Kabul kriterleri: her biri ✅/❌ ve kanıtı (komut çıktısı)
- Bilinen sorunlar / riskler
- Eklenen bağımlılıklar ve lisansları
