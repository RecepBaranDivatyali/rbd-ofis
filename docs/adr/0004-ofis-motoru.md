# ADR-0004: Ofis Düzenleme Motoru — ONLYOFFICE (gömülü, offline)

- **Durum:** Önerildi. Uygulamalı prototip (Faz 0, Deney 2b) sonrasında kesinleşecek.
- **Tarih:** 2026-11-04

## Karar
DOCX/XLSX/PPTX görüntüleme, düzenleme ve Office → PDF dönüşümü için **ONLYOFFICE** bileşenleri (AGPL-3.0) gömülecek:
- `sdkjs` + `web-apps` (JS/Canvas editörler): Windows'ta **WebView2**, Android'de **System WebView** içinde, sunucusuz çalışır.
  - Windows: `SetVirtualHostNameToFolderMapping`
  - Android: `WebViewAssetLoader`
  - Yerel TCP portu açılmaz.
- `x2t` dönüştürücü:
  - Windows: yerel derleme (Rust çekirdekten çağrılır)
  - Android: CryptPad'in `x2t.wasm` derlemesi (Web Worker içinde)

## Elenen seçenekler
- **LibreOfficeKit / Collabora:** Android'de 250 MB APK / 800 MB disk; Windows'ta derlemesi çok zor; dokunmatik deneyimi zayıf. Yalnızca dönüştürme yedeği olarak tutulur.
- **Univer:** DOCX/XLSX/PPTX içe/dışa aktarma modülleri kapalı kaynak (Pro). Kurallarımızı ihlal ettiği için elendi.
- **Euro-Office (fork):** Hukuki anlaşmazlık sürüyor. Yalnızca izleniyor.

## Yükümlülükler (AGPL Bölüm 7 ek şartları)
- Editör arayüzünde **ONLYOFFICE logosu / "Powered by ONLYOFFICE"** ibaresi korunur, kaldırılmaz.
- Uygulama adı **RBD OFİS** olarak kalır; ONLYOFFICE markası ürün adı olarak kullanılmaz.
- "Hakkında" ekranında Ascensio System SIA telif bildirimi ve kaynak bağlantısı yer alır.
- `web-apps` içindeki geri bildirim, sohbet, bulut ve hesap öğeleri yapılandırmayla kapatılır (ağ çağrısı yapılmaz).

## Prototipte doğrulanacaklar (Deney 2b)
1. WebView2'de tamamen offline açılış ve kaydetme; ağ trafiği sıfır olmalı (doğrulanacak).
2. Açılış süresi: masaüstü < 1,5 sn, mobil < 2,5 sn.
3. 4 GB RAM'li Android cihazda 100 sayfalık DOCX / 50.000 satırlık XLSX açılırken bellek yetersizliği olmamalı.
4. x2t PDF çıktısının görsel doğruluğu (korpus üzerinde).
5. Paket boyutu artışı < 65 MB (sıkıştırılmış).
