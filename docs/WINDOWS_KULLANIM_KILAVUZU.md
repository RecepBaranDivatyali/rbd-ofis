# RBD OFİS — Windows Alfa (v0.1.0-alpha) Kullanım Kılavuzu

RBD OFİS, tamamen açık kaynak, ücretsiz, reklamsız ve **%100 çevrimdışı** çalışan ofis ve PDF uygulamasıdır. Mantık katmanı yerel **Rust çekirdeğinde** çalışır, sunum katmanı ise yüksek performanslı **.NET 9 / WPF** arayüzü ile çizilir.

---

## 🚀 1. Hızlı Başlangıç

### Uygulamayı Çalıştırma:
Uygulama hiçbir kuruluma ihtiyaç duymayan taşınabilir (portable) bir pakettir:
1. `dist/windows-alpha/RbdOfis.Windows.exe` dosyasına çift tıklayın.
2. Ya da ZIP paketini (`dist/RbdOfis-v0.1.0-alpha-win-x64.zip`) istediğiniz klasöre çıkarıp `RbdOfis.Windows.exe` dosyasını çalıştırın.

---

## 📄 2. PDF Dosyalarını Varsayılan Yapma (Dosya İlişkilendirme)

RBD OFİS, bilgisayarınızda yönetici (admin) izni **istemeden** kullanıcı düzeyinde PDF dosyalarıyla ilişkilendirilebilir:

- **Arayüzden Tek Tıkla:** Pencerenin sağ üst başlık çubuğunda bulunan **"PDF ile İlişkilendir"** butonuna tıklayın.
- **Komut Satırından:**
  ```cmd
  RbdOfis.Windows.exe --register-associations
  ```
- **İlişkilendirmeyi Kaldırmak İçin:**
  ```cmd
  RbdOfis.Windows.exe --unregister-associations
  ```

---

## 🎨 3. Temel Özellikler ve Kısayollar

| Özellik | Açıklama | Kısayol |
|---|---|---|
| **Belge Aç** | Dosya seçici ile PDF belgesi açar. | `Ctrl + O` |
| **Kaydet** | Eklenen açıklamaları doğrudan PDF dosyasına işler. | `Ctrl + S` |
| **Geri Al (Undo)** | Eklenen veya silinen açıklamayı geri alır. | `Ctrl + Z` |
| **Yinele (Redo)** | Geri alınan açıklamayı yeniden uygular. | `Ctrl + Y` |
| **Canlı Arama** | Türkçe karakter duyarlı (`ı-I`, `i-İ`) hızlı metin araması. | `Ctrl + F` |
| **Tema Değiştir** | Koyu Tema (🌙) / Açık Tema (☀️) arasında anında geçiş. | Üst Bar |
| **Dil Değiştir** | Türkçe (TR) ve İngilizce (EN) arayüz desteği. | Üst Bar |

---

## 🛠️ 4. Sayfa Düzenleyici (Page Organizer)

Başlık çubuğundaki **"Sayfa Düzenleyici"** butonuna tıkladığınızda açılan görsel pencerede:
1. **Sıralama:** Sayfaları fare ile sürükleyip bırakarak yeniden sıralayabilirsiniz.
2. **Döndürme:** Seçili sayfayı 90° sağa veya sola döndürebilirsiniz.
3. **Silme:** İstenmeyen sayfaları seçip silebilirsiniz.
4. **Kırpma (Crop):** Kenar boşluklarını kırpabilirsiniz.
5. **Boş Sayfa Ekleme:** İstediğiniz konuma boş sayfa ekleyebilirsiniz.
6. **Uygula:** Tıklandığında tüm işlemler arka plandaki Rust çekirdeğinde icra edilir ve güncellenmiş PDF ana pencerede açılır.

---

## 🔒 5. Gizlilik ve Güvenlik (ADR-0003)
- RBD OFİS hiçbir telemetri, izleyici veya analitik kütüphanesi içermez.
- Sıfır ağ erişimi: Hiçbir veriniz internete gönderilmez, tamamen çevrimdışı çalışır.
