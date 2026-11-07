# ADR-0003: Windows ve Android Yayın Uyumluluğu

- **Durum:** Kabul edildi
- **Tarih:** 2026-11-04

## Bağlam
RBD OFİS, AGPL-3.0 lisanslı açık kaynak bir uygulama olarak Microsoft Store, GitHub Releases ve Google Play'de yayınlanacak. Yayın aşamasında lisans, mağaza politikası veya imzalama yüzünden engel çıkmaması gerekiyor.

## Kararlar

### A. Lisans bütünlüğü
1. **Proje lisansı: AGPL-3.0-or-later.** MuPDF (AGPL) ile birleşik eser olduğu için zorunlu.
2. **Kapalı kaynak kütüphane yasak.** AGPL'li MuPDF ile aynı ikili dosyaya kapalı kod bağlanırsa lisans ihlali olur. Bu nedenle:
   - ❌ Google ML Kit (OCR) → ✅ RapidOCR / PaddleOCR (Apache-2.0, ONNX)
   - ❌ Firebase, Crashlytics, AdMob, analitik SDK'ları → ✅ hiçbiri kullanılmaz
   - ✅ İşletim sistemi bileşenleri serbest: Windows.Media.Ocr, Android framework API'leri, WebView2. GPL'deki "sistem kütüphanesi" istisnası kapsamındadır.
3. **Kaynak kodu sunma yükümlülüğü:** Her sürümün tam kaynağı GitHub'da etiketlenir (`v1.2.3`). Uygulamanın "Hakkında" ekranında depo bağlantısı ve lisans metni yer alır.
4. **Üçüncü taraf bildirimleri:** `cargo-about` (Rust) ve Gradle lisans eklentisiyle otomatik üretilir, uygulama içinde gösterilir.
5. **Yazı tipleri:** Yalnızca OFL/Apache lisanslı yazı tipleri paketlenir (Noto, Liberation, Carlito ≈ Calibri, Caladea ≈ Cambria). Microsoft yazı tipleri paketlenmez; sistemde varsa kullanılır.
6. **Ofis motoru lisans durumu:** Projede ONLYOFFICE kodu kullanılmamaktadır (ADR-0004 revize edildi). Çevrimdışı hafif HTML düzenleyici ve saf Rust OOXML (MIT zip) kullanılmaktadır; arayüzde herhangi bir üçüncü taraf logosu veya atfı zorunluluğu bulunmamaktadır.
7. **Katkı:** Telif sahibi Recep Baran Divatyalı. Dış katkılar DCO (`Signed-off-by`) ile kabul edilir.

### B. Microsoft Store / Windows
- Bireysel geliştirici hesabı Microsoft Store'da **ücretsiz** (2024'ten beri).
- Store üzerinden dağıtılan MSIX paketini **Microsoft imzalar**; SmartScreen uyarısı çıkmaz.
- GitHub Releases'teki doğrudan indirme için imza seçenekleri: **SignPath Foundation** (açık kaynak projelere ücretsiz) veya Azure Trusted Signing (ücretli). İmzasız sürüm "beta" olarak işaretlenir.
- Store'a dosya ilişkilendirmeleri (.pdf, .docx…) manifestte bildirilir. "Varsayılan uygulama" dayatması yapılmaz (politika).

### C. Google Play / Android
- Geliştirici hesabı: 25 $ (tek seferlik). Yeni kişisel hesaplarda yayın öncesi **12 test kullanıcısıyla 14 günlük kapalı test** zorunlu. Bu süre plana eklendi.
- **Hedef API seviyesi** her yıl Google'ın istediği seviyeye yükseltilir (CI'da kontrol edilir).
- **Boyut:** Temel paket sınırı aşılmasın diye OCR/AI modelleri ve ofis motoru **Play Asset Delivery** ile ayrı paket olarak sunulur.
- **Veri güvenliği formu:** "Veri toplanmaz, paylaşılmaz." Bu beyanın doğru kalması için ağ izni (`INTERNET`) varsayılan sürümde **istenmez**. İleride bulut senkronizasyonu eklenirse yalnızca kullanıcı açtığında devreye girer.
- **Dosya erişimi:** `MANAGE_EXTERNAL_STORAGE` istenmez (Play bu izni sıkı denetler). Storage Access Framework ve "Birlikte aç" kullanılır.
- **Gizlilik politikası:** GitHub Pages'te yayınlanır (veri toplamadığımızı açıklayan kısa metin).
- **Mağaza metni:** Rakip markalar (Adobe, WPS) anahtar kelime olarak doldurulmaz (spam politikası).
- **F-Droid:** Kapalı bağımlılık olmadığı için F-Droid'de de yayınlanabilir (bonus kanal).

### D. Yapay zeka modelleri
- Her modelin lisansı ayrı ADR'de kayıt altına alınır. Tercih sırası: Apache-2.0/MIT (Qwen, RapidOCR, Docling modelleri). Gemma (Gemma Terms) yalnızca isteğe bağlı indirilebilir paket olarak sunulur.

## Sonuçlar
- Bazı "kolay" yollar (ML Kit, Firebase) kapalı. Bunun karşılığında mağaza reddi ve lisans ihlali riski minimuma iniyor.
- `cargo deny` ve bağımlılık lisans denetimi CI'da zorunlu adım olacak.
