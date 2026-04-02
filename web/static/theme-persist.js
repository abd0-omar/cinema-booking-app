(function () {
  const STORAGE_KEY = "cinema-booking-theme";
  const ALLOWED = new Set(["lobby", "auditorium"]);
  const LEGACY = { "nord-light": "lobby", "nord-dark": "auditorium" };

  function normalizeTheme(raw) {
    if (!raw) return null;
    if (ALLOWED.has(raw)) return raw;
    return LEGACY[raw] ?? null;
  }

  function restore() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      const theme = normalizeTheme(saved);
      if (!theme) return;
      if (theme !== saved) {
        try {
          localStorage.setItem(STORAGE_KEY, theme);
        } catch (_) {
          /* ignore */
        }
      }
      const input = document.querySelector(
        'input.theme-controller[value="' + theme + '"]',
      );
      if (input) input.checked = true;
    } catch (_) {
      /* ignore */
    }
  }

  function wire() {
    document.querySelectorAll("input.theme-controller").forEach(function (el) {
      el.addEventListener("change", function () {
        if (!el.checked || !ALLOWED.has(el.value)) return;
        try {
          localStorage.setItem(STORAGE_KEY, el.value);
        } catch (_) {
          /* ignore */
        }
      });
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      restore();
      wire();
    });
  } else {
    restore();
    wire();
  }
})();
