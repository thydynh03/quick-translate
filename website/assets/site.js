(() => {
  const REPO = "thydynh03/quick-translate";
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  document.documentElement.classList.add("js");

  // ------------------------------------------------------------ downloads

  const platform = navigator.userAgentData?.platform || navigator.platform || "";
  const touch = /Android|iPhone|iPad/i.test(navigator.userAgent) || (/Mac/i.test(platform) && navigator.maxTouchPoints > 1);
  const os = touch ? "" : /Win/i.test(platform) ? "windows" : /Mac/i.test(platform) ? "mac" : "";
  // The visitor's own system gets the filled button; the other one stays secondary.
  if (os === "mac") {
    document.querySelector('[data-hero="mac"]')?.classList.remove("ghost");
    document.querySelector('[data-hero="windows"]')?.classList.add("ghost");
  }

  const megabytes = (bytes) => (bytes / 1048576).toFixed(1).replace(".", ",") + " MB";

  fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
    .then((response) => (response.ok ? response.json() : null))
    .then((release) => {
      if (!release) return;
      const version = release.tag_name.replace(/^v/, "");
      const line = document.getElementById("release-line");
      if (line) line.textContent = `Phiên bản ${version}. Miễn phí, mã nguồn mở, không cần tài khoản.`;
      document.querySelectorAll("[data-asset]").forEach((link) => {
        const asset = release.assets.find((item) => item.name === link.dataset.asset);
        const size = link.querySelector(".size");
        if (asset && size) size.textContent = megabytes(asset.size);
        if (!asset) link.href = release.html_url;
        // The same file under a name that says which version it is.
        const named = release.assets.find((item) => item.name === link.dataset.asset.replace("QuickTranslate-", `QuickTranslate-${version}-`));
        if (named) link.href = named.browser_download_url;
        if (asset || named) {
          const label = document.createElement("span");
          label.className = "version";
          label.textContent = ` ${version} `;
          link.insertBefore(label, size);
        }
      });
    })
    .catch(() => {});

  // ------------------------------------------------------- scroll reveal

  const blocks = document.querySelectorAll("main section:not(.hero) .wrap");
  if (!reduced && "IntersectionObserver" in window) {
    const seen = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            entry.target.classList.add("in");
            seen.unobserve(entry.target);
          }
        }
      },
      { rootMargin: "0px 0px -12% 0px" },
    );
    blocks.forEach((block) => {
      block.classList.add("reveal");
      seen.observe(block);
    });
  }

  // ----------------------------------------------------------------- demo

  const demo = document.getElementById("demo");
  if (!demo) return;

  const NOTE = "Nguồn dịch chính không dịch được; đã dùng nguồn dự phòng.";
  const SCENES = {
    select: {
      title: "Thư khách hàng",
      net: "Đang có mạng",
      body: "<p>Hi Lan,</p><p><mark>The meeting has been moved to Thursday afternoon. Please confirm whether you can attend.</mark></p><p>Thanks,<br>David</p>",
      gesture: "drag",
      route: "EN → VI · GOOGLE",
      text: "Cuộc họp đã được dời sang chiều thứ Năm. Vui lòng xác nhận bạn có thể tham dự hay không.",
      steps: ["Kéo chuột qua đoạn cần hiểu.", "Một biểu tượng nhỏ hiện cạnh con trỏ. Bấm vào đó.", "Bản dịch hiện ngay dưới vùng bôi đen, vùng chọn vẫn giữ nguyên."],
    },
    word: {
      title: "Thư khách hàng",
      net: "Đang có mạng",
      body: "<p>Hi Lan,</p><p>The meeting has been moved to Thursday afternoon. Please confirm whether you can <mark>attend</mark>.</p><p>Thanks,<br>David</p>",
      gesture: "double",
      route: "EN → VI · GOOGLE",
      html: '<p class="pop-word">attend<span>/əˈtend/</span></p><p><span class="pop-pos">động từ</span> tham dự, có mặt, đi học</p><p class="pop-def">to be present at an event, or to go regularly to a place</p>',
      steps: ["Nhấp đúp vào một từ.", "Bấm biểu tượng vừa hiện ra.", "Một từ thì có thêm từ loại, phiên âm và định nghĩa tiếng Anh."],
    },
    ocr: {
      title: "IMG_2041.jpg",
      net: "Đang có mạng",
      body: '<p class="photo-note">Ảnh chụp, không bôi đen được chữ</p><div class="sign" data-target><b>RESTRICTED AREA</b><span>Authorized staff only</span></div>',
      gesture: "box",
      route: "EN → VI · GOOGLE · OCR",
      text: "KHU VỰC HẠN CHẾ. Chỉ nhân viên được phép vào.",
      steps: ["Nhấn Alt + W, màn hình tối lại.", "Kéo một hình chữ nhật quanh phần chữ.", "Ứng dụng nhận dạng chữ trong vùng đó rồi dịch."],
    },
    offline: {
      title: "Thông báo chuyến bay",
      net: "Không có mạng",
      netOff: true,
      body: "<p><mark>Your flight has been delayed by two hours. We apologize for the inconvenience.</mark></p><p>Gate B12 · Boarding 21:40</p>",
      gesture: "drag",
      route: "EN → VI · OFFLINE",
      text: "Chuyến bay của bạn đã bị hoãn hai giờ. Chúng tôi xin lỗi vì sự bất tiện này.",
      note: NOTE,
      steps: ["Máy đang không có mạng. Vẫn bôi đen như thường.", "Bấm biểu tượng.", "Google không phản hồi nên ứng dụng tự dịch bằng model trên máy."],
    },
  };
  const ORDER = Object.keys(SCENES);

  const $ = (selector) => demo.querySelector(selector);
  const stage = $(".stage");
  const body = $(".win-body");
  const cursor = $(".cursor");
  const trigger = $(".trigger");
  const pop = $(".pop");
  const box = $(".ocr-box");
  const caption = $(".demo-caption");
  const tabs = [...demo.querySelectorAll("[role=tab]")];

  const STOP = Symbol("stop");
  let run = 0;
  let onScreen = true;
  let current = ORDER[0];

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  /** Waits, then stops this timeline if another scene has started since. */
  async function wait(ms, token) {
    await sleep(reduced ? 0 : ms);
    while (!reduced && token === run && (!onScreen || document.hidden)) await sleep(300);
    if (token !== run) throw STOP;
  }

  /** A rectangle in stage coordinates. */
  function local(rect) {
    const origin = stage.getBoundingClientRect();
    return {
      left: rect.left - origin.left,
      top: rect.top - origin.top,
      right: rect.right - origin.left,
      bottom: rect.bottom - origin.top,
      width: rect.width,
      height: rect.height,
    };
  }

  function moveCursor(x, y, ms) {
    cursor.style.transitionDuration = `${reduced ? 0 : ms}ms, 250ms`;
    cursor.style.transform = `translate(${x}px, ${y}px)`;
  }

  function place(element, x, y) {
    element.style.left = `${Math.round(x)}px`;
    element.style.top = `${Math.round(y)}px`;
  }

  function reset() {
    for (const element of [cursor, trigger, pop, box]) element.classList.remove("on", "press");
    stage.classList.remove("capturing");
    cursor.style.transitionDuration = "0ms";
  }

  function fillPopup(scene) {
    $(".pop-route").textContent = scene.route;
    const target = $(".pop-body");
    if (scene.html) {
      target.innerHTML = scene.html;
    } else {
      target.innerHTML = "";
      const paragraph = document.createElement("p");
      for (const word of scene.text.split(" ")) {
        const span = document.createElement("span");
        span.className = "w";
        span.textContent = word + " ";
        paragraph.append(span);
      }
      target.append(paragraph);
      if (scene.note) {
        const note = document.createElement("p");
        note.className = "pop-note";
        note.textContent = scene.note;
        target.append(note);
      }
    }
  }

  async function showPopup(scene, anchor, token) {
    fillPopup(scene);
    const width = stage.clientWidth;
    const height = stage.clientHeight;
    const left = Math.max(12, Math.min(anchor.left, width - pop.offsetWidth - 12));
    const below = anchor.bottom + 12;
    const top = below + pop.offsetHeight <= height + 28 ? below : Math.max(8, anchor.top - pop.offsetHeight - 12);
    place(pop, left, top);
    pop.classList.add("on");
    for (const word of pop.querySelectorAll(".w")) {
      await wait(42, token);
      word.classList.add("on");
    }
  }

  async function play(id, token) {
    const scene = SCENES[id];
    reset();
    $(".win-title").textContent = scene.title;
    const net = $(".win-net");
    net.textContent = scene.net;
    net.classList.toggle("off", Boolean(scene.netOff));
    body.innerHTML = scene.body;
    caption.textContent = scene.steps[0];
    await wait(450, token);

    const target = body.querySelector("mark, [data-target]");
    let anchor = local(target.getBoundingClientRect());

    if (scene.gesture === "box") {
      stage.classList.add("capturing");
      place(box, anchor.left, anchor.top);
      box.style.width = `${anchor.width}px`;
      box.style.height = `${anchor.height}px`;
      moveCursor(anchor.left - 4, anchor.top - 4, 0);
      cursor.classList.add("on");
      await wait(700, token);
      caption.textContent = scene.steps[1];
      box.classList.add("on");
      moveCursor(anchor.right, anchor.bottom, 800);
      await wait(1150, token);
      stage.classList.remove("capturing");
      box.classList.remove("on");
      cursor.classList.remove("on");
    } else {
      const rects = [...target.getClientRects()].map(local);
      const first = rects[0];
      const last = rects[rects.length - 1];
      if (scene.gesture === "drag") {
        moveCursor(first.left, first.top + first.height / 2, 0);
        cursor.classList.add("on");
        await wait(500, token);
        target.style.transition = reduced ? "none" : "background-size 1s linear";
        target.classList.add("on");
        moveCursor(last.right, last.top + last.height / 2, 1000);
        await wait(1150, token);
      } else {
        moveCursor(first.left - 70, first.bottom + 50, 0);
        cursor.classList.add("on");
        await wait(200, token);
        moveCursor(first.left + first.width / 2, first.top + first.height / 2, 600);
        await wait(850, token);
        target.classList.add("on");
        await wait(350, token);
      }
      caption.textContent = scene.steps[1];
      const x = Math.min(last.right + 10, stage.clientWidth - 40);
      const y = last.bottom + 6;
      place(trigger, x, y);
      trigger.classList.add("on");
      await wait(600, token);
      moveCursor(x + 12, y + 12, 450);
      await wait(650, token);
      trigger.classList.add("press");
      await wait(160, token);
      trigger.classList.remove("on", "press");
      anchor = { ...anchor, left: first.left };
    }

    caption.textContent = scene.steps[2];
    await showPopup(scene, anchor, token);
    await wait(4200, token);
  }

  async function start(id, auto) {
    const token = ++run;
    current = id;
    for (const tab of tabs) {
      const selected = tab.dataset.scene === id;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
    }
    try {
      await play(id, token);
      if (!reduced) start(ORDER[(ORDER.indexOf(id) + 1) % ORDER.length], true);
    } catch (error) {
      if (error !== STOP) throw error;
    }
    return auto;
  }

  tabs.forEach((tab, index) => {
    tab.addEventListener("click", () => start(tab.dataset.scene));
    tab.addEventListener("keydown", (event) => {
      const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
      if (!step) return;
      const next = tabs[(index + step + tabs.length) % tabs.length];
      next.focus();
      start(next.dataset.scene);
    });
  });

  if ("IntersectionObserver" in window) {
    new IntersectionObserver(([entry]) => (onScreen = entry.isIntersecting), { threshold: 0.2 }).observe(stage);
  }

  let resizing;
  let lastWidth = innerWidth;
  addEventListener("resize", () => {
    if (innerWidth === lastWidth) return;
    lastWidth = innerWidth;
    clearTimeout(resizing);
    resizing = setTimeout(() => start(current), 250);
  });

  // Fonts change the line breaks, so the positions are measured after they load.
  (document.fonts?.ready ?? Promise.resolve()).then(() => start(current));
})();
