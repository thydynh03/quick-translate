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
      // Document meta
      "title-index": "Swipelate – Select to translate in any app on Windows & macOS",
      "desc-index": "Swipelate is a free desktop translation app: select text in any application and see the translation instantly in-place. Supports English-Vietnamese, offline models, and screen OCR.",

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
      "hero-eyebrow": "Desktop translator for Windows & macOS",
      "hero-title": "Select to translate.<br>In any application.",
      "hero-desc": "Reading an email, PDF, or chat? Highlight the text you want to understand, and the translation appears right beneath it. No window switching, no copying to a browser.",
      "hero-win": "Download for Windows",
      "hero-mac": "Download for macOS",
      "hero-release": "Free, open source. No account required.",

      // Scene Tabs & Demo
      "tab-select": "Paragraph",
      "tab-word": "Single word",
      "tab-ocr": "Screen OCR",
      "tab-offline": "Offline",
      "demo-caption": "Highlight a block of text, click the icon that appears, and the translation shows right beneath it.",

      // Ways to translate
      "ways-title": "Three ways to get a translation",
      "ways-lead": "Choose the workflow that fits what you're doing. All three preserve your text selection and restore your original clipboard.",
      "way1-key": "Drag selection",
      "way1-title": "Select and click the trigger icon",
      "way1-desc": "Drag your mouse, double-click a word, or triple-click a paragraph. A subtle icon appears by your cursor; click it to view the translation. If ignored, it disappears automatically.",
      "way2-title": "Keyboard shortcut, no mouse needed",
      "way2-desc": "Select text using your keyboard and press the shortcut. You can customize the hotkey combination in Settings if it conflicts with another program.",
      "way3-title": "Screen area OCR scan",
      "way3-desc": "For unselectable text: images, scanned PDFs, paused video frames, or remote desktops. Drag a bounding box around the text; Swipelate recognizes and translates it immediately.",

      // New features
      "fresh-eyebrow": "Recently added features",
      "fresh-title": "More than just select-to-translate",
      "fresh-lead": "Four new capabilities for when text isn't easily selectable: video games, movies, subtitle-less media, and your copied text history.",
      "feat1-title": "Real-time on-screen translation",
      "feat1-desc": "Select full screen or drag an area. As text changes, translations update live and overlay in-place. Ideal for games, hardcoded video subtitles, and English-only tools. Cached sentences reappear instantly without re-translating.",
      "feat2-key": "Drag & drop files",
      "feat2-title": "Pre-translated subtitles for full movies",
      "feat2-desc": "Drop an <code>.srt</code> or <code>.vtt</code> file into the Subtitles tab, and get a translated file saved alongside the original. VLC or MPV picks it up automatically with zero playback lag. For YouTube, there is a dedicated <a href=\"docs.html#youtube\">extension for Chrome & Edge</a>.",
      "feat3-then": "then",
      "feat3-title": "Live audio caption translation",
      "feat3-desc": "Watching a video with no subtitles? Swipelate captures Windows Live Captions generated from system audio and translates sentence by sentence. You can hide the original English subtitles to keep only the translation at the bottom of the screen. Currently supports English audio input.",
      "feat4-tag": "macOS, experimental",
      "feat4-title": "Spotlight-style clipboard manager for macOS",
      "feat4-desc": "Access your clipboard history with a Spotlight-style frosted glass HUD that matches your system appearance. Paste items back, pin frequently used snippets, or translate them instantly. Content marked confidential by password managers is automatically excluded.",
      "feat4-find": "Search clipboard",

      // Dictionary
      "dict-title": "Look up a single word for full dictionary entries",
      "dict-p1": "Double-click an English word to see Vietnamese translations categorized by part of speech, IPA phonetic transcription, and concise English definitions. For deeper study, open the exact entry directly on Cambridge or Oxford Learner's with one click.",
      "dict-p2": "Inside the popup, every word in the source sentence is clickable. Encounter an unfamiliar word mid-sentence? Click it to inspect individually without having to select text again.",
      "dict-pos": "verb",
      "dict-mean": "to attend, be present, go regularly",
      "dict-def-label": "definition",

      // Offline Chain
      "off-title": "Translate without an Internet connection",
      "off-lead": "Swipelate falls back smoothly through three translation engines. No configuration is required for the first two.",
      "off-g-desc": "Default when online. No account and no API key required.",
      "off-local-title": "On-device offline model",
      "off-local-desc": "When offline or if Google is unreachable, Swipelate automatically falls back to pre-downloaded Helsinki-NLP Opus-MT models. Text never leaves your machine. Each language direction is ~107 MB, downloaded once in Settings.",
      "off-ai-title": "Claude or DeepL",
      "off-ai-desc": "Used only when you provide your own API key. Ideal for complex paragraphs and nuanced style; completely optional.",

      // Privacy
      "priv-title": "Know exactly where your text goes",
      "priv-lead": "Swipelate has no proprietary server backend, requires no login, and sends no personal usage telemetry. The entire codebase is open source on GitHub for anyone to audit.",
      "priv-f1": "<strong>Online translation:</strong> selected text is sent directly to your chosen provider, default Google Translate.",
      "priv-f2": "<strong>Offline translation:</strong> everything is processed locally on your CPU.",
      "priv-f3": "<strong>History & API keys:</strong> stored only in the local app data folder on your machine. History can be disabled in Settings.",
      "priv-f4": "<strong>Sensitive applications:</strong> add password managers or banking apps to the blacklist so the translation popup never triggers there.",

      // Download section
      "dl-title": "Download Swipelate",
      "dl-lead": "Latest release, free under the <a href=\"license.html\">MIT license</a>.",
      "dl-win-p": "Windows 10 or 11, 64-bit. Full feature set: selection translation, hotkeys, screen OCR, hover-to-translate.",
      "dl-win-btn": "Download for Windows",
      "dl-mac-tag": "experimental",
      "dl-mac-p": "macOS 11 or later, Apple Silicon (M1+). Features selection translation, hotkeys, screen OCR, and real-time screen translation; hover-to-translate is not yet available on macOS.",
      "dl-mac-btn": "Download for macOS",
      "dl-fine": "Unsigned binaries may show an operating system warning on first launch. See the <a href=\"docs.html#cai-dat\">installation guide</a>, <a href=\"https://github.com/thydynh03/swipelate/releases/latest\">SHA-256 checksums, and previous releases</a>.",

      // FAQ
      "faq-title": "Frequently Asked Questions",
      "faq-q1": "Is Swipelate free?",
      "faq-a1": "Yes. Swipelate is free and open source under the MIT license. Both Google Translate and local offline models require no account or API keys.",
      "faq-q2": "Can I translate without an Internet connection?",
      "faq-a2": "Yes, after downloading the offline model in Settings → Translation Engine (~107 MB per direction). Whenever network drops, Swipelate seamlessly switches to local on-device translation and marks it in the popup.",
      "faq-q3": "Which applications does it support?",
      "faq-a3": "Every application where text can be selected or copied: browsers, Word, PDF viewers, Claude desktop, Slack, Zalo, VS Code. For images or scanned PDFs, use the screen OCR area scan mode.",
      "faq-q4": "What languages can it translate?",
      "faq-a4": "When online, Swipelate supports all languages offered by Google Translate with auto-detection. The default pair is English ↔ Vietnamese. Offline mode supports pairs paired with English, including Vietnamese, Japanese, Chinese, French, and German.",
      "faq-q5": "Where is my highlighted text sent?",
      "faq-a5": "When online, highlighted text is transmitted securely to your chosen translation service. In offline mode, nothing leaves your machine. Swipelate has no proprietary servers and collects no personal data.",
      "faq-q6": "Why do Windows or macOS warn during installation?",
      "faq-a6": "Because installer packages are not signed with expensive corporate developer certificates. The <a href=\"docs.html#cai-dat\">Documentation page</a> provides a step-by-step walkthrough and SHA-256 verification instructions.",

      // Footer
      "footer-copy": "© 2026 thydynh03. Released under the <a href=\"license.html\">MIT license</a>. Formerly known as Quick Translate.",
      "footer-doc": "Docs",
      "footer-license": "License",
      "footer-stats": "Stats",
      "footer-issues": "Issues",
      "footer-privacy": "Privacy",
      "footer-source": "Source",
      "footer-disclaimer": "Swipelate is not affiliated with Google, Cambridge University Press, Oxford University Press, DeepL, or Anthropic. All product names and trademarks belong to their respective owners.",

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
      "stats-dl-version-title": "Downloads by Version",
      "stats-view-table": "View as table",
      "stats-view-chart": "View as chart",
      "stats-app-today": "App users today",
      "stats-app-today-meta": "devices, UTC time",
      "stats-app-30": "App users 30d",
      "stats-app-30-meta": "unique devices",
      "stats-app-all": "All-time devices",
      "stats-app-all-meta": "since tracking started",
      "stats-web-30": "Web visitors 30d",
      "stats-web-30-meta": "unique browsers",
      "stats-chart-users-title": "Daily users, last 30 days",
      "stats-versions-title": "Active versions",
      "stats-systems-title": "Operating systems",
      "stats-footnote": "Downloads count Windows and macOS installers (checksum files excluded). Users are counted by randomized device or browser IDs once daily; server stores no IP addresses. <a href=\"docs.html#thong-ke\">Details</a>.",

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

      // 404 Page
      "404-title": "Page not found",
      "404-desc": "This page does not exist or has been moved.",
      "404-home": "Back to Home",
    },

    vi: {
      // Document meta
      "title-index": "Swipelate – Bôi đen để dịch ở mọi ứng dụng trên Windows và macOS",
      "desc-index": "Swipelate là phần mềm dịch miễn phí cho máy tính: bôi đen văn bản ở bất kỳ ứng dụng nào, bản dịch hiện ngay tại chỗ. Dịch Anh – Việt, dịch offline không cần mạng, quét chữ trên ảnh (OCR).",

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
      "hero-eyebrow": "Phần mềm dịch cho Windows và macOS",
      "hero-title": "Bôi đen là dịch.<br>Ở mọi ứng dụng.",
      "hero-desc": "Đang đọc email, PDF hay một đoạn chat tiếng Anh? Bôi đen đoạn cần hiểu, bản dịch hiện ngay bên dưới. Không phải chuyển cửa sổ, không phải sao chép sang trình duyệt.",
      "hero-win": "Tải cho Windows",
      "hero-mac": "Tải cho macOS",
      "hero-release": "Miễn phí, mã nguồn mở. Không cần tài khoản.",

      // Scene Tabs & Demo
      "tab-select": "Đoạn văn",
      "tab-word": "Tra một từ",
      "tab-ocr": "Quét chữ trên ảnh",
      "tab-offline": "Mất mạng",
      "demo-caption": "Bôi đen một đoạn văn bản, bấm biểu tượng vừa hiện ra, bản dịch nằm ngay bên dưới.",

      // Ways to translate
      "ways-title": "Ba cách gọi bản dịch",
      "ways-lead": "Chọn cách hợp với việc bạn đang làm. Cả ba đều giữ nguyên vùng bôi đen và trả lại clipboard như cũ.",
      "way1-key": "Kéo chuột",
      "way1-title": "Bôi đen rồi bấm biểu tượng",
      "way1-desc": "Kéo chuột, nhấp đúp vào một từ hoặc nhấp ba lần vào một đoạn. Một biểu tượng nhỏ hiện cạnh con trỏ; bấm vào đó để xem bản dịch. Không bấm thì nó tự biến mất.",
      "way2-title": "Phím tắt, không cần chuột",
      "way2-desc": "Bôi đen bằng bàn phím rồi nhấn phím tắt. Bạn đổi được tổ hợp phím trong Cài đặt nếu nó trùng với phần mềm khác.",
      "way3-title": "Quét một vùng màn hình",
      "way3-desc": "Dành cho chữ không bôi đen được: ảnh chụp, PDF scan, video đang dừng. Kéo một hình chữ nhật quanh chữ, ứng dụng nhận dạng rồi dịch.",

      // New features
      "fresh-eyebrow": "Mới trong các bản gần đây",
      "fresh-title": "Không chỉ là bôi đen để dịch",
      "fresh-lead": "Bốn việc mới cho những lúc chữ không đứng yên để bạn bôi đen: game, phim, video không có phụ đề, và cả những đoạn bạn vừa copy.",
      "feat1-title": "Dịch thời gian thực trên màn hình",
      "feat1-desc": "Chọn cả màn hình hoặc kéo một vùng. Chữ đổi thì bản dịch đổi theo, phủ ngay lên chỗ cũ. Hợp với game, phụ đề cứng trong video và phần mềm chỉ có tiếng Anh. Câu đã gặp được nhớ lại, lần sau hiện ngay.",
      "feat2-key": "Kéo thả file",
      "feat2-title": "Phụ đề dịch sẵn cho cả bộ phim",
      "feat2-desc": "Thả file <code>.srt</code> hoặc <code>.vtt</code> vào tab Phụ đề, nhận lại file tiếng Việt nằm cạnh file gốc. VLC hay MPV tự nhận, xem không có độ trễ nào. Với YouTube có <a href=\"docs.html#youtube\">extension riêng cho Chrome và Edge</a>.",
      "feat3-then": "rồi",
      "feat3-title": "Nghe tiếng rồi dịch",
      "feat3-desc": "Video không có phụ đề? Swipelate đọc phụ đề trực tiếp mà Windows tạo ra từ âm thanh của máy, rồi dịch từng câu. Có thể ẩn phụ đề tiếng Anh để chỉ còn bản dịch ở đáy màn hình. Hiện chỉ nghe được tiếng Anh.",
      "feat4-tag": "macOS, thử nghiệm",
      "feat4-title": "Bảng clipboard cho macOS",
      "feat4-desc": "Những đoạn bạn đã copy, mở giữa màn hình như Spotlight với nền kính theo chế độ sáng tối của máy. Chọn để dán lại, ghim đoạn hay dùng, hoặc dịch luôn đoạn đó. Những gì trình quản lý mật khẩu đánh dấu là bí mật thì không bị ghi lại.",
      "feat4-find": "Tìm trong clipboard",

      // Dictionary
      "dict-title": "Tra một từ thì ra cả mục từ điển",
      "dict-p1": "Nhấp đúp vào một từ tiếng Anh, popup cho bạn nghĩa tiếng Việt xếp theo từ loại, phiên âm, và định nghĩa bằng tiếng Anh. Muốn đọc kỹ hơn, có nút mở thẳng mục từ đó trên Cambridge hoặc Oxford Learner’s.",
      "dict-p2": "Trong popup, mỗi từ của câu gốc đều bấm được. Gặp một từ lạ giữa câu, bấm vào nó để tra riêng mà không cần bôi đen lại.",
      "dict-pos": "động từ",
      "dict-mean": "tham dự, có mặt, đi học",
      "dict-def-label": "định nghĩa",

      // Offline Chain
      "off-title": "Mất mạng vẫn dịch được",
      "off-lead": "Swipelate thử lần lượt ba nguồn dịch. Bạn không cần cấu hình gì để hai nguồn đầu hoạt động.",
      "off-g-desc": "Mặc định khi có mạng. Không cần tài khoản, không cần API key.",
      "off-local-title": "Model chạy trên máy",
      "off-local-desc": "Khi mất mạng hoặc Google không phản hồi, ứng dụng tự chuyển sang model Opus-MT đã tải sẵn. Văn bản không rời khỏi máy. Mỗi chiều dịch nặng khoảng 107 MB, tải một lần trong Cài đặt.",
      "off-ai-title": "Claude hoặc DeepL",
      "off-ai-desc": "Chỉ dùng khi bạn tự nhập API key. Hợp với câu dài cần văn phong tự nhiên hơn; hoàn toàn không bắt buộc.",

      // Privacy
      "priv-title": "Biết rõ văn bản của mình đi đâu",
      "priv-lead": "Ứng dụng không có máy chủ riêng, không yêu cầu đăng nhập và không gửi số liệu sử dụng về đâu cả. Mã nguồn công khai trên GitHub để bất kỳ ai cũng kiểm tra được.",
      "priv-f1": "<strong>Dịch online:</strong> đoạn bạn bôi đen được gửi tới dịch vụ dịch đang chọn, mặc định là Google.",
      "priv-f2": "<strong>Dịch offline:</strong> mọi thứ xử lý trên máy bạn.",
      "priv-f3": "<strong>Lịch sử và API key:</strong> lưu trong thư mục dữ liệu của ứng dụng trên máy. Tắt được lịch sử trong Cài đặt.",
      "priv-f4": "<strong>Ứng dụng nhạy cảm:</strong> thêm trình quản lý mật khẩu hay ngân hàng vào danh sách loại trừ để biểu tượng dịch không xuất hiện ở đó.",

      // Download section
      "dl-title": "Tải Swipelate",
      "dl-lead": "Bản mới nhất, miễn phí theo <a href=\"license.html\">giấy phép MIT</a>.",
      "dl-win-p": "Windows 10 hoặc 11, 64-bit. Đầy đủ tính năng: bôi đen, phím tắt, OCR, rê chuột để tra từ.",
      "dl-win-btn": "Tải cho Windows",
      "dl-mac-tag": "thử nghiệm",
      "dl-mac-p": "macOS 11 trở lên, máy chip Apple (M1 trở đi). Có bôi đen để dịch, phím tắt, quét chữ trên màn hình và dịch thời gian thực; rê chuột để dịch chưa có trên macOS.",
      "dl-mac-btn": "Tải cho macOS",
      "dl-fine": "Bộ cài chưa có chữ ký số trả phí nên hệ điều hành sẽ hỏi lại khi mở lần đầu. Xem <a href=\"docs.html#cai-dat\">hướng dẫn cài đặt</a>, <a href=\"https://github.com/thydynh03/swipelate/releases/latest\">mã SHA-256 và các bản trước</a>.",

      // FAQ
      "faq-title": "Câu hỏi thường gặp",
      "faq-q1": "Swipelate có miễn phí không?",
      "faq-a1": "Có. Ứng dụng miễn phí và mã nguồn mở theo giấy phép MIT. Dịch bằng Google và dịch offline đều không cần tài khoản hay API key.",
      "faq-q2": "Không có mạng thì có dịch được không?",
      "faq-a2": "Được, sau khi bạn tải model offline trong Cài đặt → Engine dịch (khoảng 107 MB cho mỗi chiều). Khi mất mạng, ứng dụng tự chuyển sang dịch trên máy và ghi chú điều đó trong popup.",
      "faq-q3": "Dùng được trong những ứng dụng nào?",
      "faq-a3": "Mọi ứng dụng cho phép bôi đen và sao chép văn bản: trình duyệt, Word, trình đọc PDF, Claude, Slack, Zalo, VS Code. Với ảnh hoặc PDF scan, dùng chế độ quét vùng màn hình.",
      "faq-q4": "Dịch được những ngôn ngữ nào?",
      "faq-a4": "Khi online, ứng dụng dịch được các ngôn ngữ Google Translate hỗ trợ và tự nhận ngôn ngữ nguồn. Mặc định là Anh ↔ Việt: gặp tiếng Việt thì dịch sang tiếng Anh, còn lại dịch sang tiếng Việt. Dịch offline hiện có các cặp đi qua tiếng Anh như Việt, Nhật, Trung, Pháp, Đức.",
      "faq-q5": "Văn bản tôi bôi đen được gửi đi đâu?",
      "faq-a5": "Khi dịch online, đoạn văn bản được gửi tới dịch vụ dịch bạn chọn. Khi dịch offline, nó không rời khỏi máy. Ứng dụng không có máy chủ riêng và không thu thập dữ liệu.",
      "faq-q6": "Vì sao Windows hoặc macOS cảnh báo khi cài?",
      "faq-a6": "Vì bộ cài chưa được ký bằng chứng chỉ trả phí. <a href=\"docs.html#cai-dat\">Trang tài liệu</a> hướng dẫn từng bước và cách đối chiếu mã SHA-256 của tệp tải về.",

      // Footer
      "footer-copy": "© 2026 thydynh03. Phát hành theo <a href=\"license.html\">giấy phép MIT</a>. Trước đây có tên Quick Translate.",
      "footer-doc": "Tài liệu",
      "footer-license": "Giấy phép",
      "footer-stats": "Thống kê",
      "footer-issues": "Báo lỗi",
      "footer-privacy": "Quyền riêng tư",
      "footer-source": "Mã nguồn",
      "footer-disclaimer": "Swipelate không liên kết với Google, Cambridge University Press, Oxford University Press, DeepL hay Anthropic. Tên các dịch vụ thuộc về chủ sở hữu tương ứng.",

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
      "stats-users-title": "Người dùng",
      "stats-users-source": "Nguồn: Máy chủ thống kê ẩn danh",
      "stats-dl-version-title": "Lượt tải theo phiên bản",
      "stats-view-table": "Xem dạng bảng",
      "stats-view-chart": "Xem dạng biểu đồ",
      "stats-app-today": "Dùng app hôm nay",
      "stats-app-today-meta": "số máy, theo giờ UTC",
      "stats-app-30": "Dùng app 30 ngày",
      "stats-app-30-meta": "số máy khác nhau",
      "stats-app-all": "Máy đã từng dùng",
      "stats-app-all-meta": "từ khi bắt đầu đếm",
      "stats-web-30": "Khách web 30 ngày",
      "stats-web-30-meta": "số trình duyệt khác nhau",
      "stats-chart-users-title": "Người dùng mỗi ngày, 30 ngày gần nhất",
      "stats-versions-title": "Phiên bản đang dùng",
      "stats-systems-title": "Hệ điều hành",
      "stats-footnote": "Lượt tải đếm số lần tải bộ cài Windows và macOS (không tính tệp mã kiểm tra). Người dùng đếm theo mã ngẫu nhiên của máy hoặc trình duyệt, mỗi ngày một lần; máy chủ không lưu IP. <a href=\"docs.html#thong-ke\">Chi tiết</a>.",

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

    // Translate page title and description if mapped for the current page
    const page = document.body?.dataset?.page || (location.pathname.endsWith("stats.html") ? "stats" : "index");
    if (page === "index") {
      if (dict["title-index"]) document.title = dict["title-index"];
      const metaDesc = document.querySelector('meta[name="description"]');
      if (metaDesc && dict["desc-index"]) metaDesc.setAttribute("content", dict["desc-index"]);
    }

    // Translate all elements with data-i18n
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.dataset.i18n;
      const val = dict[key];
      if (val !== undefined) {
        if (typeof val === "string" && val.includes("<") && val.includes(">")) {
          if (el.innerHTML !== val) el.innerHTML = val;
        } else {
          if (el.textContent !== val) el.textContent = val;
        }
      }
    });

    // Translate elements with explicit data-i18n-html
    document.querySelectorAll("[data-i18n-html]").forEach((el) => {
      const key = el.dataset.i18nHtml;
      const val = dict[key];
      if (val !== undefined && el.innerHTML !== val) {
        el.innerHTML = val;
      }
    });

    // Translate placeholder attributes
    document.querySelectorAll("[data-i18n-ph]").forEach((el) => {
      const key = el.dataset.i18nPh;
      const val = dict[key];
      if (val !== undefined && el.getAttribute("placeholder") !== val) {
        el.setAttribute("placeholder", val);
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
