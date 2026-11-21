// RBD OFİS — 100% Client-Side Offline Demo & Bilingual Portal (Zero Network Calls)

const SAMPLES = {
    contract: `HİZMET VE GİZLİLİK SÖZLEŞMESİ (2026)
Madde 1: RBD OFİS, tamamen çevrimdışı çalışan, reklamsız ve ücretsiz bir açık kaynak (GNU AGPL-3.0) ofis ve PDF paketidir.
Madde 2: Uygulama hiçbir telemetri, analitik veya kullanıcı verisi toplamaz; Android sürümünde INTERNET izni dahi bulunmamaktadır.
Madde 3: Adli redaksiyon özelliği, PDF içerisindeki metin operatörlerini (Tj ve TJ) ve taranmış XObject görsel piksellerini bayt düzeyinde kalıcı olarak imha eder.
Madde 4: Belgeler yerel cihaz belleğinde Rust çekirdeği, PDFium ve saf Rust Hayro motorları ile işlenir.
Madde 5: Çevrimdışı yapay zeka asistanı, TF-ISF cümle önem puanlaması kullanarak internet bağlantısı olmadan özetleme ve sayfa referanslı soru-cevap sağlar.`,
    tech: `RBD OFİS MİMARİ VE PERFORMANS RAPORU
1. Çekirdek Mimarisi: Tüm iş mantığı Rust dilinde (rbd-engine, rbd-core, rbd-ffi) geliştirilmiştir ve FFI sınırında catch_unwind ile RwLock eşzamanlılık koruması uygulanır.
2. Masaüstü ve Mobil: Windows sürümü .NET 9 WPF ve 4K Per-Monitor V2 DPI ölçekleme kullanır; statik CRT derlemesi sayesinde temiz Windows kurulumlarında VCRUNTIME140.dll gerektirmeden çalışır.
3. Android ve Huawei Uyumu: Android 15 için 16 KB bellek sayfası hizalaması (-Wl,-z,max-page-size=16384) desteklenir ve Google Play Services bağımlılığı olmadığı için Huawei AppGallery cihazlarında yerel çalışır.
4. Belge Tarayıcı: Kamera kareleri ITU-R BT.601 parlaklık formülü ve çift tepe Otsu eşikleme algoritması ile yüksek kontrastlı siyah-beyaz PDF sayfalarına dönüştürülür.`
};

const STOP_WORDS = new Set([
    "ve", "veya", "ile", "için", "bir", "bu", "da", "de", "mi", "mu", "olarak", "olan", "gibi", "daha", "çok", "her", "kadar",
    "the", "and", "or", "for", "with", "this", "that", "from", "into", "are", "was", "were", "has", "have", "had"
]);

const I18N = {
    tr: {
        nav_features: "Özellikler",
        nav_playground: "Canlı Deneyimle",
        nav_download: "İndir",
        nav_arch: "Mimari & Gizlilik",
        hero_badge: "🔒 %100 Çevrimdışı • Sıfır Telemetri • AGPL-3.0 Açık Kaynak",
        hero_title: "Belgeleriniz Yalnızca Sizin Cihazınızda Kalır.",
        hero_subtitle: "Abonelik dayatmayan, reklam göstermeyen ve internete tek bir bayt bile göndermeyen yüksek hızlı Rust çekirdekli PDF ve Ofis paketi.",
        cta_download: "⬇ Ücretsiz İndir (v1.0.0)",
        cta_try: "⚡ Tarayıcıda Hemen Dene",
        pg_title: "⚡ Tarayıcı İçi Çevrimdışı Deneyim Alanı",
        pg_btn_summarize: "📊 Belgeyi Özetle",
        pg_btn_ask: "Sor",
        dl_title: "Platformunuz İçin İndirin (v1.0.0)"
    },
    en: {
        nav_features: "Features",
        nav_playground: "Live Playground",
        nav_download: "Download",
        nav_arch: "Architecture & Privacy",
        hero_badge: "🔒 100% Offline • Zero Telemetry • AGPL-3.0 Open Source",
        hero_title: "Your Documents Stay Strictly on Your Device.",
        hero_subtitle: "High-speed Rust-powered PDF & Office suite with zero subscriptions, zero ads, and zero bytes sent over the network.",
        cta_download: "⬇ Download Free (v1.0.0)",
        cta_try: "⚡ Try in Browser Now",
        pg_title: "⚡ In-Browser Offline Playground",
        pg_btn_summarize: "📊 Summarize Document",
        pg_btn_ask: "Ask",
        dl_title: "Download for Your Platform (v1.0.0)"
    }
};

let currentLang = "tr";

function tokenize(text) {
    return (text || "")
        .toLowerCase()
        .replace(/[^\p{L}\p{N}\s-]/gu, " ")
        .split(/\s+/)
        .filter(w => w.length > 2 && !STOP_WORDS.has(w));
}

function splitSentences(text) {
    return (text || "")
        .split(/(?<=[.!?])\s+|\n+/)
        .map(s => s.trim())
        .filter(s => s.length > 8);
}

function summarizeLocal(text, maxSentences = 3) {
    const words = (text || "").trim() ? (text || "").trim().split(/\s+/) : [];
    const sentences = splitSentences(text);
    const freq = new Map();
    for (const tok of tokenize(text)) {
        freq.set(tok, (freq.get(tok) || 0) + 1);
    }

    const scored = sentences.map((s, index) => {
        const toks = tokenize(s);
        let score = 0;
        for (const t of toks) score += (freq.get(t) || 0);
        if (toks.length > 0) score /= Math.sqrt(toks.length);
        if (index === 0) score *= 1.15;
        return { sentence: s, index, score };
    });

    scored.sort((a, b) => b.score - a.score);
    const top = scored.slice(0, maxSentences).sort((a, b) => a.index - b.index);
    const topKeywords = Array.from(freq.entries())
        .sort((a, b) => b[1] - a[1])
        .slice(0, 6);

    return {
        wordCount: words.length,
        sentenceCount: sentences.length,
        readingMinutes: Math.max(0.1, words.length / 200),
        summarySentences: top.map(x => x.sentence),
        topKeywords
    };
}

function askLocal(text, query) {
    const qTokens = tokenize(query);
    const sentences = splitSentences(text);
    if (qTokens.length === 0 || sentences.length === 0) {
        return { found: false, answer: "Lütfen geçerli bir soru girin." };
    }

    let best = null;
    for (let i = 0; i < sentences.length; i++) {
        const s = sentences[i];
        const sLower = s.toLowerCase();
        let hits = 0;
        for (const qt of qTokens) {
            if (sLower.includes(qt)) hits++;
        }
        if (hits > 0) {
            const confidence = Math.min(0.99, 0.45 + (hits / qTokens.length) * 0.5);
            if (!best || confidence > best.confidence) {
                best = { found: true, answer: s, confidence, sentenceIndex: i + 1 };
            }
        }
    }
    return best || { found: false, answer: "Belgede bu sorguyla eşleşen doğrudan bir cümle bulunamadı." };
}

function renderSummary() {
    const input = document.getElementById("aiDocInput").value;
    const res = summarizeLocal(input, 3);

    document.getElementById("metricWords").textContent = `Sözcük: ${res.wordCount}`;
    document.getElementById("metricSentences").textContent = `Cümle: ${res.sentenceCount}`;
    document.getElementById("metricTime").textContent = `Okuma: ~${res.readingMinutes.toFixed(1)} dk`;

    const summaryEl = document.getElementById("aiSummaryOutput");
    summaryEl.textContent = res.summarySentences.map(s => `• ${s}`).join("\n");

    const kwEl = document.getElementById("aiKeywordsOutput");
    kwEl.innerHTML = "";
    for (const [word, count] of res.topKeywords) {
        const span = document.createElement("span");
        span.className = "kw-chip";
        span.textContent = `#${word} (${count})`;
        kwEl.appendChild(span);
    }
}

function renderAsk() {
    const input = document.getElementById("aiDocInput").value;
    const query = document.getElementById("aiQueryInput").value;
    const ans = askLocal(input, query);
    const out = document.getElementById("aiAskOutput");
    if (ans.found) {
        out.textContent = `✅ Yanıt (Madde #${ans.sentenceIndex} • Güven: %${Math.round(ans.confidence * 100)}):\n${ans.answer}`;
    } else {
        out.textContent = ans.answer;
    }
}

document.addEventListener("DOMContentLoaded", () => {
    document.getElementById("yearSpan").textContent = new Date().getFullYear();

    const aiDocInput = document.getElementById("aiDocInput");
    aiDocInput.value = SAMPLES.contract;
    renderSummary();

    document.getElementById("runSummaryBtn").addEventListener("click", renderSummary);
    document.getElementById("runAskBtn").addEventListener("click", renderAsk);
    document.getElementById("aiQueryInput").addEventListener("keydown", e => {
        if (e.key === "Enter") renderAsk();
    });

    document.querySelectorAll("[data-sample]").forEach(btn => {
        btn.addEventListener("click", () => {
            const key = btn.getAttribute("data-sample");
            if (SAMPLES[key]) {
                aiDocInput.value = SAMPLES[key];
                renderSummary();
            }
        });
    });

    // Playground Tabs
    document.querySelectorAll(".pg-tab").forEach(tab => {
        tab.addEventListener("click", () => {
            document.querySelectorAll(".pg-tab").forEach(t => t.classList.remove("active"));
            document.querySelectorAll(".pg-panel").forEach(p => p.classList.remove("active"));
            tab.classList.add("active");
            const target = tab.getAttribute("data-tab");
            document.getElementById(`pg-panel-${target}`).classList.add("active");
        });
    });

    // Web Editor actions
    document.querySelectorAll("[data-cmd]").forEach(btn => {
        btn.addEventListener("click", () => {
            document.execCommand(btn.getAttribute("data-cmd"), false, null);
        });
    });

    document.getElementById("insertSampleTableBtn").addEventListener("click", () => {
        const editor = document.getElementById("webEditorCanvas");
        const table = document.createElement("table");
        table.innerHTML = "<tr><th>Bileşen</th><th>Durum</th></tr><tr><td>Rust Çekirdek</td><td>%100 Çevrimdışı</td></tr>";
        editor.appendChild(table);
    });

    document.getElementById("printPdfBtn").addEventListener("click", () => {
        window.print();
    });

    document.getElementById("downloadMdBtn").addEventListener("click", () => {
        const text = document.getElementById("webEditorCanvas").innerText || "";
        const blob = new Blob([text], { type: "text/markdown;charset=utf-8" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "RBD_OFIS_Notlar.md";
        a.click();
        URL.revokeObjectURL(url);
    });

    // Theme toggle
    const themeBtn = document.getElementById("themeToggleBtn");
    themeBtn.addEventListener("click", () => {
        document.body.classList.toggle("light-theme");
        themeBtn.textContent = document.body.classList.contains("light-theme") ? "🌙" : "☀️";
    });

    // Language toggle
    const langBtn = document.getElementById("langToggleBtn");
    langBtn.addEventListener("click", () => {
        currentLang = currentLang === "tr" ? "en" : "tr";
        langBtn.textContent = currentLang === "tr" ? "EN" : "TR";
        const dict = I18N[currentLang];
        document.querySelectorAll("[data-i18n]").forEach(el => {
            const key = el.getAttribute("data-i18n");
            if (dict[key]) el.textContent = dict[key];
        });
    });
});
