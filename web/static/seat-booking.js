/**
 * Seat hold / checkout: Datastar expressions are not full JS; use native fetch here.
 * Reads movie slug from #cinemaMovieSlug (set by the film list Datastar handler).
 */
(function () {
  const main = document.getElementById("main-content");
  if (!main) return;

  function movieSlug() {
    const el = document.getElementById("cinemaMovieSlug");
    return el ? el.value.trim() : "";
  }

  function viewerId() {
    const el = document.getElementById("cinemaViewer");
    return el ? el.value.trim() : "";
  }

  main.addEventListener("click", async (evt) => {
    const feedbackEl = document.getElementById("bookingFeedback");
    const seatButton = evt.target.closest("[data-seat][data-state]");
    if (!seatButton) {
      if (evt.target.closest("#seatGrid") && feedbackEl) {
        feedbackEl.innerHTML =
          '<div role="status" class="alert alert-info alert-soft"><span>Click a seat square to hold or confirm.</span></div>';
      }
      return;
    }

    const slug = movieSlug();
    if (!slug) {
      if (feedbackEl) {
        feedbackEl.innerHTML =
          '<div role="status" class="alert alert-info alert-soft"><span>Pick a film from the list first.</span></div>';
      }
      return;
    }

    const seatUuid = seatButton.dataset.seat;
    const seatState = seatButton.dataset.state;
    if (!seatUuid || !seatState) {
      if (feedbackEl) {
        feedbackEl.innerHTML =
          '<div role="alert" class="alert alert-warning"><span>Could not read seat data. Try refreshing the page.</span></div>';
      }
      return;
    }

    const checkoutEl = document.getElementById("checkoutArea");
    const viewer = viewerId();

    if (seatState !== "available" && seatState !== "your_hold") {
      const msg =
        seatState === "confirmed"
          ? "This seat is already booked."
          : seatState === "other_hold"
            ? "This seat is held by someone else."
            : "This seat cannot be selected right now.";
      if (feedbackEl) {
        feedbackEl.innerHTML =
          '<div role="status" class="alert alert-info alert-soft"><span>' +
          msg +
          "</span></div>";
      }
      return;
    }

    const endpoint =
      seatState === "available" ? "/bookings/hold" : "/bookings/checkout";
    const actionLabel =
      seatState === "available" ? "Hold seat" : "Confirm booking";

    if (checkoutEl) {
      checkoutEl.innerHTML =
        '<div class="card card-border border-base-300 bg-base-100/80"><div class="card-body p-4"><p class="font-medium text-base-content">' +
        actionLabel +
        '</p><p class="text-sm text-base-content/65">Seat: ' +
        seatUuid +
        '</p><p class="text-xs text-base-content/55">Processing...</p></div></div>';
    }

    try {
      const response = await fetch(endpoint, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ movie_slug: slug, seat_uuid: seatUuid }),
      });

      if (response.status === 401) {
        window.location.href = "/login";
        return;
      }

      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || "Request failed");
      }

      const payload = await response.json();

      if (feedbackEl) {
        feedbackEl.innerHTML =
          seatState === "available"
            ? '<div role="status" class="alert alert-success alert-soft"><span>Seat held. Click the same seat again to confirm booking.</span></div>'
            : '<div role="status" class="alert alert-success alert-soft"><span>Booking confirmed.</span></div>';
      }

      if (checkoutEl) {
        const seatText = payload.seat_uuid || seatUuid;
        const movieText = payload.movie_slug || slug;
        checkoutEl.innerHTML =
          '<div class="card card-border border-base-300 bg-base-100/80"><div class="card-body p-4"><p class="font-medium text-base-content">' +
          (seatState === "available" ? "Seat held" : "Booking confirmed") +
          '</p><p class="text-sm text-base-content/65">Movie: ' +
          movieText +
          '</p><p class="text-sm text-base-content/65">Seat: ' +
          seatText +
          "</p></div></div>";
      }

      const snapUrl =
        "/seat-map-snapshot?movie_slug=" +
        encodeURIComponent(slug) +
        "&viewer=" +
        encodeURIComponent(viewer);
      const snap = await fetch(snapUrl, { credentials: "include" });
      if (snap.ok) {
        const html = await snap.text();
        const parser = new DOMParser();
        const doc = parser.parseFromString(html, "text/html");
        const next = doc.getElementById("seatGrid");
        const cur = document.getElementById("seatGrid");
        if (next && cur) {
          cur.replaceWith(next);
        }
      }
    } catch (error) {
      const msg =
        error && typeof error.message === "string"
          ? error.message
          : "Seat action failed";
      if (feedbackEl) {
        feedbackEl.innerHTML =
          '<div role="alert" class="alert alert-error"><span>' +
          msg +
          "</span></div>";
      }
      if (checkoutEl) {
        checkoutEl.innerHTML =
          '<div class="card card-border border-error/50 bg-base-100/80"><div class="card-body p-4"><p class="text-sm text-error">Could not complete seat action.</p></div></div>';
      }
    }
  });
})();
