# Huawei AppGallery Yayın Kılavuzu

RBD OFİS, tamamen açık kaynaklı, bağımsız Rust motoru üzerine kurulu olduğu ve hiçbir Google Play Services / Firebase bağımlılığı içermediği için **Huawei HarmonyOS ve EMUI cihazlarla %100 yerel ve tam uyumludur**.

---

## 1. Hazırlık ve İmzalı Paket Üretimi

Huawei AppGallery hem `.aab` hem de `.apk` kabul eder. Standart dağıtım için `.apk` veya `.aab` kullanabilirsiniz.

### Adım 1.1: Release APK veya AAB Derleme
```powershell
cd apps\android

# Ortam değişkenlerini ayarlayın (Google Play keystore'unun aynısını kullanabilirsiniz):
$env:RBD_KEYSTORE_FILE = "C:\guvenli_konum\rbdofis-release.keystore"
$env:RBD_KEYSTORE_PASSWORD = "SizinSifreniz"
$env:RBD_KEY_ALIAS = "rbdofis"
$env:RBD_KEY_PASSWORD = "SizinSifreniz"

# APK üretmek için:
.\gradlew.bat assembleRelease

# Veya AAB üretmek için:
.\gradlew.bat bundleRelease
```

Dosyalarınız şurada üretilir:
- APK: `apps/android/app/build/outputs/apk/release/app-release.apk` (~2.4 MB)
- AAB: `apps/android/app/build/outputs/bundle/release/app-release.aab` (~4.8 MB)

---

## 2. Huawei AppGallery Connect Adımları

1. **AppGallery Connect'e Giriş:**
   - [developer.huawei.com/consumer/en/service/josp/agc/index.html](https://developer.huawei.com/consumer/en/service/josp/agc/index.html) adresine gidin.
   - **My Apps (Uygulamalarım)** bölümüne girin ve **New App (Yeni Uygulama)** butonuna tıklayın.

2. **Temel Uygulama Bilgileri:**
   - **Package Type:** `APK` veya `App Bundle`
   - **Device:** `Phone` ve `Tablet` işaretleyin.
   - **App Name:** `RBD OFİS`
   - **Package Name:** `com.rbdofis.app` (İlk yüklemede dosyadan otomatik okunur)
   - **Default Language:** `Turkish (Turkey)` (veya İngilizce)
   - **App Category:** `App` ➔ `Productivity / Office (Üretkenlik / Ofis)`

3. **GMS Bağımsızlığı Doğrulaması:**
   - Huawei inceleme ekibi uygulamanın Google servislerine bağımlı olup olmadığını kontrol eder.
   - RBD OFİS içinde hiçbir Google Play Services / Firebase kütüphanesi yer almadığı için Huawei otomatik uyumluluk testlerinden **tek seferde sıfır uyarıyla geçer**.

4. **Uygulama Bilgileri & Görseller:**
   - `docs/publishing/METADATA_STORE_LISTING.md` dosyasındaki başlık ve açıklamaları girin.
   - 216x216 또는 512x512 uygulama simgesi yükleyin.
   - En az 3 adet dikey telefon/tablet ekran görüntüsü yükleyin.

5. **Sürüm Dosyasını Yükleme (Version Release):**
   - **Software Version** sekmesine gelin.
   - **Manage Package** butonuna basarak `app-release.apk` veya `app-release.aab` dosyasını yükleyin.
   - Sürüm Adı: `1.0.0`
   - Changelog (Yenilikler): `İlk resmi sürüm: Çevrimdışı PDF okuyucu, imza/çizim, form doldurma, belge tarayıcı ve çevrimdışı yapay zeka asistanı.`

6. **Gizlilik ve İncelemeye Gönderim:**
   - Gizlilik politikası URL'sini girin.
   - Uygulama içi satın alma: **Yok (Free)**.
   - Reklam: **Yok (Ad-free)**.
   - **Submit (Gönder)** butonuna basın. Huawei onay süreci genellikle 24-48 saat sürer.
