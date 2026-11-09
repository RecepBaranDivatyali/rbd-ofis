# Google Play Store Yayın Kılavuzu

RBD OFİS uygulamasını Google Play Console üzerinden yayınlamak için aşağıdaki adımları sırasıyla izleyin.

---

## 1. Hazırlık ve İmzalı AAB (Android App Bundle) Üretimi

Google Play Store, standart olarak `.aab` (Android App Bundle) formatını şart koşar.

### Adım 1.1: Release Keystore Oluşturma (Eğer mevcut değilse)
Proje kök dizininde veya güvenli bir klasörde PowerShell/Terminal açıp şu komutu çalıştırın:
```bash
keytool -genkey -v -keystore rbdofis-release.keystore -alias rbdofis -keyalg RSA -keysize 2048 -validity 10000
```
> **Önemli:** Belirlediğiniz şifreyi ve oluşturulan `rbdofis-release.keystore` dosyasını asla kaybetmeyin ve Git deposuna eklemeyin.

### Adım 1.2: Ortam Değişkenlerini Ayarlama
AAB derlemeden önce keystore bilgilerinizi terminalde tanımlayın:
```powershell
# Windows PowerShell için:
$env:RBD_KEYSTORE_FILE = "C:\guvenli_konum\rbdofis-release.keystore"
$env:RBD_KEYSTORE_PASSWORD = "SizinSifreniz"
$env:RBD_KEY_ALIAS = "rbdofis"
$env:RBD_KEY_PASSWORD = "SizinSifreniz"
```

### Adım 1.3: İmzalı Release AAB Paketini Derleme
```powershell
cd apps\android
.\gradlew.bat bundleRelease
```
Derleme tamamlandığında dosyanız şurada hazır olacaktır:
📁 `apps/android/app/build/outputs/bundle/release/app-release.aab` (~4.8 MB)

---

## 2. Google Play Console Adımları

1. **Google Play Console'a Giriş Yapın:**
   - [play.google.com/console](https://play.google.com/console) adresine gidin.
   - Geliştirici hesabınızı açın ve **Uygulama Oluştur (Create App)** butonuna tıklayın.

2. **Temel Bilgiler:**
   - **Uygulama Adı:** `RBD OFİS: PDF & Ofis Paketi`
   - **Varsayılan Dil:** `Türkçe - tr-TR` (İngilizce çeviri de eklenebilir)
   - **Uygulama mı / Oyun mu:** `Uygulama (App)`
   - **Ücretsiz mi / Ücretli mi:** `Ücretsiz (Free)`
   - Şartları kabul edip **Uygulama Oluştur**'a tıklayın.

3. **Uygulama İçeriği ve Gizlilik (En Kolay Bölüm):**
   - **Gizlilik Politikası (Privacy Policy):** RBD OFİS sıfır ağ erişimiyle çalıştığından şu metni veya linki verin:  
     `https://github.com/RecepBaranDivatyali/rbd-ofis/blob/main/docs/adr/0003-yayin-uyumlulugu.md`  
     *(Ayrıca `docs/publishing/METADATA_STORE_LISTING.md` içindeki hazır gizlilik metnini web sitenizde barındırabilirsiniz).*
   - **Veri Güvenliği (Data Safety) Formu:**
     - Uygulama herhangi bir veri topluyor mu veya paylaşıyor mu? ➔ **HAYIR (No)**.
     - Uygulamanın `INTERNET` izni yoktur; bu durum Google tarafından otomatik doğrulanır ve mağazada *"Veri toplanmıyor / Üçüncü taraflarla paylaşılmıyor"* rozeti alırsınız.
   - **Reklamlar (Ads):** Uygulama reklam içeriyor mu? ➔ **HAYIR (No)**.
   - **Uygulama Erişimi (App Access):** Özel oturum açma veya hesap gerekiyor mu? ➔ **Tüm özellikler kısıtlama olmaksızın kullanılabilir (All features are available without restriction)**.
   - **Hedef Kitle:** 18 yaş ve üzeri (veya 13+).
   - **İçerik Derecelendirmesi (IARC):** Kısa anketi doldurun (Şiddet yok, yetişkin içerik yok). Sonuç: **PEGI 3 / Herkes için uygun (Everyone)**.

4. **Mağaza Girişi (Store Listing):**
   - `docs/publishing/METADATA_STORE_LISTING.md` dosyasındaki başlık, kısa açıklama ve tam açıklamayı kopyalayıp yapıştırın.
   - 512x512 uygulama simgesi (App Icon) yükleyin.
   - 1024x500 özellik grafiği (Feature Graphic) yükleyin.
   - En az 2 adet telefon ekran görüntüsü yükleyin.

5. **Sürüm Yayınlama (Release):**
   - Sol menüden **Üretim (Production)** veya önce **Dahili Test (Internal Testing)** sekmesine gidin.
   - **Yeni Sürüm Oluştur (Create New Release)** butonuna tıklayın.
   - `app-release.aab` dosyasını sürükleyip bırakın.
   - Sürüm Adı: `1.0.0`
   - Sürüm Notları:  
     `RBD OFİS v1.0.0: Tamamen çevrimdışı, reklamsız, ücretsiz PDF okuyucu, açıklama, form doldurma, belge tarayıcı ve çevrimdışı yapay zeka asistanı.`
   - **İncelemeye Gönder** butonuna tıklayın. Google ortalama 1-3 gün içinde incelemeyi onaylar.
