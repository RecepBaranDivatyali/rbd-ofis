# RBD OFİS Test Corpus

Bu dizin, **RBD OFİS** projesinin çevrimdışı döküman motorlarını (MuPDF, PDFium, qpdf, OOXML ayrıştırıcılar) ve yerel arayüzlerini bağımsız olarak test etmek, performansını ölçmek ve sınır durumlarını sınamak için kullanılan açık lisanslı test korpusunu barındırır.

Korpus altyapısı AGPL-3.0 uyumlu, telemetrisiz ve tamamen standart Python (Python 3.14 stdlib) kullanılarak inşa edilmiştir.

---

## 1. Korpus Kategorileri ve Dosya Sayıları

Toplam **161 dosya** (~35.53 MB), 13 ayrı kategoride organize edilmiştir. Her kategoride en az 5 dosya bulunmaktadır:

| Kategori | Dosya Sayısı | Boyut (MB) | Açıklama ve Kapsam | Birincil Kaynaklar |
| :--- | :---: | :---: | :--- | :--- |
| `pdf-basic` | 15 | 1.09 | Temel PDF yapıları, metin, vektör şekiller, çoklu sayfalar, sayfa etiketleri, yer imleri, bağlantılar | Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0) |
| `pdf-large` | 5 | 16.42 | 500+ sayfalık resmi kamu belgeleri (CFR Title 12 [1.192 sayfa], CFR Title 26 [780 sayfa], 9/11 Raporu [585 sayfa], FCIC Raporu [662 sayfa]) ve 2000 sayfalık sentetik stres belgesi | US GovInfo (Kamu Malı), RBD OFİS (AGPL-3.0) |
| `pdf-encrypted` | 13 | 0.04 | Standart parola şifrelemeleri: RC4 40-bit, RC4 128-bit, AES-256, geçersiz parola anahtarları, şifreli ekler | Chromium PDFium (BSD-3-Clause), qpdf (Apache-2.0), Mozilla pdf.js (Apache-2.0) |
| `pdf-corrupt` | 17 | 0.49 | Bozuk/hatalı yapılar: geçersiz xref, bozuk sözlük anahtarları, kesik döküman, fuzzed girdiler | Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0), qpdf (Apache-2.0), RBD OFİS (AGPL-3.0) |
| `pdf-forms` | 14 | 3.25 | Etkileşimli AcroForm alanları (metin, liste, seçim kutuları, hesaplama zincirleri, IRS f1040) ve dinamik XFA formları | Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0) |
| `pdf-annotations` | 14 | 0.58 | PDF açıklamaları: vurgu (highlight), serbest çizim (ink), damga (stamp), dosya eki, çizgi, serbest metin, kenarlık stilleri | Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0) |
| `pdf-scanned` | 8 | 4.31 | Yalnızca resim içeren taranmış dökümanlar: 1934 Resmî Gazete sayıları (sayı 2741, 2865, 2888), CCITT Faks, JBIG2, JPEG | Wikimedia Commons (Kamu Malı), Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0) |
| `pdf-pdfa` | 12 | 0.09 | veraPDF resmi doğrulama korpusundan PDF/A-1a, PDF/A-1b, PDF/A-2b, PDF/A-3b geçerli ve geçersiz standart testleri | veraPDF-corpus (MPL-2.0 / GPL-3.0+) |
| `pdf-signed` | 9 | 0.19 | Dijital imzalı ve sertifikalı belgeler: PKCS#7, PAdES, DocMDP sertifikasyon, artımlı güncellemeyle çift imza, imza aralığı anomalisi | Chromium PDFium (BSD-3-Clause), Mozilla pdf.js (Apache-2.0), pyHanko (MIT) |
| `pdf-cjk-rtl-turkish` | 12 | 4.15 | Çok dilli metin ve yazı tipi testi: Basitleştirilmiş Çince, Japonca Shift-JIS, Arapça RTL CID, Türkçe Vikipedi rehberleri ve standart Helvetica fontlu Türkçe test belgesi | Mozilla pdf.js (Apache-2.0), Chromium PDFium (BSD-3-Clause), Wikimedia Commons (CC-BY-SA-4.0), RBD OFİS (AGPL-3.0) |
| `docx` | 15 | 0.30 | Karmaşık Word dökümanları: tablolar, stiller, listeler, üstbilgi/altbilgi, grafik şablonu, revizyonlar | Apache POI (Apache-2.0) |
| `xlsx` | 13 | 1.23 | Hesap tabloları: formüller, köprüler, koruma şifreleri, İBB Açık Veri Portalı Türkçe veri tabloları (kamulaştırma maliyetleri, hane büyüklükleri) | Apache POI (Apache-2.0), İBB Açık Veri Portalı (CC-BY-4.0) |
| `pptx` | 14 | 3.39 | Sunum dökümanları: çubuk ve pasta grafik şablonları, şekil hiyerarşileri, gradyanlar, açıklamalar, kırpılmış görseller | Apache POI (Apache-2.0) |
| **TOPLAM** | **161** | **35.53 MB** | **13 kategoride eksiksiz doğrulanmış açık test korpusu** | **Tümü açık kaynak / serbest lisanslı** |

---

## 2. Lisanslama ve Kaynak Güvenilirliği

RBD OFİS Ajan Anayasası (`AGENTS.md`) ve `docs/adr/0003-yayin-uyumlulugu.md` kuralları uyarınca korpustaki tüm dosyalar açık ve yeniden dağıtılabilir lisanslara sahiptir:

- **Apache-2.0:** Mozilla pdf.js, Apache POI, qpdf test suite
- **BSD-3-Clause:** Chromium PDFium test resources
- **MIT:** pyHanko dijital imza test korpusu
- **MPL-2.0 / GPL-3.0+:** veraPDF PDF/A test suite
- **Public Domain / Resmi Yayın:** US GovInfo resmi yayınları (CFR Title 12 [1.192 sayfa], CFR Title 26 [780 sayfa], 9/11 Raporu, FCIC Raporu), T.C. Resmî Gazete 1934 taramaları (FSEK m. 31)
- **CC-BY-4.0 / CC-BY-SA-4.0:** İBB Açık Veri Portalı veri setleri, Türkçe Vikipedi dökümanları
- **AGPL-3.0-or-later:** RBD OFİS projesi tarafından üretilen sentetik test belgeleri (`corpus/generate.py`)

Her dosyanın kaynak URL'si, kesin SHA-256 sağlama değeri, lisansı ve boyutu `corpus/manifest.json` dosyasında kayıtlıdır.

---

## 3. Kullanım

### A. Korpus Dosyalarını İndirme ve Doğrulama (`corpus/fetch.py`)

`fetch.py`, manifestteki tüm dosyaları `corpus/files/<kategori>/` dizinine indirir ve her bir dosyanın SHA-256 sağlama değerini kontrol eder.

- **Tam korpusu indir ve doğrula:**
  ```bash
  python corpus/fetch.py
  ```

- **Belirli bir kategoriyi filtrele:**
  ```bash
  python corpus/fetch.py --category pdf-forms
  python corpus/fetch.py --category docx
  ```

- **Önbelleği zorlayarak yeniden indir:**
  ```bash
  python corpus/fetch.py --force
  ```

- **Özellikler:**
  - **Idempotent:** Dosya yerelde mevcut ve SHA-256 değeri eşleşiyorsa indirme yapmaz (0 byte ağ transferi).
  - **Resumable (Sürdürülebilir):** Yarım kalmış indirmelerde HTTP Range (`Range: bytes=X-`) başlığını kullanarak indirmeyi kaldığı yerden devam ettirir.
  - **Hata Yönetimi:** Ağ veya sağlama değeri hatalarında sıfır dışı çıkış kodu (`exit(1)`) döner.

### B. Sentetik Sınır Durumlarını Üretme (`corpus/generate.py`)

`generate.py`, üçüncü taraf kütüphane gerektirmeden yalnızca Python standart kütüphanesini kullanarak 4 özel sınır durumu PDF'i üretir ve doğrudan ilgili kategori dizinine yazar:

1. `large-generated-2000-page.pdf` (`pdf-large`): 2000 sayfalık geçerli, paylaşımlı akış mimarili PDF stres belgesi.
2. `corrupt-generated-truncated.pdf` (`pdf-corrupt`): Akışın ortasında aniden kesilmiş, `trailer` veya `%%EOF` içermeyen eksik dosya.
3. `corrupt-generated-broken-xref.pdf` (`pdf-corrupt`): Geçersiz `startxref` ofsetine ve bozuk `xref` tablosu girdilerine sahip dosya (onarım motoru testi).
4. `tr-generated-standard-fonts.pdf` (`pdf-cjk-rtl-turkish`): Standart Type 1 font (Helvetica), `/Differences` tablosu, doğru sekizli kaçış dizileri (`\303`, `\302` vb.) ve `/ToUnicode` CMap akışı kullanılarak tüm Türkçe karakterleri (`İ, ı, Ş, ş, Ğ, ğ, Ç, ç, Ö, ö, Ü, ü`) metin kopyalama/arama uyumlu kodlayan ham PDF.

- **Çalıştırma:**
  ```bash
  python corpus/generate.py
  ```

Çıktılar doğrudan ilgili kategori klasörlerine (`corpus/files/pdf-large/`, `corpus/files/pdf-corrupt/`, `corpus/files/pdf-cjk-rtl-turkish/`) kaydedilir.

### C. Güvenlik ve Bütünlük Doğrulamaları

- **CWE-22 Path Traversal Koruması:** `fetch.py` ve `tools/build_corpus_manifest.py`, kategori ve dosya adı tanımlayıcılarını sıkı regex denetiminden (`^[a-z0-9_-]+$`, `^[a-zA-Z0-9_.-]+$`) geçirir ve `is_relative_to` ile çalışma alanı dışına yazımı engeller.
- **URL Şeması Sınırlaması:** Yalnızca güvenli `https://` ve yerel sentetik üretici `synthetic://` şemaları kabul edilir (`http://`, `file://`, `ftp://` kesinlikle reddedilir).
- **Magic-Byte ve HTML Doğrulaması:** Dosyalar indirilirken veya doğrulanırken gövde incelenir; sunucu hata sayfaları (`<!DOCTYPE html>`, `<html>`) otomatik olarak reddedilir; döküman uzantısına göre geçerli sihirli baytlar (`%PDF-`, `PK\x03\x04`) kontrol edilir.

---

## 4. `manifest.json` Şeması

`corpus/manifest.json` dosyası aşağıdaki şemaya uygun bir JSON dizisi içerir:

```json
{
  "id": "pdfium-about-blank",
  "category": "pdf-basic",
  "filename": "pdfium-about-blank.pdf",
  "url": "https://raw.githubusercontent.com/chromium/pdfium/main/testing/resources/about_blank.pdf",
  "sha256": "1bc7df1175ebba23e7eebda8b61e27a7c13da6647223b09228d54231bfa2b3ae",
  "license": "BSD-3-Clause",
  "size_bytes": 735,
  "notes": "Minimal blank PDF page"
}
```

Tüm 161 girdide `id`, `category`, `filename`, `url`, `sha256`, `license`, `size_bytes` ve `notes` alanları eksiksiz doldurulmuştur.
