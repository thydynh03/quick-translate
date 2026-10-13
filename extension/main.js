// Quick Translate for YouTube: shows the subtitles of the playing video in
// your language, fetched once for the whole video, so there is no delay.
//
// YouTube only hands out subtitles to its own player (the address carries a
// token), so this script watches the player's subtitle request and asks for
// the same file again, translated.
(() => {
  const TARGET = "vi";
  const MARK = "qt_translated";
  const nativeFetch = window.fetch;

  /** Subtitle lines of the current video: { start, end, text } in milliseconds. */
  let cues = [];
  let loadedFor = "";
  let box = null;
  let shown = "";

  const isSubtitles = (url) => String(url).includes("/api/timedtext") && !String(url).includes(MARK);

  async function load(address) {
    const url = new URL(address, location.origin);
    const from = url.searchParams.get("lang") || "";
    const key = `${url.searchParams.get("v")}|${from}|${url.searchParams.get("kind") || ""}`;
    if (key === loadedFor) return;
    loadedFor = key;
    cues = [];
    // Already in the reader's language: YouTube's own subtitles are enough.
    if (from.split("-")[0] === TARGET) return;
    url.searchParams.set("fmt", "json3");
    url.searchParams.set("tlang", TARGET);
    url.searchParams.set(MARK, "1");
    try {
      const data = await (await nativeFetch(url)).json();
      if (loadedFor !== key) return;
      cues = (data.events || [])
        .filter((event) => event.segs && event.dDurationMs)
        .map((event) => ({
          start: event.tStartMs,
          end: event.tStartMs + event.dDurationMs,
          text: event.segs.map((seg) => seg.utf8).join("").replace(/\s+/g, " ").trim(),
        }))
        .filter((cue) => cue.text);
      // Nothing yet (automatic subtitles still being made): let a later request try again.
      if (cues.length === 0) loadedFor = "";
    } catch {
      loadedFor = "";
    }
  }

  // The player asks through XMLHttpRequest today; fetch is watched as well.
  const open = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    if (isSubtitles(url)) load(url);
    return open.call(this, method, url, ...rest);
  };
  window.fetch = function (input, init) {
    const url = typeof input === "string" ? input : input && input.url;
    if (url && isSubtitles(url)) load(url);
    return nativeFetch.call(this, input, init);
  };

  function ensureBox(player) {
    if (box && box.isConnected) return box;
    box = document.createElement("div");
    box.id = "qt-subtitle";
    box.style.cssText =
      "position:absolute;left:50%;bottom:14%;transform:translateX(-50%);z-index:60;max-width:80%;" +
      "padding:.15em .5em;border-radius:.25em;background:rgba(8,8,8,.78);color:#ffe27a;" +
      "font:500 clamp(15px,2.4vw,34px)/1.35 'YouTube Noto',Roboto,Arial,sans-serif;text-align:center;" +
      "pointer-events:none;white-space:pre-wrap;display:none";
    player.appendChild(box);
    return box;
  }

  function tick() {
    const player = document.querySelector("#movie_player");
    const video = player && player.querySelector("video");
    if (!video) return;
    // During an advert the clock is the advert's, not the video's.
    const now = document.querySelector(".ad-showing") ? -1 : video.currentTime * 1000;
    // The last line that has started wins, as lines of automatic subtitles overlap.
    let text = "";
    for (const cue of cues) {
      if (cue.start > now) break;
      if (now < cue.end) text = cue.text;
    }
    if (text === shown && box && box.isConnected) return;
    shown = text;
    const target = ensureBox(player);
    target.textContent = text;
    target.style.display = text ? "block" : "none";
  }
  setInterval(tick, 100);

  // The player only asks for subtitles when they are switched on: switch them
  // on once per video, so the translation can be fetched.
  let asked = "";
  setInterval(() => {
    const id = new URLSearchParams(location.search).get("v");
    if (!id || id === asked || location.pathname !== "/watch") return;
    const button = document.querySelector(".ytp-subtitles-button");
    const video = document.querySelector("#movie_player video");
    if (!button || !video || video.readyState < 2 || document.querySelector(".ad-showing")) return;
    asked = id;
    if (button.getAttribute("aria-pressed") !== "true") {
      button.click();
    } else if (!loadedFor.startsWith(`${id}|`)) {
      // Already on before this script could watch: ask the player to load them again.
      button.click();
      setTimeout(() => button.click(), 400);
    }
  }, 1000);

  // Moving to another video inside the same page.
  document.addEventListener("yt-navigate-start", () => {
    cues = [];
    loadedFor = "";
  });
})();
