# Apple App Store (iOS, iPadOS & Mac App Store) Yayın Kılavuzu

RBD OFİS, Apple ekosisteminde hem iPhone/iPad (iOS/iPadOS 16+) hem de Mac (macOS 13+ Ventura, Sonoma, Sequoia) için evrensel olarak tasarlanmıştır.

---

## 1. Ön Koşullar

1. **Apple Developer Hesabı:** [developer.apple.com](https://developer.apple.com) üzerinden yıllık kayıtlı geliştirici hesabı ($99/yıl).
2. **Mac Bilgisayar (veya Mac Cloud/GitHub Actions macOS Runner):** Xcode 15 veya 16 kurulu bir Mac.

---

## 2. App Store Connect Hazırlığı

1. **App Store Connect'e Giriş:**
   - [appstoreconnect.apple.com](https://appstoreconnect.apple.com) adresine gidin.
   - **Apps** ➔ **+** (Yeni Uygulama) seçin.
   - **Platforms:** `iOS` ve `macOS` (veya evrensel).
   - **Name:** `RBD OFİS`
   - **Primary Language:** `Turkish`
   - **Bundle ID:** `com.rbdofis.app`
   - **SKU:** `rbdofis-app-01`
   - **User Access:** `Full Access`

2. **Gizlilik Besin Değeri (App Privacy Nutrition Labels):**
   - **Data Collection (Veri Toplama):**  
     Apple'ın sorusu: *"Do you or your third-party partners collect any data from this app?"*  
     ➔ **NO, WE DO NOT COLLECT DATA FROM THIS APP (Hayır, bu uygulamadan hiçbir veri toplamıyoruz)**.  
     *(RBD OFİS ağ izni dahi istemediğinden bu beyan anında kabul edilir).*

---

## 3. Xcode ile Arşivleme ve Gönderme (Mac Üzerinde)

Proje `apps/apple` klasöründe tam bir Swift Package ve App Store uyumlu yapılandırmaya sahiptir:
- `apps/apple/Info.plist`: İzinler, belge tipleri ve kategori tanımlı.
- `apps/apple/RbdOfis.entitlements`: App Sandbox, User Selected Files Read-Write ve Camera tanımlı (Ağ istemcisi kapalı).

### Adım 3.1: Xcode ile Projeyi Açma
Mac üzerinde terminali açın:
```bash
cd "PROJELERİM/RBD OFİS/apps/apple"
open Package.swift
# Veya Xcode'da File -> New -> Project -> Multiplatform App seçip Sources/RbdOfis klasörünü ekleyin.
```

### Adım 3.2: Signing & Capabilities
- Target seçin ➔ **Signing & Capabilities** sekmesi.
- **Team:** Kendi Apple Developer Takımınızı seçin.
- **Bundle Identifier:** `com.rbdofis.app`
- **Capabilities:** `App Sandbox` (Mac için) ve `Camera` izinlerinin `RbdOfis.entitlements` ile eşleştiğinden emin olun.

### Adım 3.3: Product ➔ Archive ve Dağıtım
1. Hedef cihaz olarak **Any iOS Device (arm64)** veya **Any Mac (Apple Silicon / Intel)** seçin.
2. Menüden **Product ➔ Archive** seçin.
3. Arşivleme bittiğinde Organizer penceresi açılır.
4. **Distribute App** ➔ **App Store Connect** ➔ **Upload** adımlarını izleyin.
5. Xcode otomatik olarak kod imzalama sertifikalarını uygular ve paketi App Store Connect'e yükler.

---

## 4. TestFlight ve İncelemeye Gönderme

1. Paket yüklendikten 10-15 dakika sonra **TestFlight** sekmesinde görünür.
2. Dahili test grubuna kendinizi ekleyip iPhone, iPad veya Mac cihazınızda TestFlight uygulamasıyla son kontrolü yapın.
3. **App Store** sekmesine dönün:
   - Sürüm: `1.0.0`
   - Açıklama, ekran görüntüleri ve anahtar kelimeleri (`docs/publishing/METADATA_STORE_LISTING.md` dosyasından) girin.
   - **App Review Information (İnceleme Bilgileri):**  
     Uygulamada oturum açma (login) gerekmediği için *"Sign-in required"* kutucuğunu işaretlemeyin.  
     Notlar kısmına: *"RBD OFIS is a fully offline, zero-network office and PDF suite. No credentials or network connectivity required."* yazın.
4. **Add for Review (İncelemeye Ekle)** ➔ **Submit to App Review (İncelemeye Gönder)** butonuna basın.  
   Apple ortalama 24-48 saat içinde onaylar.
