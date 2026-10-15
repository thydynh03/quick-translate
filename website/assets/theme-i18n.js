(() => {
  // ------------------------------------------------------------ Theme Management
  const STORAGE_KEY_THEME = "swipelate-theme";
  const STORAGE_KEY_LANG = "swipelate-lang";

  function getSystemTheme() {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }

  function getActiveTheme() {
    const saved = localStorage.getItem(STORAGE_KEY_THEME);
    if (saved === "light" || saved === "dark") return saved;
    return getSystemTheme();
  }

  function applyTheme(theme) {
    document.documentElement.dataset.theme = theme;
    const btn = document.getElementById("theme-toggle");
    if (btn) {
      const isDark = theme === "dark";
      btn.innerHTML = isDark
        ? '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>'
        : '<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';
      const currentLang = getActiveLang();
      btn.title = isDark
        ? (currentLang === "en" ? "Dark mode active. Click for light mode" : "Đang chế độ tối. Bấm để bật chế độ sáng")
        : (currentLang === "en" ? "Light mode active. Click for dark mode" : "Đang chế độ sáng. Bấm để bật chế độ tối");
      btn.setAttribute("aria-label", btn.title);
    }
  }

  function toggleTheme() {
    const current = getActiveTheme();
    const next = current === "dark" ? "light" : "dark";
    localStorage.setItem(STORAGE_KEY_THEME, next);
    applyTheme(next);
  }

  // ------------------------------------------------------------ Language Dictionary
  const I18N = {
    en: {
      // Common navigation & headers
      "nav-skip": "Skip to content",
      "nav-how": "How it works",
      "nav-new": "What's new",
      "nav-offline": "Offline translation",
      "nav-docs": "Documentation",
      "nav-license": "License",
      "nav-download": "Download",
      "nav-home": "Home",
      "nav-stats": "Statistics",
      "nav-privacy": "Privacy policy",

      // Index Hero
      "hero-title": "Select to translate.",
      "hero-desc": "Translate any text on your screen in a snap: select in any app, and the translation appears right in place. Powered by offline models and OCR.",
      "hero-win": "Download for Windows",
      "hero-mac": "Download for macOS",
      "hero-phone": "Study on phone",
      "hero-release": "Free, open source, no account required.",

      // Scene Tabs
      "tab-select": "Paragraph",
      "tab-word": "Single word",
      "tab-ocr": "Screen OCR",
      "tab-offline": "Offline",

      // Bento Grid & Features
      "f-seamless-title": "Zero context switching",
      "f-seamless-desc": "Select text in Word, browser, PDF, Claude, or Slack: a small icon appears right next to your pointer. Click it, or press Alt+Q, and the translation displays directly below your selection.",
      "f-dict-title": "Dictionary definitions for single words",
      "f-dict-desc": "Double-click an English word to view Vietnamese translations by part of speech, IPA phonetic transcription, and clear English definitions. Fast one-click shortcuts to Cambridge and Oxford dictionaries.",
      "f-ocr-title": "Alt + W for screen text OCR",
      "f-ocr-desc": "When text is inside an image, scanned PDF, video subtitle, or remote desktop where highlighting is impossible: press Alt+W, drag a box around the text, and Swipelate recognizes and translates it immediately.",
      "f-offline-title": "Translate without Internet",
      "f-offline-desc": "Helsinki-NLP Opus-MT models run directly on your CPU via ONNX Runtime without touching the cloud. Download once in Settings, then translate seamlessly during flights or network outages.",
      "f-profiles-title": "Per-application profiles",
      "f-profiles-desc": "Set specific translation engines or target languages for individual programs — such as offline engine in games or full sentence explanation in VS Code.",
      "f-phone-title": "Review saved cards on your phone",
      "f-phone-desc": "Bookmark words from the popup, then scan a QR code to study flashcards with spaced repetition right in your mobile browser without installing extra apps.",
      "f-subtitles-title": "Bilingual subtitles for video & YouTube",
      "f-subtitles-desc": "Pre-translate .srt or .vtt subtitle files without latency. Install the lightweight YouTube extension for bilingual dual subtitles on any video.",
      "f-clipboard-title": "Translation history & clipboard manager",
      "f-clipboard-desc": "Recent translations are automatically saved with configurable retention and limits. A built-in Spotlight-style clipboard manager makes pasting recent copies instantaneous.",
      "f-privacy-title": "Privacy first",
      "f-privacy-desc": "No tracking, no accounts, no ads. When offline, your text never leaves your device. When online, requests go only to the selected provider.",

      // Download section
      "dl-heading": "Download Swipelate",
      "dl-sub": "Free, open-source under the MIT license.",
      "dl-win-title": "Windows 10 / 11",
      "dl-win-btn": "Download installer (.exe)",
      "dl-mac-title": "macOS 11+ (Apple Silicon)",
      "dl-mac-btn": "Download disk image (.dmg)",
      "dl-phone-title": "Mobile phone",
      "dl-phone-desc": "Scan QR code in app Settings → Vocabulary to review your saved cards.",
      "dl-phone-btn": "Open web flashcards",

      // FAQ
      "faq-heading": "Frequently asked questions",
      "faq-q1": "Is Swipelate free?",
      "faq-a1": "Yes. Swipelate is 100% free and open source under the MIT license. Both Google translation and offline models require no account or API keys.",
      "faq-q2": "Can I translate without an Internet connection?",
      "faq-a2": "Yes. Once you download the offline model in Settings (~107 MB per language direction), Swipelate translates offline entirely on your CPU whenever network drops.",
      "faq-q3": "Which applications does Swipelate support?",
      "faq-a3": "Every app where you can highlight text: Chrome, Edge, Word, Adobe Acrobat, Claude desktop, Slack, VS Code. For images or scanned PDFs, use Alt+W screen OCR.",
      "faq-q4": "Where is my selected text sent?",
      "faq-a4": "When online, text is sent securely to your chosen service (default Google Translate). When offline, text never leaves your machine. The app collects no personal data.",
      "faq-q5": "Why do Windows or macOS display an installation warning?",
      "faq-a5": "As an open-source project without expensive corporate signing certificates, operating systems display an unrecognized publisher prompt. The Docs page provides step-by-step instructions and SHA-256 hashes to verify downloads.",

      // Footer
      "footer-tagline": "Select to translate on Windows and macOS. Free, open source under the MIT license.",
      "footer-doc": "Docs",
      "footer-license": "License",
      "footer-privacy": "Privacy",
      "footer-stats": "Stats",

      // Learn Page
      "learn-title": "Vocabulary Learn",
      "learn-streak": "Day streak",
      "learn-due": "Due",
      "learn-new": "New",
      "learn-total": "Total",
      "learn-study": "Study cards",
      "learn-quiz": "Quiz mode",
      "learn-empty-title": "No cards saved yet.",
      "learn-empty-desc": "On desktop: click the bookmark icon on the translation popup, then go to Settings → Vocabulary → Study on phone and scan the QR code.",
      "learn-list": "Card list",
      "learn-search-ph": "Search cards",
      "learn-backup": "Backup",
      "learn-backup-desc": "Study progress is stored in this browser. Add this page to your home screen to use it offline like a native app.",
      "learn-export": "Export JSON",
      "learn-import": "Import JSON",
      "learn-done": "← Done",
      "learn-tap-hint": "Tap card to flip",
      "grade-0": "Again",
      "grade-1": "Hard",
      "grade-2": "Good",
      "grade-3": "Easy",
      "quiz-next": "Next question",
      "learn-finished": "Back to Home",

      // Stats Page
      "stats-title": "Swipelate Statistics",
      "stats-sub": "Real-time metrics queried directly from GitHub Releases and anonymous ping server.",
      "stats-refresh": "Refresh",
      "stats-dl-title": "Installer Downloads",
      "stats-dl-source": "Source: GitHub Releases",
      "stats-total-dl": "Total Downloads",
      "stats-latest-ver": "Latest Version",
      "stats-win-dl": "Windows",
      "stats-mac-dl": "macOS",
      "stats-users-title": "Active Users",
      "stats-users-source": "Source: Anonymous daily ping server",
      "stats-today": "Today",
      "stats-7d": "Past 7 days",
      "stats-30d": "Past 30 days",
      "stats-alltime": "All time devices",

      // 404 Page
      "404-title": "Page not found",
      "404-desc": "This page does not exist or has been moved.",
      "404-home": "Back to Home",
    },

    vi: {
      // Common navigation & headers
      "nav-skip": "Bỏ qua điều hướng",
      "nav-how": "Cách dùng",
      "nav-new": "Tính năng mới",
      "nav-offline": "Dịch offline",
      "nav-docs": "Tài liệu",
      "nav-license": "Giấy phép",
      "nav-download": "Tải về",
      "nav-home": "Trang chủ",
      "nav-stats": "Thống kê",
      "nav-privacy": "Quyền riêng tư",

      // Index Hero
      "hero-title": "Bôi đen là dịch.",
      "hero-desc": "Dịch nhanh mọi văn bản trên màn hình máy tính: bôi đen ở bất kỳ ứng dụng nào, bản dịch hiện ngay tại chỗ. Hỗ trợ dịch offline và OCR.",
      "hero-win": "Tải cho Windows",
      "hero-mac": "Tải cho macOS",
      "hero-phone": "Học trên điện thoại",
      "hero-release": "Miễn phí, mã nguồn mở, không cần tài khoản.",

      // Scene Tabs
      "tab-select": "Đoạn văn",
      "tab-word": "Từ đơn",
      "tab-ocr": "Quét màn hình",
      "tab-offline": "Mất mạng",

      // Bento Grid & Features
      "f-seamless-title": "Không chuyển cửa sổ",
      "f-seamless-desc": "Bôi đen trong Word, trình duyệt, PDF, Claude hay Slack: biểu tượng nhỏ hiện ngay cạnh con trỏ. Bấm vào đó, hoặc nhấn Alt + Q, bản dịch nằm ngay dưới vùng chọn.",
      "f-dict-title": "Một từ thì có thêm từ điển",
      "f-dict-desc": "Nhấp đúp vào một từ tiếng Anh để xem nghĩa tiếng Việt theo từ loại, phiên âm IPA và định nghĩa tiếng Anh. Có nút mở nhanh Cambridge và Oxford.",
      "f-ocr-title": "Alt + W khi chữ nằm trong ảnh",
      "f-ocr-desc": "Chữ trong ảnh, PDF scan, phụ đề video hoặc máy ảo không bôi đen được: nhấn Alt + W, kéo chuột quanh vùng cần đọc. Ứng dụng nhận dạng chữ rồi dịch ngay.",
      "f-offline-title": "Mất mạng vẫn dịch được",
      "f-offline-desc": "Model Opus-MT của Helsinki-NLP chạy thẳng trên CPU qua ONNX Runtime, không phụ thuộc đám mây. Tải một lần trong Cài đặt, yên tâm dùng trên máy bay hay khi đứt cáp.",
      "f-profiles-title": "Hồ sơ riêng cho từng ứng dụng",
      "f-profiles-desc": "Chọn nguồn dịch hoặc ngôn ngữ đích khác nhau cho từng phần mềm — ví dụ dịch offline trong game hoặc giải thích cả câu trong VS Code.",
      "f-phone-title": "Học lại từ đã lưu trên điện thoại",
      "f-phone-desc": "Đánh dấu từ trong popup để lưu vào danh sách. Quét mã QR một lần để mở trang flashcard trên điện thoại, có lặp lại ngắt quãng, không cần cài thêm app.",
      "f-subtitles-title": "Phụ đề song ngữ cho video và YouTube",
      "f-subtitles-desc": "Dịch trước cả file phụ đề .srt hoặc .vtt để xem phim không có độ trễ. Tiện ích YouTube kèm theo hiện phụ đề song ngữ khi xem video.",
      "f-clipboard-title": "Lịch sử và bảng clipboard",
      "f-clipboard-desc": "Mọi bản dịch được lưu lại, có tuỳ chọn tự động dọn dẹp theo ngày hoặc số lượng. Kèm bảng lịch sử clipboard mở bằng phím tắt kiểu Spotlight để dán nhanh.",
      "f-privacy-title": "Bảo vệ sự riêng tư",
      "f-privacy-desc": "Không theo dõi, không tài khoản, không quảng cáo. Khi dịch offline, văn bản không rời máy tính. Khi online, chỉ gửi tới dịch vụ bạn chọn.",

      // Download section
      "dl-heading": "Tải Swipelate",
      "dl-sub": "Miễn phí, mã nguồn mở theo giấy phép MIT.",
      "dl-win-title": "Windows 10 / 11",
      "dl-win-btn": "Tải bộ cài (.exe)",
      "dl-mac-title": "macOS 11+ (Apple Silicon)",
      "dl-mac-btn": "Tải bản DMG (.dmg)",
      "dl-phone-title": "Điện thoại",
      "dl-phone-desc": "Quét mã QR trong app Cài đặt → Từ vựng để ôn lại các từ đã lưu.",
      "dl-phone-btn": "Mở web flashcard",

      // FAQ
      "faq-heading": "Câu hỏi thường gặp",
      "faq-q1": "Swipelate có miễn phí không?",
      "faq-a1": "Có. Swipelate miễn phí và mã nguồn mở theo giấy phép MIT. Dịch bằng Google và dịch offline đều không cần tài khoản hay API key.",
      "faq-q2": "Không có mạng thì có dịch được không?",
      "faq-a2": "Được, sau khi bạn tải model offline trong phần Cài đặt (khoảng 107 MB cho mỗi chiều dịch). Khi mất mạng, ứng dụng tự chuyển sang dịch trên máy.",
      "faq-q3": "Swipelate dùng được trong những ứng dụng nào?",
      "faq-a3": "Mọi ứng dụng cho phép bôi đen và sao chép văn bản: trình duyệt, Word, PDF, Claude, Slack, Zalo, VS Code. Với ảnh hoặc PDF scan, dùng chế độ quét vùng màn hình.",
      "faq-q4": "Văn bản tôi bôi đen được gửi đi đâu?",
      "faq-a4": "Khi dịch online, đoạn văn bản được gửi tới dịch vụ dịch bạn chọn (mặc định là Google Translate). Khi dịch offline, văn bản không rời khỏi máy. Ứng dụng không có máy chủ riêng và không thu thập dữ liệu.",
      "faq-q5": "Vì sao Windows hoặc macOS cảnh báo khi cài?",
      "faq-a5": "Bộ cài chưa được ký bằng chứng chỉ trả phí nên hệ điều hành hiện cảnh báo với nhà phát hành chưa xác định. Trang tài liệu có hướng dẫn từng bước và mã SHA-256 để bạn kiểm tra tệp tải về.",

      // Footer
      "footer-tagline": "Bôi đen là dịch trên Windows và macOS. Miễn phí, mã nguồn mở theo giấy phép MIT.",
      "footer-doc": "Tài liệu",
      "footer-license": "Giấy phép",
      "footer-privacy": "Quyền riêng tư",
      "footer-stats": "Thống kê",

      // Learn Page
      "learn-title": "Học từ vựng",
      "learn-streak": "Số ngày học liên tiếp",
      "learn-due": "Cần ôn",
      "learn-new": "Thẻ mới",
      "learn-total": "Tổng số thẻ",
      "learn-study": "Học thẻ",
      "learn-quiz": "Trắc nghiệm",
      "learn-empty-title": "Chưa có thẻ nào.",
      "learn-empty-desc": "Trên máy tính: bấm nút đánh dấu trên popup dịch để lưu từ, rồi vào Cài đặt → Từ vựng → Học trên điện thoại và quét mã QR bằng camera.",
      "learn-list": "Danh sách thẻ",
      "learn-search-ph": "Tìm từ",
      "learn-backup": "Sao lưu",
      "learn-backup-desc": "Tiến độ học lưu trong trình duyệt này. Thêm trang vào màn hình chính để mở như một ứng dụng và học cả khi không có mạng.",
      "learn-export": "Xuất tệp",
      "learn-import": "Nhập tệp",
      "learn-done": "← Xong",
      "learn-tap-hint": "Chạm vào thẻ để lật",
      "grade-0": "Quên",
      "grade-1": "Khó",
      "grade-2": "Nhớ",
      "grade-3": "Dễ",
      "quiz-next": "Câu tiếp",
      "learn-finished": "Về trang chính",

      // Stats Page
      "stats-title": "Thống kê",
      "stats-sub": "Số liệu thật, đọc trực tiếp khi mở trang.",
      "stats-refresh": "Làm mới",
      "stats-dl-title": "Lượt tải bộ cài",
      "stats-dl-source": "Nguồn: GitHub Releases",
      "stats-total-dl": "Tổng lượt tải",
      "stats-latest-ver": "Bản mới nhất",
      "stats-win-dl": "Windows",
      "stats-mac-dl": "macOS",
      "stats-users-title": "Người dùng thực tế",
      "stats-users-source": "Nguồn: Máy chủ thống kê ẩn danh",
      "stats-today": "Hôm nay",
      "stats-7d": "7 ngày qua",
      "stats-30d": "30 ngày qua",
      "stats-alltime": "Tổng máy từ trước đến nay",

      // 404 Page
      "404-title": "Không tìm thấy trang",
      "404-desc": "Địa chỉ này không tồn tại hoặc đã được chuyển đi.",
      "404-home": "Về trang chủ",
    },
  };

  function getActiveLang() {
    const saved = localStorage.getItem(STORAGE_KEY_LANG);
    if (saved === "en" || saved === "vi") return saved;
    return "vi";
  }

  function applyLanguage(lang) {
    document.documentElement.lang = lang;
    const dict = I18N[lang] || I18N.vi;

    // Translate all elements with data-i18n
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.dataset.i18n;
      if (dict[key] && el.textContent !== dict[key]) {
        el.textContent = dict[key];
      }
    });

    // Translate placeholder attributes
    document.querySelectorAll("[data-i18n-ph]").forEach((el) => {
      const key = el.dataset.i18nPh;
      if (dict[key] && el.getAttribute("placeholder") !== dict[key]) {
        el.setAttribute("placeholder", dict[key]);
      }
    });

    // Update active states on lang switch buttons
    document.querySelectorAll(".lang-btn").forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.lang === lang);
    });

    // Refresh theme button title with correct language
    applyTheme(getActiveTheme());

    // Dispatch event so page scripts (site.js demo, stats.js, learn.js) can react
    window.dispatchEvent(new CustomEvent("swipelate-lang-changed", { detail: { lang } }));
  }

  function setLanguage(lang) {
    localStorage.setItem(STORAGE_KEY_LANG, lang);
    applyLanguage(lang);
  }

  // ------------------------------------------------------------ Inject & Wire Header Controls
  function setupControls() {
    // Check if controls already exist
    document.querySelectorAll(".top-controls").forEach((container) => {
      // Wire language buttons
      container.querySelectorAll(".lang-btn").forEach((btn) => {
        btn.onclick = () => setLanguage(btn.dataset.lang);
      });
      // Wire theme button
      const themeBtn = container.querySelector("#theme-toggle");
      if (themeBtn) {
        themeBtn.onclick = toggleTheme;
      }
    });

    // Apply active theme and language
    applyTheme(getActiveTheme());
    applyLanguage(getActiveLang());

    // Listen to system theme change if no override stored
    window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
      if (!localStorage.getItem(STORAGE_KEY_THEME)) {
        applyTheme(getSystemTheme());
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", setupControls);
  } else {
    setupControls();
  }

  // Export globally for page scripts
  window.SwipelateI18n = {
    getActiveLang,
    setLanguage,
    getActiveTheme,
    toggleTheme,
    t: (key) => (I18N[getActiveLang()] || I18N.vi)[key] || key,
  };
})();
