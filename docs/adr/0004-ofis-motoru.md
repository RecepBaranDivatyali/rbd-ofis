# ADR-0004: Ofis Düzenleme Motoru — Çevrimdışı Hafif HTML Düzenleyici + Saf Rust OOXML

- **Durum:** Revize edildi (Başlangıçta ONLYOFFICE değerlendirildi; Faz 6 uygulamasında hafif HTML contenteditable düzenleyici ve saf Rust OOXML mimarisi benimsendi)
- **İlk Tarih:** 2026-11-04
- **Revizyon Tarihi:** 2026-11-07

## Revizyon Notu (2026-11-07)
Başlangıçta DOCX/XLSX/PPTX düzenleme ve dönüşüm için ONLYOFFICE (`sdkjs` + `x2t`) bileşenlerinin gömülmesi değerlendirilmişti. Ancak yapılan analiz ve Faz 6 prototipleme sürecinde aşağıdaki nedenlerle bu yaklaşımdan vazgeçilmiştir:
1. **Aşırı Paket Boyutu ve Kaynak Tüketimi:** ONLYOFFICE bileşenleri ve x2t dönüştürücüsü uygulama boyutunu ve bellek kullanımını aşırı artırmaktadır (mobil cihazlarda kararsızlık riski).
2. **Lisans ve Marka Kısıtları:** ONLYOFFICE AGPL Bölüm 7 gereğince logoların ve marka atıflarının arayüzde zorunlu tutulması RBD OFİS'in bağımsız tasarım hedefleriyle uyuşmamaktadır.
3. **Gerçek Mimari:** Projede ONLYOFFICE kodu veya kütüphaneleri **kullanılmamaktadır**. Bunun yerine:
   - **Kullanıcı Arayüzü:** Windows'ta WebView2, Android'de System WebView ve Apple'da WKWebView içinde çalışan; sıfır ağ erişimli (`https://rbd-office.local/`), hafif HTML `contenteditable` tabanlı WYSIWYG düzenleyici tuvali geliştirilmiştir.
   - **Çekirdek OOXML İşleme:** Saf Rust `zip` ve XML ayrıştırma (`core/crates/rbd-engine/src/ooxml.rs`) ile DOCX, XLSX ve PPTX metin çıkarma, belge denetleme ve temel DOCX paket oluşturma (`create_simple_docx`) uygulanmıştır.
   - Bu çözüm WPS seviyesinde devasa bir ofis paketi iddiasında olmayıp, hafif ofis dökümanı okuma, düzenleme ve DOCX üretimi ihtiyaçlarını karşılayan yerel ve çevrimdışı bir yapıdır.

---

## Tarihsel Karar Kaydı (Önceki Durum)

### Karar (Öneri Aşaması)
DOCX/XLSX/PPTX görüntüleme, düzenleme ve Office → PDF dönüşümü için **ONLYOFFICE** bileşenleri (AGPL-3.0) gömülecek:
- `sdkjs` + `web-apps` (JS/Canvas editörler): Windows'ta **WebView2**, Android'de **System WebView** içinde, sunucusuz çalışır.
  - Windows: `SetVirtualHostNameToFolderMapping`
  - Android: `WebViewAssetLoader`
  - Yerel TCP portu açılmaz.
- `x2t` dönüştürücü:
  - Windows: yerel derleme (Rust çekirdekten çağrılır)
  - Android: CryptPad'in `x2t.wasm` derlemesi (Web Worker içinde)

### Elenen seçenekler
- **LibreOfficeKit / Collabora:** Android'de 250 MB APK / 800 MB disk; Windows'ta derlemesi çok zor; dokunmatik deneyimi zayıf. Yalnızca dönüştürme yedeği olarak tutulur.
- **Univer:** DOCX/XLSX/PPTX içe/dışa aktarma modülleri kapalı kaynak (Pro). Kurallarımızı ihlal ettiği için elendi.
- **Euro-Office (fork):** Hukuki anlaşmazlık sürüyor. Yalnızca izleniyor.

### Değerlendirilen Yükümlülükler (Revizyonla Hükümsüz Kalan)
- ONLYOFFICE kodu kullanılmadığı için arayüzde ONLYOFFICE logosu veya "Powered by ONLYOFFICE" atfı zorunluluğu kalmamıştır; tüm bu atıflar arayüzlerden kaldırılmıştır.
